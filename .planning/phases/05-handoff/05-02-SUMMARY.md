---
phase: 05-handoff
plan: 02
subsystem: tooling
tags: [handoff, consistency-verification, verification-script, audit-report, bash]

requires:
  - phase: 05-handoff
    provides: 0 LIVE + 24 DISCLOSED register state from Plan 05-01
provides:
  - "Reproducible four-check verification script (`verify-consistency.sh`) that future CIP reviewers can re-run against any later commit"
  - "Phase-5-close consistency audit log (`05-CONSISTENCY-REPORT.md`) with per-check PASS/FAIL verdicts"
  - "Two in-place defect fixes: upstream-Leios `ImpactAnalysis.md` citations reformatted from backtick-wrapped paths to italic proper-name references"
affects: [05-03-cip-author-summary]

tech-stack:
  added: []
  patterns:
    - "Pure-POSIX shell verification (bash + grep + awk + sed; no yq dependency despite plan's D-52 suggestion — yq not installed locally)"
    - "Multi-prefix fallback path resolver for backtick-wrapped citations (REPO_ROOT, sim-rs/, sim-rs/sim-core/src/, sim-rs/parameters/phase-2-sweep/)"
    - "Documentation-metasyntax filtering (exclude RSK-NN, RSK-slug, RSK-ids, CLM-NN, CLM-slug, EXP-NN, EXP-slug from dead-reference scans)"

key-files:
  created:
    - ".planning/phases/05-handoff/verify-consistency.sh (430 lines; executable; bash -n clean)"
    - ".planning/phases/05-handoff/05-CONSISTENCY-REPORT.md (141 lines; within 120-180 target)"
  modified:
    - "docs/phase-2/realism-risks-register.md (line 5: ImpactAnalysis.md citation reformatted)"
    - "docs/phase-2/coverage-check.md (line 5: ImpactAnalysis.md citation reformatted)"

key-decisions:
  - "yq is NOT installed in the dev environment despite the plan's D-52 reference; switched to pure-POSIX `grep -E '^\\s*-\\s*name:'` extraction against the suite YAMLs' jobs: block. This works because the phase-2 suite YAML schema is flat (no nested `name:` keys elsewhere)."
  - "Two in-place defect fixes were within Plan 05-02's deviation-rule scope: small, format-level edits (backtick → italic) with no semantic change to the cited Leios upstream precedent. The Plan 04-07 clean-baseline holds modulo this format polish."
  - "The 2 `phase-3-sign-flip-variance` hashes that are EXEMPT (non-pinned suite) remain BACKED in the coverage-check by virtue of the hash-diversity gate (20/20 distinct) and BCa-bootstrap confidence intervals, not by a pinned-goldens match — documented in the report's Check (iii) section."

patterns-established:
  - "Reproducible-by-future-reviewers contract: the script is the audit trail (idempotent, exits cleanly); the report is the snapshot. Both committed together so peer reviewers re-run from a known baseline."
  - "Two-pass audit: first run surfaces defects; defects fixed in place per Plan-04-07 deviation-rule; second run confirms clean. The report records both the defect cluster and the post-fix state."

requirements-completed: [HAND-02]

duration: 50min
completed: 2026-05-18
---

# Phase 5 / Plan 02: Reproducible Consistency Verification — Summary

**A reproducible four-check bash script lands at `.planning/phases/05-handoff/verify-consistency.sh` + a 141-line audit log at `.planning/phases/05-handoff/05-CONSISTENCY-REPORT.md`; OVERALL: PASS after one defect cluster fixed in place.**

## Performance

- **Tasks:** 2 of 2 completed
- **Files created:** 2 (verify-consistency.sh, 05-CONSISTENCY-REPORT.md)
- **Files modified:** 2 (register.md + coverage-check.md, both line 5)
- **Commits:** 1

## Accomplishments

### Task 1 — verify-consistency.sh

A pure-POSIX bash script (430 lines) implementing the four D-51 checks against the six in-scope CIP-cited documents (per D-50):

