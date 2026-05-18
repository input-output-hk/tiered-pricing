---
phase: 04-refresh-and-anchor
plan: 01
subsystem: docs
tags: [docs, anchor-or-disclose, literature-search, controller-calibration, eip-1559, cip-evidence]

# Dependency graph
requires:
  - phase: 01-register-inventory
    provides: RSK-un-anchored-controller-knobs umbrella entry (lines 80-104) — the four-sub-knob enumeration this plan re-grades
  - phase: 03-targeted-cheap-tests
    provides: TEST-07a regime-dependence finding (.planning/realism-tests/multiplier-floor-16-companion/results.md) — cited in sub-knob 2 disclosure rationale
provides:
  - Per-sub-knob anchor decisions for the four un-anchored controller knobs (window-length 32 ANCHORED; multiplier-floor 4 DISCLOSED; multiplier-floor 16 DISCLOSED; lane-signal-source DISCLOSED)
  - Draft (value, source, date-retrieved) triples for cardano-realism-audit.md §"Pricing-controller calibration" refresh (Plan 04-04 paste-target)
  - Draft register prose blocks for RSK-un-anchored-controller-knobs.disclosure-paragraph (Plan 04-06 paste-target)
  - Umbrella verdict recommendation: LIVE → DISCLOSED with sub-knob granularity (only 1 of 4 sub-knobs anchors)
  - Rejected-citations list with one-line rationale per rejection (reviewer traceability per D-37)
affects:
  - Plan 04-04 (cardano-realism-audit.md §"Pricing-controller calibration" refresh)
  - Plan 04-06 (realism-risks-register.md RSK-un-anchored-controller-knobs verdict flip + disclosure-paragraph rewrite)
  - Plan 04-07 (Wave-3 consistency review may optionally run the 2024-2026 arXiv follow-up pass)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "D-35 motivating-citation bar applied per knob: anchor iff a published paper or deployed-system reference motivates the kind of choice the simulator makes"
    - "Per-sub-knob disposition block shape (Disposition / Citations consulted / Decision rationale / Draft audit-section copy / Draft register prose) for Wave-2 paste consumption"
    - "Explicit search cut-off documentation per D-37: rationale recorded before exit; 2024-2026 follow-up pass scoped but deferred to Plan 04-07's discretion"

key-files:
  created:
    - .planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md
  modified: []

key-decisions:
  - "Window length 32 ANCHORED at D-35 bar: Reijsbergen et al. AFT 2021 (chaotic short-term oscillation finding) plus Liu et al. CCS 2022 (empirical counter-bound) plus Leonardos et al. AFT 2021 (theoretical bounded-oscillation) motivate the kind of choice — a smoothing layer over the unwindowed Ethereum baseline. The specific length 32 is within the {16, 32, 64} sweep band, not a numerical anchor."
  - "Multiplier-floor 4 DISCLOSED: no external anchor exists for a second-lane multiplier (Ethereum has no second lane). The value 4 is an internal calibration accommodation per CLAUDE.md §'Calibration choices'; TEST-07a evidences the welfare regime-dependence at floor = 16, which the disclosure paragraph names explicitly."
  - "Multiplier-floor 16 DISCLOSED: no external anchor exists for the magnitude; mechanism-design.md line 155 / 290 declares the default without citing calibration data; the 'strong price-discrimination guarantee' justification is spec-internal."
  - "Lane-signal-source choices DISCLOSED: the specification explicitly leaves both choices open (mechanism-design.md lines 207-211 for un-reserved priority; line 238 for both-dynamic standard); the EIP-1559 literature addresses single-lane controllers only; no deployed dynamic-pricing system (Sui, Solana, NEAR) has a comparable second-lane signal-source choice."
  - "Umbrella verdict: LIVE → DISCLOSED (1 of 4 sub-knobs anchors; cannot land MITIGATED under the umbrella-grouping rule which requires all four to anchor). Plan 04-06 reads this as the verdict-flip recommendation."
  - "Search cut-off per D-37 documented inline: the 2024-2026 arXiv follow-up pass was not executed in this run (web-fetch tooling unavailable for this executor) but is verdict-robust — for sub-knobs 2/3/4 no follow-up can surface anchors because no comparable deployed second-lane mechanism exists, and for sub-knob 1 a follow-up would only strengthen the existing anchor. Plan 04-07 may run the follow-up pass at its discretion."

