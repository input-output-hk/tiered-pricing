#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Run the phase-2 urgency-inversion paper-like suite.

Usage:
  scripts/run_phase2_urgency_inversion.sh run [--label <name>] [--output-root <dir>]
  scripts/run_phase2_urgency_inversion.sh resume <run-dir>

Commands:
  run                  Build the release binary and launch the urgency-inversion suite.
  resume <run-dir>     Resume an interrupted or failed suite run.

Options:
  --label <name>       Optional run label for `run`.
                       Default: phase2-urgency-inversion-paper-like
  --output-root <dir>  Parent directory for new suite run directories.
                       Default: output/experiment-suites
  -h, --help           Show this help.

Notes:
  - This wrapper runs a single `experiment-suite` config, not the Stage A shard batch.
  - Use `scripts/batch_progress.sh <run-dir> --watch` to monitor progress.
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIM_RS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${SIM_RS_DIR}"

DEFAULT_LABEL="phase2-urgency-inversion-paper-like"
DEFAULT_OUTPUT_ROOT="output/experiment-suites"
SUITE_CONFIG="parameters/phase-2-sweep/suites/phase-2-urgency-inversion-paper-like.yaml"
RELEASE_BIN="./target/release/experiment-suite"

ensure_release_binary() {
  cargo build --release -q -p sim-cli --bin experiment-suite
}

run_suite() {
  local label="${DEFAULT_LABEL}"
  local output_root="${DEFAULT_OUTPUT_ROOT}"

  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --label)
        label="${2:-}"
        shift 2
        ;;
      --output-root)
        output_root="${2:-}"
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown option for run: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
  done

  ensure_release_binary
  "${RELEASE_BIN}" run "${SUITE_CONFIG}" --label "${label}" --output-root "${output_root}"
}

resume_suite() {
  local run_dir="${1:-}"
  if [[ -z "${run_dir}" ]]; then
    echo "resume requires a <run-dir> argument" >&2
    usage >&2
    exit 1
  fi
  if [[ ! -d "${run_dir}" ]]; then
    echo "Run directory does not exist: ${run_dir}" >&2
    exit 1
  fi

  ensure_release_binary
  "${RELEASE_BIN}" resume "${run_dir}"
}

main() {
  local command="${1:-}"
  case "${command}" in
    run)
      shift
      run_suite "$@"
      ;;
    resume)
      shift
      resume_suite "$@"
      ;;
    -h|--help|"")
      usage
      ;;
    *)
      echo "Unknown command: ${command}" >&2
      usage >&2
      exit 1
      ;;
  esac
}

main "$@"
