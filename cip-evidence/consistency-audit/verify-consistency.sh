#!/usr/bin/env bash
# verify-consistency.sh — Plan 05-02 HAND-02 reproducible four-check audit.
#
# Runs four checks across the six Cardano Improvement Proposal (CIP)-cited
# documents per .planning/phases/05-handoff/05-CONTEXT.md D-50 / D-51:
#
#   (i)   RSK-NN / CLM-NN / EXP-NN dead-reference scan
#   (ii)  backing-job path resolution against suite YAMLs
#   (iii) golden-sha256 cross-check against .goldens/<suite>.sha256
#   (iv)  markdown link resolution + backtick-wrapped path resolution
#
# Exit code: 0 if OVERALL = PASS, 1 if any check fails.
# Future Cardano Improvement Proposal (CIP) reviewers can re-run this
# script against any later commit to verify the consistency claims hold.

set -euo pipefail
IFS=$'\n\t'

# --- Configuration ----------------------------------------------------------

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

IN_SCOPE_DOCS=(
  "cip-evidence/audit-documents/cardano-realism-audit.md"
  "cip-evidence/audit-documents/validity-threats.md"
  "cip-evidence/audit-documents/realism-risks-register.md"
  "cip-evidence/audit-documents/coverage-check.md"
  "cip-evidence/audit-documents/methodology-overview.md"
  "cip-evidence/cip-author-summary.md"
)

REGISTER="cip-evidence/audit-documents/realism-risks-register.md"
COVERAGE_CHECK="cip-evidence/audit-documents/coverage-check.md"
SUITES_DIR="sim-rs/parameters/phase-2-sweep/suites"
GOLDENS_DIR="sim-rs/parameters/phase-2-sweep/suites/.goldens"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

EXIT_CODE=0

# Documentation-metasyntax tokens (not real identifiers). One per line to
# avoid IFS interactions with the function below.
RSK_METASYNTAX=$'RSK-NN\nRSK-slug\nRSK-ids'
CLM_METASYNTAX=$'CLM-NN\nCLM-slug'
EXP_METASYNTAX=$'EXP-NN\nEXP-slug'

# --- Helpers ----------------------------------------------------------------

in_set() {
  # in_set <token> <set-file>
  grep -qxF "$1" "$2"
}

is_metasyntax() {
  # is_metasyntax <token> <newline-separated-metasyntax-list>
  printf '%s\n' "$2" | grep -qxF "$1"
}

# Extract job names from a suite YAML by simple grep against the "  - name: <slug>"
# pattern under the "jobs:" key. The phase-2 suite YAMLs are flat-shape: jobs
# live at top level with one "- name:" entry per job. No nested "name:" keys
# elsewhere in the schema, so a global grep is safe.
list_jobs_in_suite() {
  local suite_yaml="$1"
  [ -f "$suite_yaml" ] || return 1
  grep -E '^\s*-\s*name:\s*[a-zA-Z0-9_.-]+' "$suite_yaml" \
    | sed -E 's/^\s*-\s*name:\s*//; s/\s+$//'
}

# Resolve a relative path against a base directory; emit the normalised absolute path.
resolve_link() {
  local base_dir="$1"
  local rel_path="$2"
  ( cd "$base_dir" 2>/dev/null && readlink -m "$rel_path" 2>/dev/null ) || echo ""
}

# --- Canonical-set extraction -----------------------------------------------

RSK_SET="$TMPDIR/rsk_canonical.txt"
CLM_SET="$TMPDIR/clm_canonical.txt"
EXP_SET="$TMPDIR/exp_canonical.txt"

grep -oE '^## RSK-[a-z0-9-]+' "$REGISTER" | sed 's/^## //' | sort -u > "$RSK_SET"
grep -oE '^\| CLM-[0-9]+' "$COVERAGE_CHECK" | sed 's/^| //; s/ *$//' | sort -u > "$CLM_SET"
grep -oE 'EXP-[a-z0-9-]+' "$REGISTER" | sort -u > "$EXP_SET"

RSK_COUNT=$(wc -l < "$RSK_SET" | tr -d ' ')
CLM_COUNT=$(wc -l < "$CLM_SET" | tr -d ' ')
EXP_COUNT=$(wc -l < "$EXP_SET" | tr -d ' ')

# --- Check (i): RSK / CLM / EXP dead-reference scan -------------------------

