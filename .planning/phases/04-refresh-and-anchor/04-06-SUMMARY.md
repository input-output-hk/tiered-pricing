---
phase: 04-refresh-and-anchor
plan: 06
subsystem: docs
tags: [docs, register-edit, verdict-flip, disclosure-paragraph, coverage-check-signal-source]
requires:
  - 04-01 (DOC-03 anchor-or-disclose literature search outcomes)
  - 04-03 (Phase 3 evidence summary — TEST-05 / TEST-06 disclose-only fallback, TEST-07a regime-dependence)
provides:
  - "5 RSK entries' verdicts refreshed per Plan 04-01 and Plan 04-03 outcomes"
  - "RSK-un-anchored-controller-knobs disclosure-paragraph carries the four per-sub-knob Draft register prose blocks from Plan 04-01 as load-bearing CIP-paste source"
  - "RSK-steady-state-run-length disclosure-paragraph drafted from forward-pointer placeholder to load-bearing engineering-report prose"
  - "RSK-multiplier-floor-4-suite-coverage disclosure-paragraph carries the TEST-07a regime-dependence finding as load-bearing CIP-paste source"
  - "coverage-check.md signal-source-anchoring cells reflect Plan 04-01's per-sub-knob anchor outcomes (window-length 32 ANCHORED; three sub-knobs DISCLOSED)"
affects:
  - docs/phase-2/realism-risks-register.md (5 entries touched: RSK-un-anchored-controller-knobs, RSK-pool-count, RSK-calibration-stale-stake-snapshot, RSK-steady-state-run-length, RSK-multiplier-floor-4-suite-coverage; plus reading guide + footer prose updated)
  - docs/phase-2/coverage-check.md (CLM-05 signal-source-anchoring parenthetical updated to cite Reijsbergen/Leonardos/Liu for window-length 32)
tech-stack:
  added: []
  patterns: [verdict-flip via Plan 04-01 umbrella + Plan 04-03 disclose-only fallback + Plan 04-03 TEST-07a reframe]
key-files:
  created:
    - .planning/phases/04-refresh-and-anchor/04-06-SUMMARY.md
  modified:
    - docs/phase-2/realism-risks-register.md
    - docs/phase-2/coverage-check.md
decisions:
  - "Umbrella verdict for RSK-un-anchored-controller-knobs lands DISCLOSED (one of four sub-knobs ANCHORED — window-length 32 via Reijsbergen et al. AFT 2021 + Leonardos et al. AFT 2021 + Liu et al. CCS 2022; three sub-knobs DISCLOSED conditional on the multiplier-floor 4 / multiplier-floor 16 / lane-signal-source choices). Per the umbrella-grouping rule, the umbrella entry's verdict is DISCLOSED rather than MITIGATED because MITIGATED requires every sub-knob to land ANCHORED."
  - "RSK-pool-count, RSK-calibration-stale-stake-snapshot, RSK-steady-state-run-length all flipped LIVE → DISCLOSED via the disclose-only fallback decision per CONTEXT.md `<deferred>`: TEST-05 and TEST-06 re-runs are user-managed and out of scope for Phase 4. Existing draft fallback paragraphs become load-bearing (italic markers dropped); RSK-steady-state-run-length's forward-pointer placeholder is replaced with newly drafted load-bearing engineering-report prose naming the partial TEST-06 coverage and the fall-back framing."
  - "RSK-multiplier-floor-4-suite-coverage flipped LIVE → DISCLOSED with the TEST-07a regime-dependence reframe (the expected-MITIGATED path failed because the findings are calibration-specific). New disclosure-paragraph names: (a) rb-scarcity inversion at floor = 16 (welfare collapses 93-98%); (b) urgency-inversion weak reversal at floor = 16 (~13% correctly-priced advantage); (c) cross-cell SHA-256 identity finding linking the floor = 16 partitioned-both-dynamic degeneracy to priority-only-static under congested demand, mechanistically related to the floor = 4 cross-arm duplicate-job artefact."
  - "Coverage-check.md CLM-05 single-lane EIP-1559 signal-source-anchoring parenthetical updated to cite the Reijsbergen / Leonardos / Liu motivating citations for window-length 32 (was cross-referencing the RSK entry). The enum stays `spec-default` (D=8, target=0.5 match Ethereum mainnet bit-exact); only the window-length 32 parenthetical inside that cell shifts to motivating-citation form. Other CLM rows whose load-bearing sub-knob is multiplier-floor or lane-signal-source keep `unanchored (RSK-un-anchored-controller-knobs)` since those sub-knobs are DISCLOSED."
