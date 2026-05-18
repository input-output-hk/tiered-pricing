---
phase: 05-handoff
plan: 01
subsystem: documentation
tags: [register-edits, verdict-flips, disclosure-paragraph-promotion, handoff]

requires:
  - phase: 04-refresh-and-anchor
    provides: 6 LIVE + 18 DISCLOSED register state with draft-fallback / TBD-Phase-4 placeholders on the six remaining LIVE entries
provides:
  - 0 LIVE + 24 DISCLOSED final register state with load-bearing disclosure-paragraphs on every entry
  - Reading-guide preamble + footer narrative naming Plan 05-01 as the source of the six final flips and the phase-2-cip-evidence-v1 tag as the citable register state
affects: [05-02-handoff-verify-consistency, 05-03-cip-author-summary]

tech-stack:
  added: []
  patterns:
    - "Append-only RSK-NN identifiers (preserved per Phase 1 D-05)"
    - "Engineering-report voice for CIP-pasteable disclosure-paragraphs"

key-files:
  created: []
  modified:
    - "cip-evidence/audit-documents/realism-risks-register.md (6 verdict flips, 6 disclosure-paragraph rewrites, reading-guide preamble + footer)"

key-decisions:
  - "Per D-47 / D-48 / D-49: all six remaining LIVE entries flip to DISCLOSED; no LIVE survives into the CIP"
  - "RSK-hash-diversity-policy cites the Phase 2 D-19 strict-gate rule verbatim + the Phase 3 17/17 BACKED-eligible cells pass result + TEST-07a cross-cell SHA-256 framing as across-cell (does not violate within-cell gate)"
  - "Footer names phase-2-cip-evidence-v1 as the citable post-Phase-5 tag, anticipating the user-applied tag at Phase 5 close per don't-auto-commit memory"

patterns-established:
  - "Prose-promotion: removing (draft fallback;) prefix and integrating concrete Phase 3 / Phase 4 evidence into existing draft prose preserves the disclosure-paragraph schema while making the prose load-bearing"
  - "Within-paragraph abbreviation expansion (CIP, BCa, IQR, EIP-1559, SODA, SHA-256, RB) ensures each disclosure-paragraph stands alone when pasted into the CIP independent of the register header"

requirements-completed: [HAND-01]

duration: 25min
completed: 2026-05-18
---

# Phase 5 / Plan 01: Register Verdict-Flip Edits — Summary

**Six remaining LIVE register entries flipped to DISCLOSED with load-bearing disclosure-paragraphs; register reaches its final 24 DISCLOSED state for the CIP-author summary to paste from.**

## Performance

- **Tasks:** 2 of 2 completed
- **Files modified:** 1 (`cip-evidence/audit-documents/realism-risks-register.md`)
- **Commits:** 2 (Task 1 + Task 2)

## Accomplishments

### Task 1 — five non-policy LIVE entries → DISCLOSED

For each of `RSK-single-seed-precision`, `RSK-three-seed-statistical-power`, `RSK-unresolved-suite-claims`, `RSK-standard-user-fee-drift-exposure`, and `RSK-menu-collapse-to-advocacy`:

- Per-entry `**Verdict:**` line: LIVE → DISCLOSED.
- Per-entry `**EXP / Resolution:**` line appended with `— RESOLVED to DISCLOSED via Plan 05-01 prose-promotion …` annotation.
- Per-entry `**Disclosure-paragraph:**` field:
  - For the two entries with `*(draft fallback; …)*` prefixes (`RSK-single-seed-precision`, `RSK-standard-user-fee-drift-exposure`): prefix removed; prose light-touch refined to integrate Phase 3 / Phase 4 evidence (TEST-03 / TEST-04 N=20 BCa CI verdicts; the EIP-1559 `±1/D` per-block clamp on standard-quote drift).
  - For the three entries with `TBD — drafted in Phase 4` placeholders (`RSK-three-seed-statistical-power`, `RSK-unresolved-suite-claims`, `RSK-menu-collapse-to-advocacy`): placeholders replaced with load-bearing CIP-pasteable prose integrating the specific evidence named in D-49 (nine-cell Phase-3-promoted N=20 set; Plan 02-02 output-read resolution of the four UNRESOLVED suites; the four non-welfare property columns + Chung & Shi SODA 2023 impossibility result).
- Index-table Verdict cell for each row: LIVE → DISCLOSED.

### Task 2 — RSK-hash-diversity-policy + Index header / reading-guide / footer

- `RSK-hash-diversity-policy`:
  - Verdict: LIVE → DISCLOSED.
  - EXP / Resolution line rewritten to cite the strict gate as the resolution and the Phase 3 17/17 result as the load-bearing evidence.
  - Disclosure-paragraph: TBD placeholder replaced with load-bearing prose quoting the Phase 2 D-19 strict-gate rule, citing the Phase 3 hash-diversity-gate 17/17 BACKED-eligible cells pass result, and framing the TEST-07a cross-cell SHA-256 identity as across-cell (does not violate within-cell gate).
  - Index-table Verdict cell: LIVE → DISCLOSED.
- Reading-guide preamble (lines ~14): rewritten as a Plan-04-07-style continuation narrative reading "Post-Plan-05-01 verdict distribution: 0 LIVE + 24 DISCLOSED + 0 MITIGATED + 0 DORMANT" and tracing the full evolution from the v1 register's 12 LIVE + 12 DISCLOSED through Phase 4 Waves 2 + 3 to the Phase 5 Wave 1 close-state.
- Footer (last paragraph): rewritten as Phase-5-Wave-1 close-state narrative naming the six Plan-05-01 flips, the prior-phase flip history, and the `phase-2-cip-evidence-v1` tag as the citable register state. Supporting `.planning/` artefacts (spike READMEs, family-b memos, mechanism-welfare-impact memo) named explicitly as out of HAND-02 scope per D-50.

## Verification

Plan-level success criteria (all met):

```
$ grep -E '^\*\*Verdict:\*\*' cip-evidence/audit-documents/realism-risks-register.md | sort | uniq -c
     24 **Verdict:** DISCLOSED

$ awk '/^## Index/,/^## RSK-pool/' cip-evidence/audit-documents/realism-risks-register.md | grep -oE '\| (LIVE|DISCLOSED|MITIGATED|DORMANT) \|' | sort | uniq -c
     24 | DISCLOSED |

$ grep -c '0 LIVE + 24 DISCLOSED' cip-evidence/audit-documents/realism-risks-register.md
     2  (reading-guide preamble + footer prose)

$ grep -c 'TBD — drafted in Phase 4' cip-evidence/audit-documents/realism-risks-register.md
     0
$ grep -c '(draft fallback;' cip-evidence/audit-documents/realism-risks-register.md
     0
```

`RSK-hash-diversity-policy`'s disclosure-paragraph cites the Phase 2 D-19 strict-gate rule verbatim, the Phase 3 17/17 result, and the TEST-07a across-cell SHA-256 observation framing.

## Open questions for Plan 05-02 / 05-03

None blocking. The post-Plan-05-01 state is the input to Plan 05-02's HAND-02 consistency-verification script and Plan 05-03's HAND-01 cip-author-summary; both downstream plans operate on a known stable register.

## Notes

- Each disclosure-paragraph expands its abbreviations on first-use within the paragraph itself, per `CLAUDE.md` §"Conventions / gotchas", so each disclosure-paragraph stands alone when pasted into the CIP independent of the register header's consolidated abbreviation block.
- Tag application is user-executed per the don't-auto-commit memory; the footer narrative anticipates the tag rather than asserting it has been applied.