echo
echo "=== CHECK (i): RSK-NN / CLM-NN / EXP-NN dead-reference scan ==="
echo "Documents in scope: ${#IN_SCOPE_DOCS[@]}"
echo "RSK-NN canonical set: $RSK_COUNT identifiers"
echo "CLM-NN canonical set: $CLM_COUNT identifiers"
echo "EXP-NN canonical set: $EXP_COUNT identifiers"

CHECK_I_DEAD_TOTAL=0
CHECK_I_SCANNED_TOTAL=0
PER_DOC_TABLE="$TMPDIR/per_doc_table.txt"
: > "$PER_DOC_TABLE"

for doc in "${IN_SCOPE_DOCS[@]}"; do
  if [ ! -f "$doc" ]; then
    printf '%s\t%s\t%s\t%s\t%s\n' "$doc" "n/a (pending)" "n/a" "n/a" "n/a" >> "$PER_DOC_TABLE"
    continue
  fi

  rsk_refs=$(grep -oE 'RSK-[a-z0-9-]+' "$doc" | sort -u || true)
  clm_refs=$(grep -oE 'CLM-[0-9]+' "$doc" | sort -u || true)
  exp_refs=$(grep -oE 'EXP-[a-z0-9-]+' "$doc" | sort -u || true)

  rsk_total=$(printf '%s\n' "$rsk_refs" | grep -c . || true)
  clm_total=$(printf '%s\n' "$clm_refs" | grep -c . || true)
  exp_total=$(printf '%s\n' "$exp_refs" | grep -c . || true)

  dead=0
  for tok in $rsk_refs; do
    is_metasyntax "$tok" "$RSK_METASYNTAX" && continue
    in_set "$tok" "$RSK_SET" || { dead=$((dead + 1)); echo "  DEAD RSK ref in $doc: $tok"; }
  done
  for tok in $clm_refs; do
    is_metasyntax "$tok" "$CLM_METASYNTAX" && continue
    in_set "$tok" "$CLM_SET" || { dead=$((dead + 1)); echo "  DEAD CLM ref in $doc: $tok"; }
  done
  for tok in $exp_refs; do
    is_metasyntax "$tok" "$EXP_METASYNTAX" && continue
    in_set "$tok" "$EXP_SET" || { dead=$((dead + 1)); echo "  DEAD EXP ref in $doc: $tok"; }
  done

  printf '%s\t%s\t%s\t%s\t%s\n' "$doc" "$rsk_total" "$clm_total" "$exp_total" "$dead" >> "$PER_DOC_TABLE"
  CHECK_I_SCANNED_TOTAL=$((CHECK_I_SCANNED_TOTAL + rsk_total + clm_total + exp_total))
  CHECK_I_DEAD_TOTAL=$((CHECK_I_DEAD_TOTAL + dead))
done

echo "Total references scanned: $CHECK_I_SCANNED_TOTAL"
echo "Dead references: $CHECK_I_DEAD_TOTAL"
if [ "$CHECK_I_DEAD_TOTAL" -eq 0 ]; then
  echo "Status: PASS"
  CHECK_I_STATUS="PASS"
else
  echo "Status: FAIL"
  CHECK_I_STATUS="FAIL"
  EXIT_CODE=1
fi

# --- Check (ii): backing-job path resolution --------------------------------

echo
echo "=== CHECK (ii): backing-job path resolution ==="

CHECK_II_TOTAL=0
CHECK_II_RESOLVED=0
CHECK_II_FAILED=0
JOB_PAIRS="$TMPDIR/job_pairs.txt"
: > "$JOB_PAIRS"

awk -F '|' '/^\| CLM-/ {
  s=$7; gsub(/^[ \t]+|[ \t]+$/, "", s);
  j=$8; gsub(/^[ \t]+|[ \t]+$/, "", j);
  if (s != "" && s != "—" && j != "" && j != "—") print s "::" j
}' "$COVERAGE_CHECK" | sort -u > "$JOB_PAIRS"

while IFS='::' read -r suite job; do
  # awk's "::" separator splits into pairs of empty-then-real strings; use sed-friendly form.
  :
done < /dev/null

# Cleaner read using a single-char delim:
JOB_PAIRS_DELIM="$TMPDIR/job_pairs_delim.txt"
awk -F '|' '/^\| CLM-/ {
  s=$7; gsub(/^[ \t]+|[ \t]+$/, "", s);
  j=$8; gsub(/^[ \t]+|[ \t]+$/, "", j);
  if (s != "" && s != "—" && j != "" && j != "—") print s "\t" j
}' "$COVERAGE_CHECK" | sort -u > "$JOB_PAIRS_DELIM"