metrics:
  duration: "approximately 35 minutes wall-clock (sequential 3-task execution; no test re-runs)"
  completed_date: "2026-05-18"
  tasks_completed: 3
  files_modified: 2
---

# Phase 4 Plan 06: Realism-Risks Register Refresh and Coverage-Check Signal-Source Update Summary

One-liner: Register-side companion to Plan 04-04's audit refresh — landed five RSK verdict flips (one umbrella DISCLOSED with per-sub-knob granularity; three TEST-05 / TEST-06 disclose-only fall-backs; one TEST-07a regime-dependence reframe) and pasted Plan 04-01's per-sub-knob Draft register prose blocks into RSK-un-anchored-controller-knobs as load-bearing CIP-paste source.

## Scope and intent

This plan applied the register-side companion to Plan 04-04's audit narration: each of the five `RSK-NN` entries that Phase 4 touches now carries a final-state verdict (DISCLOSED for all five), and the load-bearing disclosure-paragraphs are either pasted from Plan 04-01's per-sub-knob Draft register prose (RSK-un-anchored-controller-knobs sub-knobs (a)–(d)), unchanged from the existing draft fallback (RSK-pool-count and RSK-calibration-stale-stake-snapshot), newly drafted from a forward-pointer placeholder (RSK-steady-state-run-length), or newly drafted reflecting the TEST-07a regime-dependence finding (RSK-multiplier-floor-4-suite-coverage).

Per the project's `<context>` constraints (no worktree; atomic per-task commits enabled), three commits land per task plus this SUMMARY commit, all on `dynamic-experiment` branch.

## Register-entry verdict-flip table

| RSK | Before | After | Action |
|---|---|---|---|
| RSK-un-anchored-controller-knobs | LIVE | DISCLOSED | Umbrella verdict per Plan 04-01 (one sub-knob ANCHORED, three DISCLOSED); disclosure-paragraph rewritten with four per-sub-knob blocks pasted verbatim from Plan 04-01; EXP / Resolution updated with per-sub-knob RESOLVED status |
| RSK-pool-count | LIVE | DISCLOSED | Per CONTEXT.md `<deferred>` TEST-05 disclose-only fallback; existing draft fallback paragraph promoted to load-bearing (italic marker dropped); locked threshold preserved |
| RSK-calibration-stale-stake-snapshot | LIVE | DISCLOSED | Same TEST-05 disclose-only fallback via shared dependency with RSK-pool-count; existing draft fallback promoted to load-bearing |
| RSK-steady-state-run-length | LIVE | DISCLOSED | Per CONTEXT.md `<deferred>` TEST-06 disclose-only fallback; new load-bearing disclosure-paragraph drafted (replacing the "TBD — drafted in Phase 4 if test verdict lands as DISCLOSED" forward-pointer); names partial TEST-06 coverage at 1 of 4 menu arms, the steady-state criterion, and the re-run recipe |
| RSK-multiplier-floor-4-suite-coverage | LIVE | DISCLOSED | Per Plan 04-03 TEST-07a "LIVE → DISCLOSED with regime-dependence reframe"; new load-bearing disclosure-paragraph names (a) rb-scarcity inversion at floor = 16, (b) urgency-inversion weak reversal at floor = 16, (c) cross-cell SHA-256 identity finding |

## Coverage-check signal-source-anchoring cell changes

Per Plan 04-01's umbrella verdict (window-length 32 ANCHORED via the EIP-1559 academic-critique tradition; three other sub-knobs DISCLOSED), cell-level flips were limited because most CLM rows depend on multiplier-floor or lane-signal-source (which remain DISCLOSED). Only CLM-05 (single-lane EIP-1559 control) had a load-bearing window-length-32 cell where the existing parenthetical cross-referenced the RSK entry rather than a motivating citation.

| CLM | Cell before | Cell after | Notes |
|---|---|---|---|
| CLM-05 (single-lane-EIP-1559-control) | `spec-default (... window-length 32 is anchored separately under \`RSK-un-anchored-controller-knobs\` for the four un-anchored knobs)` | `spec-default (... window-length 32 anchored via the Ethereum Improvement Proposal 1559 (EIP-1559) academic-critique tradition per Reijsbergen et al. Advances in Financial Technologies (AFT) 2021 "Transaction Fees on a Honeymoon" §"Short-term oscillation", Leonardos et al. AFT 2021 bounded-oscillation result, and Liu et al. Conference on Computer and Communications Security (CCS) 2022 empirical counter-bound; see \`.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md\` §"Sub-knob 1")` | Enum stays `spec-default`; the parenthetical shifts from RSK cross-reference to motivating-citation form per Plan 04-01's per-sub-knob output |

