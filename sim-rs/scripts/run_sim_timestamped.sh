#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Run sim-cli with a unique timestamped output directory to avoid overwriting results.

Usage:
  scripts/run_sim_timestamped.sh --experiment <path> [options] [-- <extra sim-cli args>]

Options:
  --experiment <path>          Required. Experiment parameter file.
  --compare-experiment <path>  Optional. Second experiment file for compare mode.
  --slots <n>                  Slots to simulate. Default: 2000
  --label <name>               Optional label appended to the timestamped run directory.
  --output-root <dir>          Output parent directory. Default: output/eb-compare
  --topology <path>            Topology YAML. Default: parameters/topology.default.yaml
  --base-parameters <path>     Base parameters file. Default: parameters/linear.yaml
  --timescale <n>              Optional timescale passed to sim-cli (-t).
  --trace                      Enable JSONL trace output (default is --no-trace).
  --no-trace                   Disable JSONL trace output (default).
  -h, --help                   Show this help.

Examples:
  scripts/run_sim_timestamped.sh \
    --experiment parameters/phase-2-sweep/experiments/paper-like-eip1559.yaml \
    --label eip1559-smoke

  scripts/run_sim_timestamped.sh \
    --experiment parameters/phase-2-sweep/experiments/paper-like-eip1559.yaml \
    --compare-experiment parameters/phase-2-sweep/experiments/paper-like-combined-winner-delay0-denom8.yaml \
    --label eip1559-vs-tiered
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIM_RS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${SIM_RS_DIR}"

EXPERIMENT=""
COMPARE_EXPERIMENT=""
SLOTS="2000"
LABEL=""
OUTPUT_ROOT="output/eb-compare"
TOPOLOGY="parameters/topology.default.yaml"
BASE_PARAMETERS="parameters/linear.yaml"
TIMESCALE=""
NO_TRACE=1
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --experiment)
      EXPERIMENT="${2:-}"
      shift 2
      ;;
    --compare-experiment)
      COMPARE_EXPERIMENT="${2:-}"
      shift 2
      ;;
    --slots)
      SLOTS="${2:-}"
      shift 2
      ;;
    --label)
      LABEL="${2:-}"
      shift 2
      ;;
    --output-root)
      OUTPUT_ROOT="${2:-}"
      shift 2
      ;;
    --topology)
      TOPOLOGY="${2:-}"
      shift 2
      ;;
    --base-parameters)
      BASE_PARAMETERS="${2:-}"
      shift 2
      ;;
    --timescale)
      TIMESCALE="${2:-}"
      shift 2
      ;;
    --trace)
      NO_TRACE=0
      shift
      ;;
    --no-trace)
      NO_TRACE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      EXTRA_ARGS=("$@")
      break
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "${EXPERIMENT}" ]]; then
  echo "Missing required --experiment argument." >&2
  usage >&2
  exit 1
fi

if [[ ! "${SLOTS}" =~ ^[0-9]+$ ]]; then
  echo "--slots must be a non-negative integer." >&2
  exit 1
fi

timestamp="$(date +"%Y%m%d-%H%M%S")"
sanitized_label="$(printf "%s" "${LABEL}" | tr '[:space:]' '-' | tr -cd '[:alnum:]_.-')"
run_name="${timestamp}"
if [[ -n "${sanitized_label}" ]]; then
  run_name="${run_name}-${sanitized_label}"
fi

run_dir="${OUTPUT_ROOT}/${run_name}"
mkdir -p "${run_dir}"

trace_path="${run_dir}/events.jsonl"
comparison_output="${run_dir}/report.txt"

cmd=(
  cargo run -q -p sim-cli --
  "${TOPOLOGY}"
  -p "${BASE_PARAMETERS}"
  -p "${EXPERIMENT}"
  "${trace_path}"
  -s "${SLOTS}"
)

if [[ -n "${COMPARE_EXPERIMENT}" ]]; then
  cmd+=(
    --compare-parameters "${COMPARE_EXPERIMENT}"
    --comparison-output "${comparison_output}"
  )
fi

if [[ -n "${TIMESCALE}" ]]; then
  cmd+=(-t "${TIMESCALE}")
fi

if [[ "${NO_TRACE}" -eq 1 ]]; then
  cmd+=(--no-trace)
fi

if [[ "${#EXTRA_ARGS[@]}" -gt 0 ]]; then
  cmd+=("${EXTRA_ARGS[@]}")
fi

echo "Run directory: ${run_dir}"
echo "Executing command:"
printf '  %q' "${cmd[@]}"
echo

"${cmd[@]}"

echo
echo "Artifacts:"
find "${run_dir}" -maxdepth 2 -type f \
  \( -name "metrics_comparison.txt" -o -name "time_series.csv" -o -name "tiered_plot.html" -o -name "diagnostics.log" -o -name "report.txt" -o -name "*.jsonl" \) \
  | sort