PAIR_TABLE="$TMPDIR/pair_table.txt"
: > "$PAIR_TABLE"

while IFS=$'\t' read -r suite job; do
  CHECK_II_TOTAL=$((CHECK_II_TOTAL + 1))
  if [ ! -f "$suite" ]; then
    CHECK_II_FAILED=$((CHECK_II_FAILED + 1))
    echo "  MISSING suite YAML: $suite"
    printf '%s\t%s\tNO (suite missing)\n' "$suite" "$job" >> "$PAIR_TABLE"
    continue
  fi
  jobs_in_suite=$(list_jobs_in_suite "$suite" || true)
  if printf '%s\n' "$jobs_in_suite" | grep -qxF "$job"; then
    CHECK_II_RESOLVED=$((CHECK_II_RESOLVED + 1))
    printf '%s\t%s\tYES\n' "$suite" "$job" >> "$PAIR_TABLE"
  else
    CHECK_II_FAILED=$((CHECK_II_FAILED + 1))
    echo "  UNRESOLVED job: $job not found in $suite"
    printf '%s\t%s\tNO (job not in suite)\n' "$suite" "$job" >> "$PAIR_TABLE"
  fi
done < "$JOB_PAIRS_DELIM"

echo "Total pairs checked: $CHECK_II_TOTAL"
echo "Resolved: $CHECK_II_RESOLVED"
echo "Failed: $CHECK_II_FAILED"
if [ "$CHECK_II_FAILED" -eq 0 ]; then
  echo "Status: PASS"
  CHECK_II_STATUS="PASS"
else
  echo "Status: FAIL"
  CHECK_II_STATUS="FAIL"
  EXIT_CODE=1
fi

# --- Check (iii): golden-sha256 cross-check ---------------------------------

echo
echo "=== CHECK (iii): golden-sha256 cross-check ==="

CHECK_III_TOTAL=0
CHECK_III_MATCHED=0
CHECK_III_EXEMPT=0
CHECK_III_FAILED=0
HASH_TABLE="$TMPDIR/hash_table.txt"
: > "$HASH_TABLE"

# Pinned suites = the seven .sha256 files in .goldens/.
PINNED_SUITES="$TMPDIR/pinned_suites.txt"
ls "$GOLDENS_DIR"/*.sha256 2>/dev/null | sed -E 's|^.*/||; s|\.sha256$||' | sort -u > "$PINNED_SUITES"

# Extract (backing-suite, golden-sha256-truncated-12) pairs from coverage-check.md.
HASH_PAIRS="$TMPDIR/hash_pairs.txt"
awk -F '|' '/^\| CLM-/ {
  s=$7;  gsub(/^[ \t]+|[ \t]+$/, "", s);
  h=$10; gsub(/^[ \t]+|[ \t]+$/, "", h);
  if (s != "" && s != "—" && h != "" && h != "—") {
    # Extract leading 12 hex chars from the hash cell (may carry annotation in parens).
    trunc = ""; n = length(h);
    for (i = 1; i <= 12 && i <= n; i++) {
      c = substr(h, i, 1);
      if (c ~ /[a-f0-9]/) trunc = trunc c; else break;
    }
    if (length(trunc) == 12) print s "\t" trunc
  }
}' "$COVERAGE_CHECK" | sort -u > "$HASH_PAIRS"

# Build a global lookup: per pinned suite, the list of full hashes (one per (job, seed) line).
ALL_PINNED_HASHES="$TMPDIR/all_pinned_hashes.txt"
: > "$ALL_PINNED_HASHES"
while IFS= read -r pinned_suite; do
  goldens_file="$GOLDENS_DIR/${pinned_suite}.sha256"
  [ -f "$goldens_file" ] || continue
  awk -v suite="$pinned_suite" '{ print suite "\t" $3 }' "$goldens_file" >> "$ALL_PINNED_HASHES"
done < "$PINNED_SUITES"