The 14 other CLM rows reading `unanchored (RSK-un-anchored-controller-knobs)` were audited; each row's load-bearing sub-knob is one of {multiplier-floor 4, multiplier-floor 16, lane-signal-source choice}, all of which Plan 04-01 disposed as DISCLOSED. The cells correctly stay `unanchored (RSK-un-anchored-controller-knobs)` since the umbrella entry still exists and the load-bearing sub-knobs remain unanchored. No `related-RSK-ids` cells required updates (the cross-reference is to the umbrella entry which exists in both pre- and post-Plan-04-06 register states).

## Post-Phase-4 register verdict distribution

Counts after Plan 04-06's edits only (Plan 04-04's `RSK-substrate-scope` flip is tracked separately and may further shift the distribution):

| Verdict | Count |
|---|---|
| LIVE | 7 |
| DISCLOSED | 17 |
| MITIGATED | 0 |
| DORMANT | 0 |
| **Total** | **24** |

The v1 register (Phase 1 plan-02 SUMMARY) reported 12 LIVE + 12 DISCLOSED. Plan 04-06 flipped five entries from LIVE to DISCLOSED, yielding the post-plan distribution above. Plan 04-04 (running in parallel on a separate file, `docs/phase-2/cardano-realism-audit.md`) may flip `RSK-substrate-scope` from LIVE to DISCLOSED via the audit's "substrate-scope paragraph" fold; that flip lands in Plan 04-04's SUMMARY and updates the register-side `RSK-substrate-scope` Verdict line — but Plan 04-06's atomic scope is the five entries listed in this SUMMARY's verdict-flip table, not `RSK-substrate-scope`. Plan 04-07 (Wave 3 consistency review) re-verifies the final post-Phase-4 distribution after both Plan 04-04 and Plan 04-05 (validity-threats refresh) close.

## Reading-guide and footer updates

Two narrative-prose updates in `docs/phase-2/realism-risks-register.md`:

1. **Reading guide** (lines around 13): replaced "Phase 1 v1 register" framing with "register through Phase 4 / Wave 2 (Plan 04-06 edits)"; added the post-Phase-4 distribution narration (7 LIVE + 17 DISCLOSED + 0 MITIGATED + 0 DORMANT) plus the evolution narrative (v1's 12/12 snapshot → Plan 04-06's five flips → Plan 04-04's separate `RSK-substrate-scope` flip → Plan 04-07's consistency re-verification).
2. **Footer** (final line of file): replaced "Phase 1 register — finalised v1" framing with "Register through Phase 4 Wave 2 (Plan 04-06)" framing; explicit list of the five flipped entries; forward-pointer to Plan 04-07 for the post-Phase-4 final consistency check.

## Final consistency check

Per Task 3's §"Final register consistency check" audit:

1. **§"Index" table verdict cells match per-entry `**Verdict:**` fields for all 5 entries touched.** Verified: RSK-pool-count, RSK-un-anchored-controller-knobs, RSK-calibration-stale-stake-snapshot, RSK-multiplier-floor-4-suite-coverage, RSK-steady-state-run-length all show DISCLOSED in both Index table and per-entry block.
2. **No `(draft fallback; ...)` italic markers on rewritten DISCLOSED entries.** Verified: the 5 entries touched no longer carry the italic marker. Remaining markers (line 78 RSK-single-seed-precision; line 338 RSK-standard-user-fee-drift-exposure) are on entries Plan 04-06 did not touch — both are LIVE per scope, so the italic marker remains correct.
3. **No `TBD plan 02` markers anywhere.** Verified: zero matches across the file.
4. **All `.planning/...` cross-reference paths resolve on disk.** Verified: 21 unique paths checked, all resolved.
5. **All cross-reference identifiers (RSK-NN, EXP-NN) preserved per append-only convention.** Verified: no renumbering occurred; the four EXP slugs under RSK-un-anchored-controller-knobs (`EXP-window-length-anchor`, `EXP-multiplier-floor-4-anchor`, `EXP-multiplier-floor-16-anchor`, `EXP-lane-signal-source-anchor`) preserved with their per-sub-knob RESOLVED dispositions appended in parentheses.