patterns-established:
  - "Citation-consultation log per sub-knob: each citation tested against each sub-knob with anchor-supporting / rejected outcome and one-line rationale, enabling Plan 04-07 to verify the consultation actually occurred (T-04-01 mitigation)"
  - "Self-contained register-prose paragraphs: each per-sub-knob Draft register prose block expands every abbreviation on first use within the block in isolation, so the paragraph pastes verbatim into the umbrella entry's disclosure-paragraph without requiring the reader to consult the rest of the planning artefact"

requirements-completed: [DOC-03]

# Metrics
duration: ~15min
completed: 2026-05-18
---

# Phase 04 Plan 01: DOC-03 Anchor-or-Disclose Literature Search Summary

**Window-length 32 ANCHORED to Reijsbergen / Liu / Leonardos EIP-1559 academic-critique; multiplier-floor 4, multiplier-floor 16, and lane-signal-source choices DISCLOSED at D-35; umbrella verdict LIVE → DISCLOSED.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-05-18T12:40Z (approx.)
- **Completed:** 2026-05-18T12:55Z
- **Tasks:** 2 of 2 complete
- **Files modified:** 1 (created)

## Accomplishments

- Recorded per-sub-knob anchor-or-disclose dispositions for all four sub-knobs of `RSK-un-anchored-controller-knobs` under the D-35 motivating-citation bar
- Drafted `(value, source, date-retrieved)` triples for the four sub-knobs ready for Plan 04-04 to fold into `docs/phase-2/cardano-realism-audit.md` §"Pricing-controller calibration"
- Drafted self-contained register prose paragraphs for Plan 04-06 to paste into `RSK-un-anchored-controller-knobs.disclosure-paragraph`
- Recommended the umbrella verdict flip: LIVE → DISCLOSED (with sub-knob granularity preserved)
- Listed 10 rejected citations with one-line rationale each for reviewer traceability per D-37 / T-04-01

## Task Commits

Each task was committed atomically:

1. **Task 1: Literature search + per-knob anchor decisions** — `4ff3b32` (docs)
2. **Task 2: Umbrella verdict + rejected citations + abbreviation audit** — `0062e80` (docs)

## Files Created/Modified

- `.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md` — 167-line literature-search log with: §"Search methodology and cut-off" (citations-consulted table + D-37 cut-off rationale); §"Per-knob anchor decisions" (four sub-knob blocks each with Disposition / Citations consulted / Decision rationale / Draft audit-section copy / Draft register prose); §"Umbrella verdict for RSK-un-anchored-controller-knobs" (LIVE → DISCLOSED recommendation + 4-row disposition table); §"Rejected citations" (10 rejection lines + 2024-2026 follow-up pass disposition); §"Abbreviations expanded on first use (in-document audit)" (self-check against CLAUDE.md first-use rule).

## Consulted-vs-rejected citation counts

- **Citations consulted:** 8 distinct sources (Liu CCS 2022; Reijsbergen AFT 2021; Leonardos AFT 2021; Azouvi DISC 2023; Roughgarden EC 2021 + working paper; EIP-1559 specification; CIP-0164; Sui / Solana / NEAR deployed-system docs as one triangulation set; Consensys cross-confirmation), plus three in-repo references (`docs/phase-2/mechanism-design.md`; `CLAUDE.md` §"Calibration choices"; TEST-07a results) substantively engaged in the per-sub-knob rationale.
- **Citations anchoring:** 3 distinct sources anchor sub-knob 1 (Reijsbergen AFT 2021 as primary; Liu CCS 2022 as counter-bound; Leonardos AFT 2021 as theoretical complement); 1 secondary anchor (Azouvi DISC 2023; window-smoothing-weakens-manipulation-attack).
- **Citations rejected:** 10 rejection lines in the consolidated list (5 EIP-1559 primary citations rejected for sub-knobs 2/3/4; 3 deployed-system docs rejected for all four sub-knobs; CIP-0164 rejected for all four sub-knobs; Consensys rejected as non-independent).

## Umbrella verdict recommendation

**LIVE → DISCLOSED** (with sub-knob granularity: 1 of 4 sub-knobs ANCHORED, 3 of 4 DISCLOSED).

| Sub-knob | Disposition |
|---|---|
| 1. Window length 32 (capacity-varying signals) | **ANCHORED** |
| 2. Multiplier-floor 4 (`phase-2-rb-scarcity`, `phase-2-urgency-inversion`) | **DISCLOSED** |
| 3. Multiplier-floor 16 (spec default) | **DISCLOSED** |
| 4. Lane-signal-source choices | **DISCLOSED** |

Plan 04-06 reads this as the input to flip the register entry's verdict from LIVE to DISCLOSED and to rewrite the entry's `disclosure-paragraph` using the four per-sub-knob `Draft register prose` blocks pasted in order (a), (b), (c), (d).