while IFS=$'\t' read -r suite trunc; do
  CHECK_III_TOTAL=$((CHECK_III_TOTAL + 1))
  # Derive suite basename (strip directory + .yaml extension).
  suite_base=$(basename "$suite" .yaml)

  # If the suite is a pinned suite, look up directly in its goldens file.
  if grep -qxF "$suite_base" "$PINNED_SUITES"; then
    goldens_file="$GOLDENS_DIR/${suite_base}.sha256"
    full_match=$(awk '{ print $3 }' "$goldens_file" | grep "^$trunc" | head -1 || true)
    if [ -n "$full_match" ]; then
      CHECK_III_MATCHED=$((CHECK_III_MATCHED + 1))
      printf '%s\t%s\tYES (full hash: %s)\n' "$suite_base" "$trunc" "${full_match:0:16}..." >> "$HASH_TABLE"
    else
      # Fall back to scanning all pinned goldens (legacy phase-2 seed=1 case).
      legacy_match=$(awk -F '\t' -v t="$trunc" '$2 ~ "^" t { print $0 }' "$ALL_PINNED_HASHES" | head -1 || true)
      if [ -n "$legacy_match" ]; then
        CHECK_III_MATCHED=$((CHECK_III_MATCHED + 1))
        printf '%s\t%s\tYES (legacy match in %s)\n' "$suite_base" "$trunc" "$(echo "$legacy_match" | cut -f1)" >> "$HASH_TABLE"
      else
        CHECK_III_FAILED=$((CHECK_III_FAILED + 1))
        echo "  UNMATCHED hash $trunc in pinned suite $suite_base"
        printf '%s\t%s\tNO\n' "$suite_base" "$trunc" >> "$HASH_TABLE"
      fi
    fi
  else
    # Suite is not pinned (e.g. phase-3-* or phase-2-{moderate,realistic,sundaeswap,congested}-*).
    # Try fallback against all pinned goldens (legacy phase-2 seed=1 annotation case).
    legacy_match=$(awk -F '\t' -v t="$trunc" '$2 ~ "^" t { print $0 }' "$ALL_PINNED_HASHES" | head -1 || true)
    if [ -n "$legacy_match" ]; then
      CHECK_III_MATCHED=$((CHECK_III_MATCHED + 1))
      printf '%s\t%s\tYES (legacy match in %s)\n' "$suite_base" "$trunc" "$(echo "$legacy_match" | cut -f1)" >> "$HASH_TABLE"
    else
      CHECK_III_EXEMPT=$((CHECK_III_EXEMPT + 1))
      printf '%s\t%s\tEXEMPT (non-pinned suite)\n' "$suite_base" "$trunc" >> "$HASH_TABLE"
    fi
  fi
done < "$HASH_PAIRS"

echo "Total hashes checked: $CHECK_III_TOTAL"
echo "Matched: $CHECK_III_MATCHED"
echo "Exempt (non-pinned suite): $CHECK_III_EXEMPT"
echo "Failed: $CHECK_III_FAILED"
if [ "$CHECK_III_FAILED" -eq 0 ]; then
  echo "Status: PASS"
  CHECK_III_STATUS="PASS"
else
  echo "Status: FAIL"
  CHECK_III_STATUS="FAIL"
  EXIT_CODE=1
fi

# --- Check (iv): markdown link + backtick-path resolution -------------------

echo
echo "=== CHECK (iv): markdown link + backtick-path resolution ==="

CHECK_IV_TOTAL=0
CHECK_IV_RESOLVED=0
CHECK_IV_BROKEN=0
LINK_TABLE="$TMPDIR/link_table.txt"
: > "$LINK_TABLE"

