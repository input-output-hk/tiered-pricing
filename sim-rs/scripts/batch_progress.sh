#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Show progress and estimated time remaining for a batch experiment run.

Usage:
  scripts/batch_progress.sh <batch-dir>
  scripts/batch_progress.sh <batch-dir> --watch [--interval <seconds>]

Options:
  --watch              Continuously refresh the progress display.
  --interval <seconds> Refresh interval for watch mode (default: 30).
  -h, --help           Show this help.

Examples:
  scripts/batch_progress.sh output/experiment-batches/20260401-163916-phase2-stage-a-v10
  scripts/batch_progress.sh output/experiment-batches/20260401-163916-phase2-stage-a-v10 --watch
  scripts/batch_progress.sh output/experiment-batches/20260401-163916-phase2-stage-a-v10 --watch --interval 10
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SIM_RS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${SIM_RS_DIR}"

BATCH_DIR=""
WATCH=0
INTERVAL=30

while [[ $# -gt 0 ]]; do
  case "$1" in
    --watch) WATCH=1; shift ;;
    --interval) INTERVAL="${2:-30}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *)
      if [[ -z "${BATCH_DIR}" ]]; then
        BATCH_DIR="$1"; shift
      else
        echo "Unknown argument: $1" >&2; usage >&2; exit 1
      fi
      ;;
  esac
done

if [[ -z "${BATCH_DIR}" ]]; then
  echo "Error: <batch-dir> is required." >&2
  usage >&2
  exit 1
fi

if [[ ! -d "${BATCH_DIR}" ]]; then
  echo "Error: batch directory does not exist: ${BATCH_DIR}" >&2
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "Error: jq is required but not found." >&2
  exit 1
fi

# The core jq program that processes all manifests and produces tab-separated output.
# Each manifest is read as a separate input via --slurp.
JQ_PROGRAM='
def fmt_dur:
  if . == null then "?"
  elif . < 60 then "<1m"
  elif . < 3600 then "\(. / 60 | floor)m"
  else "\(. / 3600 | floor)h\((. % 3600) / 60 | floor)m"
  end;

def strip_nanos:
  sub("\\.[0-9]+Z$"; "Z") // .;

def parse_ts:
  if . == null then null
  else (strip_nanos | fromdateiso8601)
  end;

def safe_div(a; b):
  if b == 0 then null else a / b end;

($now | strip_nanos | fromdateiso8601) as $now_epoch |

