#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Run the phase-2 urgency-inversion paper-like sweep in parallel, one seed per shard.

Usage:
  scripts/run_phase2_urgency_inversion_parallel.sh run [--label <name>] [--output-root <dir>]
  scripts/run_phase2_urgency_inversion_parallel.sh resume <batch-dir>

Commands:
  run                    Build the release binary, launch all urgency-inversion shards
                         in parallel, and wait for them to finish.
  resume <batch-dir>     Resume every shard suite in an existing batch directory.

Options:
  --label <name>         Optional batch label for `run`.
                         Default: phase2-urgency-inversion-paper-like
  --output-root <dir>    Parent directory for new batch directories.
                         Default: output/experiment-batches
  -h, --help             Show this help.

Notes:
  - Each seed gets its own suite output root under <batch-dir>/shards/<seed>/.
  - Press Ctrl+C once to forward a graceful interrupt to all shard suites.
  - Resume uses the existing per-shard manifest.json files created by experiment-suite.
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIM_RS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${SIM_RS_DIR}"

shopt -s nullglob

DEFAULT_LABEL="phase2-urgency-inversion-paper-like"
DEFAULT_OUTPUT_ROOT="output/experiment-batches"
SHARD_CONFIG_DIR="parameters/phase-2-sweep/shards"
RELEASE_BIN="./target/release/experiment-suite"
CHILD_PIDS=()
CHILD_NAMES=()
INTERRUPT_COUNT=0

slugify() {
  printf "%s" "$1" | tr '[:space:]' '-' | tr -cd '[:alnum:]_.-'
}

ensure_release_binary() {
  cargo build --release -q -p sim-cli --bin experiment-suite
}

shard_name_from_config() {
  local config="$1"
  local name
  name="$(basename "${config}")"
  name="${name%.yaml}"
  name="${name#phase-2-urgency-inversion-paper-like-}"
  printf "%s\n" "${name}"
}

find_single_run_dir() {
  local shard_root="$1"
  local dirs=("${shard_root}"/*)
  if [[ ! -d "${shard_root}" || "${#dirs[@]}" -eq 0 ]]; then
    return 1
  fi
  if [[ "${#dirs[@]}" -ne 1 || ! -d "${dirs[0]}" ]]; then
    echo "Expected exactly one suite run directory under ${shard_root}" >&2
    return 1
  fi
  printf "%s\n" "${dirs[0]}"
}

print_batch_summary() {
  local batch_dir="$1"
  local shard_root run_dir shard_name
  echo
  echo "Batch directory: ${batch_dir}"
  echo "Shard run directories:"
  for shard_root in "${batch_dir}"/shards/*; do
    [[ -d "${shard_root}" ]] || continue
    shard_name="$(basename "${shard_root}")"
    if run_dir="$(find_single_run_dir "${shard_root}" 2>/dev/null)"; then
      echo "  ${shard_name}: ${run_dir}"
    else
      echo "  ${shard_name}: <not created>"
    fi
  done
}

reset_children() {
  CHILD_PIDS=()
  CHILD_NAMES=()
  INTERRUPT_COUNT=0
}

handle_interrupt() {
  INTERRUPT_COUNT=$((INTERRUPT_COUNT + 1))
  if [[ "${INTERRUPT_COUNT}" -eq 1 ]]; then
    echo
    echo "Interrupting shard suites gracefully..."
    local pid
    for pid in "${CHILD_PIDS[@]}"; do
      kill -INT "${pid}" 2>/dev/null || true
    done
  else
    echo
    echo "Force-terminating shard suites..."
    local pid
    for pid in "${CHILD_PIDS[@]}"; do
      kill -TERM "${pid}" 2>/dev/null || true
    done
    exit 130
  fi
}

launch_child() {
  local shard_name="$1"
  local command_log="$2"
  shift 2

  "$@" >"${command_log}" 2>&1 &
  CHILD_PIDS+=("$!")
  CHILD_NAMES+=("${shard_name}")
}

wait_for_children() {
  local batch_dir="$1"
  local exit_code=0
  local idx shard_status

  trap handle_interrupt INT TERM
  for idx in "${!CHILD_PIDS[@]}"; do
    if wait "${CHILD_PIDS[${idx}]}"; then
      printf "Shard completed: %s\n" "${CHILD_NAMES[${idx}]}"
    else
      shard_status=$?
      printf "Shard failed: %s (exit %s)\n" "${CHILD_NAMES[${idx}]}" "${shard_status}" >&2
      if [[ "${exit_code}" -eq 0 ]]; then
        exit_code="${shard_status}"
      fi
    fi
  done
  trap - INT TERM

  print_batch_summary "${batch_dir}"
  return "${exit_code}"
}

run_batch() {
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

  local sanitized_label timestamp batch_dir
  sanitized_label="$(slugify "${label}")"
  timestamp="$(date +"%Y%m%d-%H%M%S")"
  batch_dir="${output_root}/${timestamp}-${sanitized_label}"
  mkdir -p "${batch_dir}/logs" "${batch_dir}/shards"

  ensure_release_binary

  local configs=("${SHARD_CONFIG_DIR}"/phase-2-urgency-inversion-paper-like-*.yaml)
  if [[ "${#configs[@]}" -eq 0 ]]; then
    echo "No shard configs found under ${SHARD_CONFIG_DIR}" >&2
    exit 1
  fi

  local config shard_name shard_output_root shard_log shard_label
  reset_children
  for config in "${configs[@]}"; do
    shard_name="$(shard_name_from_config "${config}")"
    shard_output_root="${batch_dir}/shards/${shard_name}"
    shard_log="${batch_dir}/logs/${shard_name}.log"
    shard_label="${sanitized_label}-${shard_name}"
    mkdir -p "${shard_output_root}"
    printf "Launching shard: %s\n" "${shard_name}"
    printf "  config: %s\n" "${config}"
    printf "  log:    %s\n" "${shard_log}"
    launch_child \
      "${shard_name}" \
      "${shard_log}" \
      "${RELEASE_BIN}" run "${config}" --label "${shard_label}" --output-root "${shard_output_root}"
  done

  wait_for_children "${batch_dir}"
}

resume_batch() {
  local batch_dir="${1:-}"
  if [[ -z "${batch_dir}" ]]; then
    echo "resume requires a <batch-dir> argument" >&2
    usage >&2
    exit 1
  fi
  if [[ ! -d "${batch_dir}" ]]; then
    echo "Batch directory does not exist: ${batch_dir}" >&2
    exit 1
  fi

  ensure_release_binary

  local shard_root shard_name run_dir shard_log
  reset_children
  for shard_root in "${batch_dir}"/shards/*; do
    [[ -d "${shard_root}" ]] || continue
    shard_name="$(basename "${shard_root}")"
    run_dir="$(find_single_run_dir "${shard_root}")"
    shard_log="${batch_dir}/logs/${shard_name}.resume.log"
    printf "Resuming shard: %s\n" "${shard_name}"
    printf "  run dir: %s\n" "${run_dir}"
    printf "  log:     %s\n" "${shard_log}"
    launch_child \
      "${shard_name}" \
      "${shard_log}" \
      "${RELEASE_BIN}" resume "${run_dir}"
  done

  if [[ "${#CHILD_PIDS[@]}" -eq 0 ]]; then
    echo "No shard run directories found under ${batch_dir}/shards" >&2
    exit 1
  fi

  wait_for_children "${batch_dir}"
}

main() {
  local command="${1:-}"
  case "${command}" in
    run)
      shift
      run_batch "$@"
      ;;
    resume)
      shift
      resume_batch "$@"
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
