#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Run the phase-2 EIP-like robustness sweep as fine-grained matrix shards.

Usage:
  scripts/run_phase2_eip1559_robustness_matrix_parallel.sh run [--jobs <n>] [--label <name>] [--output-root <dir>]
  scripts/run_phase2_eip1559_robustness_matrix_parallel.sh resume <batch-dir> [--jobs <n>]

Commands:
  run                    Build the release experiment-suite binary, generate one
                         shard per seed x demand x pricing case, and run up to
                         --jobs shards concurrently.
  resume <batch-dir>     Resume generated shard suites from an existing matrix
                         batch directory, again capped by --jobs.

Options:
  --jobs <n>             Maximum concurrent shard suites.
                         Default: 8.
  --label <name>         Optional batch label for `run`.
                         Default: phase2-eip1559-robustness-matrix-paper-like
  --output-root <dir>    Parent directory for new batch directories.
                         Default: output/experiment-batches
  -h, --help             Show this help.

Notes:
  - Matrix size is 5 seeds x 2 demand profiles x 9 EIP-like configs = 90 runs.
  - Outputs are compatible with scripts/batch_progress.sh.
  - A 9950X3D can run more than the old 5 seed shards, but memory is the
    practical limit under congested demand. Start with --jobs 8 or --jobs 10.
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIM_RS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${SIM_RS_DIR}"

DEFAULT_LABEL="phase2-eip1559-robustness-matrix-paper-like"
DEFAULT_OUTPUT_ROOT="output/experiment-batches"
DEFAULT_JOBS=8
RELEASE_BIN="./target/release/experiment-suite"

SEEDS=(42 43 44 45 46)
DEMAND_IDS=(moderate congested)
DEMAND_OVERLAYS=(
  parameters/phase-2-sweep/experiments/paper-like-moderate-demand.yaml
  parameters/phase-2-sweep/experiments/paper-like-congested-demand.yaml
)
PRICING_IDS=(
  denom4-target40
  denom4-target50
  denom4-target60
  denom8-target40
  denom8-target50
  denom8-target60
  denom16-target40
  denom16-target50
  denom16-target60
)
PRICING_OVERLAYS=(
  parameters/phase-2-sweep/experiments/eip1559-denom4-target40.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom4-target50.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom4-target60.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom8-target40.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom8-target50.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom8-target60.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom16-target40.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom16-target50.yaml
  parameters/phase-2-sweep/experiments/eip1559-denom16-target60.yaml
)

CHILD_PIDS=()
CHILD_NAMES=()
EXIT_CODE=0
INTERRUPT_COUNT=0

slugify() {
  printf "%s" "$1" | tr '[:space:]' '-' | tr -cd '[:alnum:]_.-'
}

ensure_release_binary() {
  cargo build --release -q -p sim-cli --bin experiment-suite
}

validate_jobs() {
  local jobs="$1"
  if ! [[ "${jobs}" =~ ^[0-9]+$ ]] || [[ "${jobs}" -lt 1 ]]; then
    echo "--jobs must be a positive integer" >&2
    exit 1
  fi
}

reset_children() {
  CHILD_PIDS=()
  CHILD_NAMES=()
  EXIT_CODE=0
  INTERRUPT_COUNT=0
}

handle_interrupt() {
  INTERRUPT_COUNT=$((INTERRUPT_COUNT + 1))
  if [[ "${INTERRUPT_COUNT}" -eq 1 ]]; then
    echo
    echo "Interrupting matrix shards gracefully..."
    local pid
    for pid in "${CHILD_PIDS[@]}"; do
      kill -INT "${pid}" 2>/dev/null || true
    done
  else
    echo
    echo "Force-terminating matrix shards..."
    local pid
    for pid in "${CHILD_PIDS[@]}"; do
      kill -TERM "${pid}" 2>/dev/null || true
    done
    exit 130
  fi
}

compact_children() {
  local new_pids=()
  local new_names=()
  local idx pid
  for idx in "${!CHILD_PIDS[@]}"; do
    pid="${CHILD_PIDS[${idx}]}"
    if kill -0 "${pid}" 2>/dev/null; then
      new_pids+=("${pid}")
      new_names+=("${CHILD_NAMES[${idx}]}")
    fi
  done
  CHILD_PIDS=("${new_pids[@]}")
  CHILD_NAMES=("${new_names[@]}")
}

wait_for_capacity() {
  local max_jobs="$1"
  local status
  while [[ "${#CHILD_PIDS[@]}" -ge "${max_jobs}" ]]; do
    if wait -n; then
      :
    else
      status=$?
      if [[ "${EXIT_CODE}" -eq 0 ]]; then
        EXIT_CODE="${status}"
      fi
    fi
    compact_children
  done
}

wait_for_all() {
  local status
  while [[ "${#CHILD_PIDS[@]}" -gt 0 ]]; do
    if wait -n; then
      :
    else
      status=$?
      if [[ "${EXIT_CODE}" -eq 0 ]]; then
        EXIT_CODE="${status}"
      fi
    fi
    compact_children
  done
  return "${EXIT_CODE}"
}