## Decisions Made

See frontmatter `key-decisions`. Six explicit decisions recorded (one per sub-knob plus the umbrella verdict and the search cut-off).

## Deviations from Plan

**1. [Search-budget cut-off note] 2024-2026 arXiv follow-up pass deferred to Plan 04-07's discretion**

- **Found during:** Task 1 (literature search)
- **Issue:** The plan scopes the 2024-2026 follow-up pass as optional ("any 2024-2026 Ethereum gas-fee-market literature surfacing in a basic Google Scholar / arXiv pass"). The executor environment for this run does not expose `WebSearch` / `WebFetch` tools, so the follow-up could not be performed directly during execution.
- **Resolution applied:** Per D-37, the cut-off decision is recorded explicitly in §"Search methodology and cut-off" of the artefact. The disposition is verdict-robust: for sub-knobs 2 / 3 / 4 no follow-up can surface anchors because no comparable deployed second-lane mechanism exists, so the marginal-new-citation expectation is zero; for sub-knob 1 a follow-up would strengthen the existing anchor without flipping the disposition. The artefact explicitly flags that Plan 04-07 (Wave-3 consistency review) may run the optional follow-up at its discretion and re-grade.
- **Files modified:** None beyond the artefact.
- **Verification:** §"Search methodology and cut-off" records the cut-off rationale; the umbrella verdict and per-sub-knob dispositions are robust to a follow-up re-grade in the direction of more anchors only.
- **Committed in:** `4ff3b32` (Task 1 commit).

**Note:** Spike 003 (`.planning/spikes/003-pricing-controller-calibration/README.md`, dated 2026-05-13) is the substantive WebFetch-retrieved evidence base for the three primary citations; the spike's citations are quoted in this artefact's citations-consulted table with retrieval dates. The artefact does not synthesise new web-retrieved material beyond Spike 003's; it re-grades Spike 003's NEEDS-DISCLOSURE finding under the D-35 motivating-citation bar.

---

**Total deviations:** 1 (search-budget cut-off note — within Plan / D-37 scope; not a Rule 1/2/3 auto-fix).
**Impact on plan:** None on the per-sub-knob dispositions; the cut-off is documented per D-37 for reviewer traceability. Plan 04-07 may extend the search at its discretion.

## Issues Encountered

None. Spike 003 (2026-05-13) and the in-repo CLAUDE.md / mechanism-design.md content together provided sufficient consultation material to record per-sub-knob dispositions definitively under D-35.

## Self-Check

Verified:

- `.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md` exists (167 lines; well above the 80-line `min_lines` floor).
- Four `### Sub-knob N:` headings present (verified via `grep -cE`).
- Four `Disposition:` lines with ANCHORED or DISCLOSED (verified via `grep -cE`).
- `## Umbrella verdict for RSK-un-anchored-controller-knobs` heading present.
- `## Rejected citations` heading present.
- "LIVE → DISCLOSED" recommendation present (3 occurrences across the file).
- Zero `\bTBD\b` markers anywhere in the file.
- Commit `4ff3b32` exists in `git log` (Task 1).
- Commit `0062e80` exists in `git log` (Task 2).
- Abbreviation-on-first-use rule audited per CLAUDE.md §"Conventions / gotchas": CIP, EIP-1559, CCS, AFT, RB, EB, AIMD, RB-reserved, KiB, MiB, MMIC, OCA, DSIC all expanded on first use in the document and within each per-sub-knob Draft register prose block in isolation.

## Self-Check: PASSED

## Next Phase Readiness

- **Plan 04-04 (Wave 2)** can fold the four per-sub-knob `Draft audit-section copy` blocks verbatim into `docs/phase-2/cardano-realism-audit.md` §"Pricing-controller calibration". The four `(value, source, date-retrieved)` triples are paste-ready.
- **Plan 04-06 (Wave 2)** can fold the four per-sub-knob `Draft register prose` blocks verbatim into the `RSK-un-anchored-controller-knobs.disclosure-paragraph` field (in order a/b/c/d) and flip the umbrella verdict from LIVE to DISCLOSED per the §"Umbrella verdict" recommendation.
- **Plan 04-07 (Wave 3)** consistency review may at its discretion run the deferred 2024-2026 arXiv follow-up pass. The §"Rejected citations" §"2024-2026 follow-up arXiv pass" sub-section records the cut-off rationale Plan 04-07 challenges.

---
*Phase: 04-refresh-and-anchor*
*Completed: 2026-05-18*