- **Check (i): RSK-NN / CLM-NN / EXP-NN dead-reference scan.** Extracts canonical sets from the definition sites (24 RSK-NN from the register's `^## RSK-…` headings; 55 CLM-NN from the coverage-check's `^| CLM-…` table-row anchors; 12 EXP-NN from the register's `EXP-…` tokens) and scans every in-scope document for tokens not in the canonical set. Documentation-metasyntax tokens (`RSK-NN`, `RSK-slug`, `RSK-ids`, `CLM-NN`, `CLM-slug`, `EXP-NN`, `EXP-slug`) are excluded.

- **Check (ii): backing-job path resolution.** Parses 25 `(backing-suite, backing-job)` pairs from `docs/phase-2/coverage-check.md` (table columns 6–7 of each `CLM-` row, skipping `—`), confirms each suite YAML exists, then scans its `jobs:` block (the `^  - name: <slug>` lines) and confirms each job-slug resolves. Uses pure-POSIX `grep -E '^\s*-\s*name:'` instead of `yq` because yq is not installed in the dev environment — the flat-shape phase-2 suite YAML schema makes the pure-bash approach safe.

- **Check (iii): golden-sha256 cross-check.** Parses 9 `(backing-suite, truncated-hash)` pairs (column 9 of each CLM row), extracts the leading 12 hex characters, and matches against the seven pinned `.goldens/<suite>.sha256` files. Falls back to scanning all pinned goldens for the legacy-phase-2-seed=1 annotation case where the CLM row's backing-suite is a `phase-3-*` suite but the hash comes from the original phase-2 pinned suite. Non-pinned-suite cells that have no pinned match are marked `EXEMPT (non-pinned suite)` (the two `phase-3-sign-flip-variance` Phase-3-only hashes).

- **Check (iv): markdown link + backtick-path resolution.** Extracts every `[text](path)` markdown link (with `#anchor` suffixes stripped, `http://`/`https://`/`mailto:` skipped) and every backtick-wrapped path with a recognised file extension that contains at least one directory separator (bare-filename citations, glob patterns, command lines, and YAML-excerpt tokens are filtered out). Resolves each path against four candidate prefixes plus the document's own directory.

The script:
- Sets `set -euo pipefail` + `IFS=$'\n\t'` for safety.
- Uses `mktemp -d` for temporary files cleaned via `trap`.
- Emits a fenced-markdown block per check + a final SUMMARY block to stdout.
- Caches per-check tables under `.planning/phases/05-handoff/.cache/` for the report-author to consume.
- Exits 0 on OVERALL: PASS, 1 on OVERALL: FAIL.

### Task 2 — 05-CONSISTENCY-REPORT.md

A 141-line markdown report (within CONTEXT.md's 120-180 target) with:

- Opening Status block naming the post-Plan-05-01 register distribution (0 LIVE + 24 DISCLOSED), the six in-scope documents (with cip-author-summary marked as pending Plan 05-03), the reviewer (verify-consistency.sh), and the reproducer command.
- Per-check sections (i)–(iv) with method narrative, structured tables (per-doc tally for Check i; 25-row pair table for Check ii; 9-row hash table for Check iii; per-doc tally for Check iv), and explicit PASS / FAIL verdict lines.
- A `Defects found and fixed in place` section enumerating the one defect cluster (the two upstream-Leios `ImpactAnalysis.md` backtick-wrapped citations) and the in-place fixes applied.
- An `Open for user review` section reading "None." (D-47 holds; no surprises).
- A closing footer naming the reproducer command and the SUMMARY filepath.

### Defects fixed in place

| # | Defect | Document | Fix |
|---|---|---|---|
| 1 | `docs/ImpactAnalysis.md` backtick-wrapped as if local path; the file is an upstream Leios precedent in `input-output-hk/ouroboros-leios`, not local | `docs/phase-2/realism-risks-register.md` line 5 | Replaced backtick path with italic proper-name reference: "*ImpactAnalysis.md* document in the input-output-hk/ouroboros-leios repository" |
| 2 | Same defect, parallel citation | `docs/phase-2/coverage-check.md` line 5 | Same fix: italic upstream-reference framing |

Both fixes are small, format-level edits with no semantic change to the cited precedent. After the fixes, `verify-consistency.sh` exits 0 with OVERALL: PASS.

## Verification

```
$ bash -n .planning/phases/05-handoff/verify-consistency.sh
$ test -x .planning/phases/05-handoff/verify-consistency.sh && echo "executable: yes"
executable: yes
$ wc -l .planning/phases/05-handoff/verify-consistency.sh
430
$ .planning/phases/05-handoff/verify-consistency.sh > /tmp/v.out 2>&1; echo "exit: $?"
exit: 0
$ tail -10 /tmp/v.out
=== SUMMARY ===
Check (i)   RSK/CLM/EXP dead refs:            PASS
Check (ii)  backing-job resolution:           PASS
Check (iii) golden-sha256 matches:            PASS
Check (iv)  markdown link resolution:         PASS

OVERALL: PASS
$ wc -l .planning/phases/05-handoff/05-CONSISTENCY-REPORT.md
141
```

All Plan 05-02 success criteria met: script + report on disk; script depends only on bash + grep + awk + sed (no new deps); report records the post-Plan-05-01 distribution; the defect-cluster was within Plan-05-02 edit authority and was fixed in place.

## Open questions for Plan 05-03

Plan 05-03 re-runs `verify-consistency.sh` after writing `docs/phase-2/cip-author-summary.md` to confirm the OVERALL: PASS verdict still holds with all six in-scope documents present. The current report records the 5-document baseline; Plan 05-03's re-run will append a post-summary numbers section. The current `Open for user review` section reads "None."; Plan 05-03 reviews whether any of the cip-author-summary's cross-references introduce new defects.

## Notes

- The plan's D-52 references `yq` as available in the repo but `yq` is not installed in the dev environment. The script switched to pure-POSIX `grep`/`awk`/`sed` for YAML parsing, which works for the flat-shape phase-2 suite YAMLs and removes the dependency. If a future suite schema introduces nested `name:` keys, the YAML parser needs revisiting.
- The `.planning/phases/05-handoff/.cache/` directory carries per-run TSV artefacts (per-doc table, pair table, hash table, link table). These are regenerated per run and are not intended to be committed; future cleanup may add a `.gitignore` entry.