launch_child() {
  local max_jobs="$1"
  local shard_name="$2"
  local command_log="$3"
  shift 3

  wait_for_capacity "${max_jobs}"
  "$@" >"${command_log}" 2>&1 &
  CHILD_PIDS+=("$!")
  CHILD_NAMES+=("${shard_name}")
}

write_suite_config() {
  local config_path="$1"
  local job_id="$2"
  local label="$3"
  local demand_overlay="$4"
  local pricing_overlay="$5"
  local seed="$6"

  cat >"${config_path}" <<YAML
# yaml-language-server: \$schema=../../../../parameters/suites/suite.schema.json

defaults:
  topology: parameters/topology.default.yaml
  parameters:
    - parameters/linear.yaml
    - parameters/phase-2-sweep/protocol-base.yaml
  slots: 2000
  trace: false

jobs:
  - id: ${job_id}
    label: ${label}
    parameters:
      - ${demand_overlay}
      - ${pricing_overlay}
    seeds: [${seed}]
YAML
}

print_batch_summary() {
  local batch_dir="$1"
  echo
  echo "Batch directory: ${batch_dir}"
  echo "Watch progress:"
  echo "  scripts/batch_progress.sh ${batch_dir} --watch --interval 30"
}

run_batch() {
  local label="${DEFAULT_LABEL}"
  local output_root="${DEFAULT_OUTPUT_ROOT}"
  local jobs="${DEFAULT_JOBS}"

  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --jobs)
        jobs="${2:-}"
        shift 2
        ;;
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
  validate_jobs "${jobs}"

  local sanitized_label timestamp batch_dir config_dir
  sanitized_label="$(slugify "${label}")"
  timestamp="$(date +"%Y%m%d-%H%M%S")"
  batch_dir="${output_root}/${timestamp}-${sanitized_label}"
  config_dir="${batch_dir}/configs"
  mkdir -p "${batch_dir}/logs" "${batch_dir}/shards" "${config_dir}"

  ensure_release_binary
  reset_children
  trap handle_interrupt INT TERM

  local seed demand_idx pricing_idx demand_id pricing_id demand_overlay pricing_overlay
  local shard_name shard_output_root shard_log shard_label config_path job_id job_label
  for seed in "${SEEDS[@]}"; do
    for demand_idx in "${!DEMAND_IDS[@]}"; do
      demand_id="${DEMAND_IDS[${demand_idx}]}"
      demand_overlay="${DEMAND_OVERLAYS[${demand_idx}]}"
      for pricing_idx in "${!PRICING_IDS[@]}"; do
        pricing_id="${PRICING_IDS[${pricing_idx}]}"
        pricing_overlay="${PRICING_OVERLAYS[${pricing_idx}]}"
        shard_name="seed-${seed}-${demand_id}-${pricing_id}"
        shard_output_root="${batch_dir}/shards/${shard_name}"
        shard_log="${batch_dir}/logs/${shard_name}.log"
        shard_label="${sanitized_label}-${shard_name}"
        config_path="${config_dir}/${shard_name}.yaml"
        job_id="eip1559-${demand_id}-${pricing_id}-seed-${seed}"
        job_label="EIP-1559 robustness ${demand_id} ${pricing_id} seed ${seed}"
        mkdir -p "${shard_output_root}"
        write_suite_config "${config_path}" "${job_id}" "${job_label}" "${demand_overlay}" "${pricing_overlay}" "${seed}"
        printf "Launching shard: %s\n" "${shard_name}"
        launch_child \
          "${jobs}" \
          "${shard_name}" \
          "${shard_log}" \
          "${RELEASE_BIN}" run "${config_path}" --label "${shard_label}" --output-root "${shard_output_root}"
      done
    done
  done

  wait_for_all
  trap - INT TERM
  print_batch_summary "${batch_dir}"
  return "${EXIT_CODE}"
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

resume_batch() {
  local batch_dir="${1:-}"
  local jobs="${DEFAULT_JOBS}"
  shift || true

  while [[ "$#" -gt 0 ]]; do
    case "$1" in
      --jobs)
        jobs="${2:-}"
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown option for resume: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
  done
  validate_jobs "${jobs}"

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
  reset_children
  trap handle_interrupt INT TERM

  local shard_root shard_name run_dir shard_log
  for shard_root in "${batch_dir}"/shards/*; do
    [[ -d "${shard_root}" ]] || continue
    shard_name="$(basename "${shard_root}")"
    if ! run_dir="$(find_single_run_dir "${shard_root}" 2>/dev/null)"; then
      continue
    fi
    shard_log="${batch_dir}/logs/${shard_name}.resume.log"
    printf "Resuming shard: %s\n" "${shard_name}"
    launch_child \
      "${jobs}" \
      "${shard_name}" \
      "${shard_log}" \
      "${RELEASE_BIN}" resume "${run_dir}"
  done

  wait_for_all
  trap - INT TERM
  print_batch_summary "${batch_dir}"
  return "${EXIT_CODE}"
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