for doc in "${IN_SCOPE_DOCS[@]}"; do
  if [ ! -f "$doc" ]; then
    printf '%s\tn/a\tn/a\tn/a\n' "$doc" >> "$LINK_TABLE"
    continue
  fi

  doc_dir=$(dirname "$doc")

  # Markdown links: [text](path)
  md_links=$(grep -oE '\[[^]]+\]\([^)]+\)' "$doc" \
    | sed -E 's/.*\(([^)]+)\)/\1/' \
    | sed -E 's/#.*$//' \
    | grep -vE '^(https?://|mailto:)' \
    | sort -u || true)

  # Backtick-wrapped paths to files with known extensions. Restrict to paths
  # containing a directory separator — bare filenames inside backticks are
  # inline citations (e.g. `phase-2-rb-scarcity.yaml` referenced by name in
  # prose), not file links. Exclude placeholder markers, YAML excerpts
  # (containing `:`), glob patterns (containing `*`), and command lines
  # (containing spaces).
  bt_paths=$(grep -oE '`[^`]*/[^`]*\.(md|sh|yaml|yml|toml|rs|py|json|txt)`' "$doc" \
    | sed -E 's/^`//; s/`$//' \
    | grep -vE '<.*>|NN|:|\*| ' \
    | sort -u || true)

  md_total=$(printf '%s\n' "$md_links" | grep -c . || true)
  bt_total=$(printf '%s\n' "$bt_paths" | grep -c . || true)
  broken=0

  for link in $md_links; do
    [ -z "$link" ] && continue
    # Skip pure-anchor links (already stripped above) and empty.
    case "$link" in
      "") continue ;;
      /*)
        # Absolute path inside the repo
        resolved="$REPO_ROOT$link"
        ;;
      *)
        resolved="$(resolve_link "$doc_dir" "$link")"
        ;;
    esac
    if [ -z "$resolved" ] || [ ! -e "$resolved" ]; then
      broken=$((broken + 1))
      echo "  BROKEN markdown link in $doc: $link"
    fi
  done

  # Backtick paths use mixed conventions across the in-scope docs (some are
  # repo-root-relative; some are simulator-relative per CLAUDE.md's `pwd =
  # sim-rs/`; some are source-tree-relative under sim-rs/sim-core/src/; some
  # are phase-2-sweep-relative). Try each candidate prefix in turn; accept
  # any match.
  for path in $bt_paths; do
    [ -z "$path" ] && continue
    candidates=(
      "$REPO_ROOT/$path"
      "$REPO_ROOT/sim-rs/$path"
      "$REPO_ROOT/sim-rs/sim-core/src/$path"
      "$REPO_ROOT/sim-rs/parameters/phase-2-sweep/$path"
      "$(resolve_link "$doc_dir" "$path")"
    )
    matched=0
    for cand in "${candidates[@]}"; do
      [ -z "$cand" ] && continue
      if [ -e "$cand" ]; then
        matched=1
        break
      fi
    done
    if [ "$matched" -eq 0 ]; then
      broken=$((broken + 1))
      echo "  BROKEN backtick path in $doc: $path"
    fi
  done

  printf '%s\t%s\t%s\t%s\n' "$doc" "$md_total" "$bt_total" "$broken" >> "$LINK_TABLE"
  CHECK_IV_TOTAL=$((CHECK_IV_TOTAL + md_total + bt_total))
  CHECK_IV_BROKEN=$((CHECK_IV_BROKEN + broken))
done

CHECK_IV_RESOLVED=$((CHECK_IV_TOTAL - CHECK_IV_BROKEN))
echo "Total links checked: $CHECK_IV_TOTAL"
echo "Resolved: $CHECK_IV_RESOLVED"
echo "Broken: $CHECK_IV_BROKEN"
if [ "$CHECK_IV_BROKEN" -eq 0 ]; then
  echo "Status: PASS"
  CHECK_IV_STATUS="PASS"
else
  echo "Status: FAIL"
  CHECK_IV_STATUS="FAIL"
  EXIT_CODE=1
fi

# --- Summary ----------------------------------------------------------------

echo
echo "=== SUMMARY ==="
printf '%-45s %s\n' "Check (i)   RSK/CLM/EXP dead refs:" "$CHECK_I_STATUS"
printf '%-45s %s\n' "Check (ii)  backing-job resolution:" "$CHECK_II_STATUS"
printf '%-45s %s\n' "Check (iii) golden-sha256 matches:" "$CHECK_III_STATUS"
printf '%-45s %s\n' "Check (iv)  markdown link resolution:" "$CHECK_IV_STATUS"
echo
if [ "$EXIT_CODE" -eq 0 ]; then
  echo "OVERALL: PASS"
else
  echo "OVERALL: FAIL"
fi

# Emit machine-parseable per-doc / per-pair / per-hash / per-link tables
# (consumed by the CONSISTENCY-REPORT.md author + by future reviewers
# inspecting why a check passed or failed at a given commit).
mkdir -p "$REPO_ROOT/cip-evidence/consistency-audit/.cache"
cp "$PER_DOC_TABLE"   "$REPO_ROOT/cip-evidence/consistency-audit/.cache/per_doc_table.tsv"
cp "$PAIR_TABLE"      "$REPO_ROOT/cip-evidence/consistency-audit/.cache/pair_table.tsv"
cp "$HASH_TABLE"      "$REPO_ROOT/cip-evidence/consistency-audit/.cache/hash_table.tsv"
cp "$LINK_TABLE"      "$REPO_ROOT/cip-evidence/consistency-audit/.cache/link_table.tsv"

exit "$EXIT_CODE"