## Deviations from Plan

None — plan executed exactly as written per Plan 04-06 PLAN.md. The automated verification commands in Tasks 2 and 3 used `awk` range expressions of the form `/^## RSK-X/,/^## RSK-/` which match only a single line when both patterns hit the same target heading; the equivalent single-pass-with-flag idiom (`/^## RSK-X/{flag=1; next} /^## RSK-/{flag=0} flag`) is used in this SUMMARY for verification confirmation. The underlying register edits themselves are unaffected by the awk subtlety.

## Anomalies for Plan 04-07's consistency review

1. **Plan 04-04 cardano-realism-audit.md substrate-scope flip not visible here.** Plan 04-06 deliberately does not touch `RSK-substrate-scope` — that LIVE → DISCLOSED flip is part of Plan 04-04's audit-side fold per the v1 register's `RSK-substrate-scope` entry. Plan 04-07's consistency review should verify Plan 04-04 also updates the register entry's Verdict line (and §"Index" row) when the audit-side fold lands; otherwise the entry will read LIVE in the register but be folded as DISCLOSED in the audit, which is a register-vs-audit drift.
2. **The TEST-07a cross-cell SHA-256 identity finding is now narrated in both the register (RSK-multiplier-floor-4-suite-coverage disclosure-paragraph) and the audit (per Plan 04-04 §"Pricing-controller calibration").** Plan 04-07 should confirm both narrations are consistent (same regime-dependence framing, same cross-cell finding referenced) and that the duplication is the audit-vs-register dual-purpose pattern from CONTEXT.md D-39, not accidental divergence.
3. **Coverage-check CLM-05 parenthetical now cites Plan 04-01 by path.** A reviewer following the path can verify the citation chain end-to-end (Plan 04-01 → Reijsbergen / Leonardos / Liu). Plan 04-07 should confirm Plan 04-04's audit-side narration uses the same citation forms (preferred form per Plan 04-01: in-document expansion of Reijsbergen et al. AFT 2021 + Leonardos et al. AFT 2021 + Liu et al. CCS 2022 with the §"Short-term oscillation" anchor).
4. **Reading-guide and footer narrative drift.** Both updated to reflect the post-Plan-04-06 distribution. If Plan 04-04 flips `RSK-substrate-scope` and the distribution moves to 6 LIVE + 18 DISCLOSED, Plan 04-07 should reconcile.
5. **The five disclosure-paragraphs touched are now load-bearing CIP-paste prose.** Any future RSK entry whose disclosure-paragraph cross-references one of the five (none currently do) would need to re-verify the cross-reference remains correct.

## Commits

| Task | Commit | Files |
|---|---|---|
| Task 1 | 9e7cfff | docs/phase-2/realism-risks-register.md (RSK-un-anchored-controller-knobs verdict + per-sub-knob disclosure-paragraph + EXP / Resolution + Index table); docs/phase-2/coverage-check.md (CLM-05 signal-source-anchoring parenthetical) |
| Task 2 | 6513913 | docs/phase-2/realism-risks-register.md (RSK-pool-count + RSK-calibration-stale-stake-snapshot + RSK-steady-state-run-length verdicts + scope-of-resolution + disclosure-paragraphs + Index table) |
| Task 3 | b2fe1cb | docs/phase-2/realism-risks-register.md (RSK-multiplier-floor-4-suite-coverage verdict + disclosure-paragraph + scope-of-resolution + EXP / Resolution + Index table; reading guide and footer prose) |
| SUMMARY | (this commit) | .planning/phases/04-refresh-and-anchor/04-06-SUMMARY.md |

## Self-Check: PASSED

Verified each commit hash exists in `git log --all`:

- `9e7cfff` — present
- `6513913` — present
- `b2fe1cb` — present

Verified each file modified by this plan exists at its declared path:

- `docs/phase-2/realism-risks-register.md` — present (443 lines)
- `docs/phase-2/coverage-check.md` — present (154 lines)
- `.planning/phases/04-refresh-and-anchor/04-06-SUMMARY.md` — created by this commit

Verified the §"Index" table verdict cells match per-entry `**Verdict:**` fields for the 5 entries Phase 4 touched (all five read DISCLOSED in both places).

Verified all `.planning/...` cross-reference paths in `docs/phase-2/realism-risks-register.md` resolve on disk (21 unique paths checked; all present).
