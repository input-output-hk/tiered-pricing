#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest_source="config/sweeps/cert-void-confirm.json"
out_dir="sweep-results/cert-void-confirm"
pilot_report="sweep-results/cert-void-smoke/comparison.json"
pilot_report_sha256="254f59113bea4166087cdc11afa7f60a8c9f8ab949088f737af3e9c34726750e"
pilot_source_patch="sweep-results/cert-void-smoke/source.patch"
pilot_source_patch_sha256="132cdb7a7d7d6cd336b540b7283758064b4500fce5565dbce1e074d95f98bf10"
seed_start=420
seeds=10
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/run_cert_void_confirm.sh [--out DIR]

Held-out confirmation of the cert-void pilot: three arms (recommended
W20/W5, cert-gated sample-and-hold, third-of-an-EB no-cert contribution)
on fresh seeds 420-429, 2,000 slots, independent RNG streams, all five
headline loads. The two co-primary endpoints, their criteria, and the
evidence scope are pre-registered in the manifest's analysisPlan block;
scripts/compare_cert_void_confirm.py refuses to evaluate anything else.

The pilot report and source patch must match their predeclared hashes and
the simulator binary must be the pilot's. The output directory is reserved
before execution and never overwritten; the analysis plan and its evaluator
are hashed into DIR/analysis-plan.sha256 before any fresh result exists.

Takes roughly 15-25 minutes on the development machine; launch-day is the
long tail.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --out)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      [[ -n "$2" ]] || { echo "error: --out must not be empty" >&2; exit 2; }
      out_dir="$2"
      shift 2
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

cd "$project_dir"

if [[ ! -f "$pilot_report" ]]; then
  echo "error: pilot report not found: $pilot_report" >&2
  exit 2
fi
pilot_report_hash_line="$(sha256sum -- "$pilot_report")"
if [[ "${pilot_report_hash_line%% *}" != "$pilot_report_sha256" ]]; then
  echo "error: pilot report hash differs from the predeclared artifact" >&2
  exit 2
fi
if [[ ! -f "$pilot_source_patch" ]]; then
  echo "error: pilot source patch not found: $pilot_source_patch" >&2
  exit 2
fi
pilot_patch_hash_line="$(sha256sum -- "$pilot_source_patch")"
if [[ "${pilot_patch_hash_line%% *}" != "$pilot_source_patch_sha256" ]]; then
  echo "error: pilot source-patch hash differs from the predeclared artifact" >&2
  exit 2
fi

if [[ -e "$out_dir" ]]; then
  echo "error: refusing to overwrite existing output directory: $out_dir" >&2
  echo "choose another directory with --out DIR" >&2
  exit 2
fi

SECONDS=0
stack build
simulator="$(stack path --local-install-root)/bin/abstract-sim-hs-exe"
if [[ ! -x "$simulator" ]]; then
  echo "error: built simulator executable not found: $simulator" >&2
  exit 2
fi
simulator_hash_line="$(sha256sum -- "$simulator")"
simulator_sha256="${simulator_hash_line%% *}"

pilot_sha256="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["provenance"]["simulator_sha256"])' "$pilot_report")"
if [[ "$simulator_sha256" != "$pilot_sha256" ]]; then
  echo "error: simulator binary differs from the pilot" >&2
  echo "  pilot:        $pilot_sha256" >&2
  echo "  confirmation: $simulator_sha256" >&2
  echo "rerun the pilot with this binary before treating a new run as its confirmation" >&2
  exit 2
fi

mkdir -p "$(dirname -- "$out_dir")"
if ! mkdir -- "$out_dir"; then
  echo "error: could not reserve output directory: $out_dir" >&2
  exit 2
fi

# Freeze the predeclared plan and its evaluator before any fresh result exists.
manifest="$out_dir/manifest.json"
cp -- "$manifest_source" "$manifest"
sha256sum -- \
  "$manifest" \
  scripts/compare_cert_void_confirm.py \
  scripts/compare_cert_void_pilot.py \
  scripts/run_cert_void_confirm.sh \
  > "$out_dir/analysis-plan.sha256"

run_load() {
  local name="$1"
  local kind="$2"
  local value="$3"
  local args=(
    sweep "$manifest"
    --seed-start "$seed_start"
    --seeds "$seeds"
    --slots "$slots"
    --summary-only
    --out "$out_dir/$name"
  )
  case "$kind" in
    profile) args+=(--load-profile "$value") ;;
    preset) args+=(--load "$value") ;;
    *) echo "error: unknown load kind: $kind" >&2; exit 2 ;;
  esac
  echo "running $name..."
  "$simulator" "${args[@]}"
}

run_load low preset low
run_load mid-load profile config/loads/mid-load.json
run_load severe-congestion profile config/loads/severe-congestion.json
run_load eb-capacity-stress profile config/loads/eb-capacity-stress.json
run_load launch-day profile config/loads/launch-day.json

python3 scripts/compare_cert_void_confirm.py \
  --root "$out_dir" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"

echo "cert-void confirmation complete in ${SECONDS}s:"
echo "  report: $out_dir/comparison.md"
echo "  data:   $out_dir/comparison.json"