# Extract shard name from the label (strip common prefixes).
(.["label"] // "unknown") as $lbl |
($lbl | sub("^phase2-stage-a-[a-z0-9]+-"; "")) as $shard_name |

# Job counts.
(.jobs | length) as $total |
([.jobs[] | select(.status == "completed")] | length) as $completed |
([.jobs[] | select(.status == "failed")] | length) as $failed |
([.jobs[] | select(.status == "running")] | length) as $running |

# Completed job durations (seconds).
[.jobs[]
  | select(.status == "completed")
  | .attempts[-1]
  | select(.["started-at"] != null and .["finished-at"] != null)
  | ((.["finished-at"] | parse_ts) - (.["started-at"] | parse_ts))
] as $durations |

# Average completed duration.
(safe_div($durations | add; $durations | length)) as $avg_dur |

# Running job elapsed time.
([.jobs[]
  | select(.status == "running")
  | .attempts[-1]
  | select(.["started-at"] != null)
  | ($now_epoch - (.["started-at"] | parse_ts))
] | if length > 0 then .[0] else null end) as $running_elapsed |

# Shard ETA: remaining jobs * avg duration, minus elapsed on current job.
(if $completed == $total then 0
 elif $avg_dur == null then null
 else
   (($total - $completed) as $remaining |
    if $running > 0 and $running_elapsed != null then
      (($remaining - 1) * $avg_dur) + ([$avg_dur - $running_elapsed, 0] | max)
    else
      $remaining * $avg_dur
    end)
 end) as $shard_eta |

# Suite created-at for elapsed calculation.
((.["created-at"] | parse_ts) // null) as $created |
(if $created != null then $now_epoch - $created else null end) as $elapsed |

# Output as tab-separated line.
[
  $shard_name,
  "\($completed)/\($total)\(if $failed > 0 then ", \($failed)!" else "" end)",
  (if $avg_dur == null then "-" else ($avg_dur | floor | fmt_dur) end),
  (if $completed == $total then "done"
   elif $running_elapsed != null then
     if $avg_dur != null and $running_elapsed > ($avg_dur * 3) then "stalled?"
     else ($running_elapsed | floor | fmt_dur)
     end
   else "-"
   end),
  (if $completed == $total then "done"
   elif $shard_eta == null then "?"
   else "~\($shard_eta | floor | fmt_dur)"
   end),
  ($shard_eta // 0 | tostring),
  ($elapsed // 0 | tostring)
] | @tsv
'

render_progress() {
  local batch_dir="$1"
  local batch_label
  batch_label="$(basename "${batch_dir}")"

  # Find all manifest files — supports both batch layout (shards/*/manifest.json)
  # and direct suite layout (manifest.json at top level).
  local manifests=()
  while IFS= read -r mf; do
    manifests+=("${mf}")
  done < <(find "${batch_dir}" -name "manifest.json" 2>/dev/null | sort)

  if [[ ${#manifests[@]} -eq 0 ]]; then
    echo "Batch: ${batch_label}"
    echo "Waiting for shards to start..."
    return
  fi

  local now
  now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  # Process each manifest individually (jq doesn't support multiple file --arg well with slurp).
  local lines=()
  local max_eta=0
  local max_eta_shard=""
  local total_jobs=0
  local total_completed=0
  local batch_elapsed=0

  for mf in "${manifests[@]}"; do
    local line
    line="$(jq -r --arg now "${now}" "${JQ_PROGRAM}" "${mf}" 2>/dev/null)" || continue
    lines+=("${line}")

    # Parse the numeric ETA and elapsed from the last two fields.
    local shard_eta_s elapsed_s shard_name
    shard_eta_s="$(echo "${line}" | cut -f6)"
    elapsed_s="$(echo "${line}" | cut -f7)"
    shard_name="$(echo "${line}" | cut -f1)"

    # Track max ETA for batch-level estimate.
    if [[ -n "${shard_eta_s}" ]] && (( $(echo "${shard_eta_s} > ${max_eta}" | bc -l 2>/dev/null || echo 0) )); then
      max_eta="${shard_eta_s}"
      max_eta_shard="${shard_name}"
    fi

    # Accumulate totals from progress field.
    local progress
    progress="$(echo "${line}" | cut -f2)"
    local done_count total_count
    done_count="$(echo "${progress}" | grep -oP '^\d+' || echo 0)"
    total_count="$(echo "${progress}" | grep -oP '/\K\d+' || echo 0)"
    total_completed=$((total_completed + done_count))
    total_jobs=$((total_jobs + total_count))

    # Track max elapsed for batch elapsed display.
    if [[ -n "${elapsed_s}" ]] && (( $(echo "${elapsed_s} > ${batch_elapsed}" | bc -l 2>/dev/null || echo 0) )); then
      batch_elapsed="${elapsed_s}"
    fi
  done

  # Format batch elapsed.
  local batch_elapsed_fmt
  batch_elapsed_int="${batch_elapsed%.*}"
  if [[ "${batch_elapsed_int}" -lt 60 ]]; then
    batch_elapsed_fmt="<1m"
  elif [[ "${batch_elapsed_int}" -lt 3600 ]]; then
    batch_elapsed_fmt="$((batch_elapsed_int / 60))m"
  else
    batch_elapsed_fmt="$((batch_elapsed_int / 3600))h$((batch_elapsed_int % 3600 / 60))m"
  fi

  # Format batch ETA.
  local batch_eta_fmt
  max_eta_int="${max_eta%.*}"
  if [[ "${total_completed}" -eq "${total_jobs}" ]]; then
    batch_eta_fmt="done"
  elif [[ "${max_eta_int}" -eq 0 ]] && [[ "${total_completed}" -lt "${total_jobs}" ]]; then
    batch_eta_fmt="?"
  elif [[ "${max_eta_int}" -lt 60 ]]; then
    batch_eta_fmt="~<1m"
  elif [[ "${max_eta_int}" -lt 3600 ]]; then
    batch_eta_fmt="~$((max_eta_int / 60))m"
  else
    batch_eta_fmt="~$((max_eta_int / 3600))h$((max_eta_int % 3600 / 60))m"
  fi

  # Print header.
  local pct=0
  if [[ "${total_jobs}" -gt 0 ]]; then
    pct=$((total_completed * 100 / total_jobs))
  fi
  printf "Batch: %-45s Elapsed: %s\n" "${batch_label}" "${batch_elapsed_fmt}"
  echo ""
  printf "%-28s %-12s %-10s %-10s %s\n" "Shard" "Progress" "Avg/Job" "Running" "Shard ETA"
  printf "%-28s %-12s %-10s %-10s %s\n" "────────────────────────────" "────────────" "──────────" "──────────" "──────────"

  # Print each shard row.
  for line in "${lines[@]}"; do
    local shard prog avg running eta
    shard="$(echo "${line}" | cut -f1)"
    prog="$(echo "${line}" | cut -f2)"
    avg="$(echo "${line}" | cut -f3)"
    running="$(echo "${line}" | cut -f4)"
    eta="$(echo "${line}" | cut -f5)"
    printf "%-28s %-12s %-10s %-10s %s\n" "${shard}" "${prog}" "${avg}" "${running}" "${eta}"
  done

  # Print summary.
  echo ""
  if [[ "${total_completed}" -eq "${total_jobs}" ]]; then
    printf "Total: %d/%d jobs (100%%)   Batch complete in %s\n" "${total_completed}" "${total_jobs}" "${batch_elapsed_fmt}"
  else
    printf "Total: %d/%d jobs (%d%%)   Batch ETA: %s" "${total_completed}" "${total_jobs}" "${pct}" "${batch_eta_fmt}"
    if [[ -n "${max_eta_shard}" ]] && [[ "${batch_eta_fmt}" != "?" ]]; then
      printf " (bottleneck: %s)" "${max_eta_shard}"
    fi
    printf "\n"
  fi
}

if [[ "${WATCH}" -eq 1 ]]; then
  while true; do
    clear
    render_progress "${BATCH_DIR}"
    sleep "${INTERVAL}"
  done
else
  render_progress "${BATCH_DIR}"
fi
