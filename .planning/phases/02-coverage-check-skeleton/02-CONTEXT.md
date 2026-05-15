# Phase 2: Coverage Check Skeleton - Context

**Gathered:** 2026-05-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Build `docs/phase-2/coverage-check.md` — a flat Requirements-Traceability-Matrix-style table that maps every menu-item trade-off claim the Cardano Improvement Proposal (CIP) will make to a specific backing simulator job. Each row is one `(claim, menu-option)` pair with the full column set: claim text, menu-option, backing suite, backing job, seeds cited, confidence method, golden `sha256`, status verdict, and cross-references to register `RSK-NN` identifiers. Non-welfare property columns (anti-bribery, standard-user-fee-drift-exposure, signal-source-anchoring, implementation-complexity) sit alongside welfare claims so the menu remains a menu rather than collapsing into welfare-only advocacy (PITFALLS CRIT-2 prevention).

This phase produces the SKELETON. `BACKED` rows are filled in during Phase 3 as cheap-test results arrive. Phase 2's job is to enumerate every claim, lay out the column structure, populate cells with what's available NOW from existing artefacts, mark gaps as `UNBACKED`, and surface compute priorities for Phase 3 task ordering.

Pure documentation work; no simulator code is touched. Inputs are existing artefacts: the Phase 1 register, the 7 goldens-pinned suite READMEs, the 12 unpinned demand-regime suite YAMLs, and the family-B analysis trio (`family-b-decision-2026-05-14.md`, `family-b-results-table-2026-05-14.md`, `mechanism-welfare-impact-2026-05-14.md`).

Requirements covered: COV-01, COV-02, COV-03, COV-04, COV-06.

</domain>

<decisions>
## Implementation Decisions

### Claim taxonomy (D-11)

The coverage check enumerates four classes of claim, all in one table:

- **Welfare claims** — numeric assertions about a menu option's welfare metric ("un-reserved both-dynamic median welfare > X under demand profile Y"). Confidence method: simulator job + variance bands (paired-bootstrap Bias-corrected and accelerated (BCa) confidence intervals if Phase 3 has run, or single-seed point estimate flagged `WEAK` otherwise).
- **Comparative claims** — ordering or sign assertions between two menu options ("priority-only-RB-reserved < both-dynamic-un-partitioned on net-utility in 4/5 demand regimes"). Confidence method: Paired Seed Evaluation (PSE) delta if Phase 3 has run, or sign-coherence count from the existing 33-job sundaeswap-smoke otherwise.
- **Structural / by-construction claims** — assertions that hold by the mechanism's construction rather than empirical measurement ("Family B is reorg-safe by construction", "the chain-derived controller cannot suffer node-local accumulator contamination"). Confidence method: citation to a spec section in [`docs/phase-2/mechanism-design.md`](../../../docs/phase-2/mechanism-design.md), a proof in [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md), or a `RSK-NN` entry that disclosed the limit.
- **Calibration claims** — assertions that a parameter value is anchored to mainnet data ("rb-generation-probability = 0.05 anchored to Cardano mainnet `activeSlotsCoeff`"). Confidence method: a `(value, source, date-retrieved)` triple cited from the refreshed [`cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md) (Phase 4 / DOC-01 will fold those triples in; for Phase 2 the citation may be to the current annotated audit document).

All four classes share the same `CLM-NN` namespace (append-only); the `confidence-method` column distinguishes them. Expected total: ~25–40 CLM entries (table rows are per-(claim, option) pairs — see D-13 — so ~5–10 distinct claims × 5 menu options).

### Source of claims — extract-then-augment (D-12)

The executor enumerates claims by reading four sources in priority order:

1. [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md) — every welfare-impact and headline claim
2. [`.planning/family-b-full-sweep-analysis-2026-05-14.md`](../../family-b-full-sweep-analysis-2026-05-14.md) and [`.planning/family-b-results-table-2026-05-14.md`](../../family-b-results-table-2026-05-14.md) — numeric and comparative claims with their backing jobs
3. [`.planning/mechanism-welfare-impact-2026-05-14.md`](../../mechanism-welfare-impact-2026-05-14.md) — the mechanism-independence claim and the four sign-flip cells named there
4. The seven goldens-pinned suite READMEs under [`sim-rs/parameters/phase-2-sweep/suites/`](../../../sim-rs/parameters/phase-2-sweep/suites/) — per-suite framing, especially the M4 suites' README narratives

User augmentation in `<specifics>` below seeds additional claims the executor should ensure are enumerated even if they're not surfaced in those four source documents — primarily structural / by-construction claims and explicit calibration anchoring claims that may be discussed in [`CLAUDE.md`](../../../CLAUDE.md) §"Calibration choices" but not pulled out as named "claims" in the family-B artefacts. (See the `<specifics>` block for the initial seed list.)

### Row shape — per-(claim, menu-option) pair (D-13)

The table is denormalised: one row per `(claim, menu-option)` pair. With ~5 menu options × ~5–10 distinct claim types, the row count is ~25–40. Each row is atomic — greppable by claim ID, by menu option, by status, by RSK reference. This matches the Leios [`docs/ImpactAnalysis.md`](https://github.com/input-output-hk/ouroboros-leios/blob/main/docs/ImpactAnalysis.md) precedent and the standard Requirements Traceability Matrix shape.

When a single claim genuinely applies identically to all 5 menu options, the executor still emits 5 rows (one per option) with identical content — the redundancy is acceptable because the alternative (omitting rows where the claim doesn't differ by option) makes the table inconsistent and harder to filter. Comparative claims emit one row per ordered pair where the comparison is asserted (e.g. `(priority-only-RB-reserved, both-dynamic-un-partitioned)` is one CLM with one row capturing the `<` assertion).

Five menu options in the namespace:
- `priority-only-RB-reserved`
- `priority-only-un-reserved`
- `both-dynamic-partitioned`
- `both-dynamic-un-partitioned`
- `single-lane-EIP-1559-control` (control evidence only, not a CIP menu item — per the initialization-questioning decision; retained for baseline comparison)

### Non-welfare cell semantics — mixed enum + citation (D-14)

The four mandatory non-welfare property columns (per REQ-COV-03) each have a controlled enum vocabulary plus a citation or quantitative bound:

- **`anti-bribery`** ∈ `{formal, informal, absent}` + spec-section citation (or `disclosed gap` linking to the relevant `RSK-NN`). `formal` = the mechanism design proves the property by construction (cite spec section); `informal` = the property holds in practice but is not load-bearing on a formal argument; `absent` = the mechanism does not provide this property and the gap is disclosed.
- **`signal-source-anchoring`** ∈ `{mainnet-data-cited, spec-default, unanchored}` + citation. `mainnet-data-cited` = the controller knob value is anchored to a `(value, source, date-retrieved)` triple in [`cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md); `spec-default` = the value matches an EIP-1559 spec default with citation; `unanchored` = no external anchor, names the `RSK-NN` that disclosed it (one of the four un-anchored controller knobs from Phase 1).
- **`standard-user-fee-drift-exposure`** ∈ `{none, bounded, exposed}` + quantitative bound where applicable. `none` = the mechanism does not perturb the standard-lane fee for standard-lane traffic; `bounded` = perturbation exists but is bounded by a stated maximum (cite the `Δfee/byte` bound and the source job); `exposed` = un-bounded perturbation; cite the `RSK-NN` that disclosed it.
- **`implementation-complexity`** ∈ `{low, medium, high}` + Lines-of-Code (LoC) estimate or "+/- N modules" change scope. `low` = changes < 100 LoC in a single module; `medium` = 100–500 LoC across 2–3 modules; `high` = > 500 LoC or > 3 modules.

Cell format: `<enum-value> (<citation>)` where the citation is a backtick-quoted file:line or RSK-NN reference. Example: `formal (mechanism-design.md §"Priority lane partitioning")` or `unanchored (RSK-window-length-32)`.

### Decisions carried forward (D-15..D-21)

- **D-15** ID convention: `CLM-NN` for coverage rows, append-only, never renumbered. Cross-references to register identifiers use `RSK-NN` directly in the `related-RSK-ids` column. (From Phase 1 D-05; extended to CLM namespace.)
- **D-16** Verdict vocabulary for coverage rows is exactly four values: `BACKED` / `WEAK` / `UNBACKED` / `OUT-OF-SCOPE`. Different vocabulary from the register's LIVE / DORMANT / MITIGATED / DISCLOSED. `BACKED` requires the hash-diversity gate (D-19); rows whose evidence is below that bar default to `WEAK` with annotation, or `UNBACKED` if no evidence exists at all.
- **D-17** Required columns (per REQ-COV-02): `id` (CLM-NN), `claim` (text), `menu-option` (one of the 5 above), `backing-suite` (path or "—" if UNBACKED), `backing-job` (job name or "—"), `seeds-cited` (integer or "—"), `confidence-method` (e.g. `paired-bootstrap BCa N=20`, `sign-coherence N=5`, `by-construction (spec §X)`, `mainnet-anchored (value, source, date)`, or "—"), `golden-sha256` (hex or "—"), `status` (BACKED / WEAK / UNBACKED / OUT-OF-SCOPE), `related-RSK-ids` (comma-separated). Additional non-welfare property columns per D-14 above.
- **D-18** The 12 unpinned demand-regime suites (`paper_like_*`, `sundaeswap_*` beyond the 7 goldens-pinned set) appear as `WEAK`-verdict rows where they cover claims not backed by goldens-pinned suites. They are NOT promoted to goldens-pinned in this milestone; the `WEAK` verdict carries the disclosure that the row's evidence is not under the 3-layer determinism regime. (From REQ-COV-04; init-questioning decision.)
- **D-19** Hash-diversity gate is **strict** — a row can only be cited as `BACKED` when the distinct `pricing_event_stream.sha256` count equals the seed count. Rows where seeds collapse to fewer distinct hashes are downgraded to `WEAK` with annotation, or the cell is re-run with different seed values in Phase 3. (From REQ-COV-05; init-questioning decision; gates Phase 3 BACKED promotions, not Phase 2 skeleton.)
- **D-20** Coverage check skeleton is committable before Phase 3 begins: rows for claims awaiting cheap-test results carry `status: UNBACKED`, surfacing compute priorities for Phase 3 task ordering. (From REQ-COV-06.)
- **D-21** Abbreviation-on-first-use rule (per [`CLAUDE.md`](../../../CLAUDE.md) §"Conventions / gotchas") applies to coverage-check.md prose, including column headers and enum vocabulary first introductions.

### Claude's Discretion

The following items were not raised in discussion; the planner / executor may apply reasonable defaults consistent with the locked decisions above:

- **Column ordering.** Suggested order: `id | claim | menu-option | status | confidence-method | backing-suite | backing-job | seeds-cited | golden-sha256 | anti-bribery | signal-source-anchoring | standard-user-fee-drift-exposure | implementation-complexity | related-RSK-ids`. The non-welfare property columns sit to the right of welfare-backing columns so a casual reader's first scan hits status + welfare evidence; reviewers who want non-welfare comparison scroll right.
- **Status enum priority on conflicting evidence.** When a cell has both a goldens-pinned job (BACKED-eligible) AND an unpinned-suite job (WEAK-eligible), prefer the goldens-pinned job and mark BACKED, ignore the unpinned-suite job for that row.
- **UNRESOLVED suites output-read scope.** The Phase 1 SUMMARY-2 named four UNRESOLVED non-pinned suites (`phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`). The executor walks their `sim-rs/output/` directories (if present) for existing run data; if data exists, the corresponding rows go `WEAK` (citing the unpinned-suite job); if no data exists, the rows go `UNBACKED`. The executor does NOT re-run any of these suites in Phase 2 — runs are Phase 3 work.
- **Multi-RSK rows.** A single `(claim, menu-option)` row may reference multiple `RSK-NN` entries in `related-RSK-ids` if multiple registered risks bear on its evidence. Comma-separate; no per-RSK sub-rows.
- **EXP-NN forward references.** For UNBACKED rows whose backing test exists as an `EXP-NN` in the register, include the `EXP-NN` slug in a parenthetical inside the `claim` cell so the Phase 3 planner can map work items back to coverage rows. Example: `un-reserved both-dynamic median welfare > X (EXP-canonical-variance)`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level
- [`.planning/PROJECT.md`](../../PROJECT.md) — Project context, core value, Active requirements
- [`.planning/REQUIREMENTS.md`](../../REQUIREMENTS.md) — REQ-IDs covered by this phase (COV-01..04, COV-06); also COV-05 hash-diversity gate semantics (applied in Phase 3 but referenced here)
- [`.planning/ROADMAP.md`](../../ROADMAP.md) §"Phase 2: Coverage Check Skeleton" — goal, dependencies, 5 success criteria
- [`CLAUDE.md`](../../../CLAUDE.md) — Abbreviation-on-first-use rule (§"Conventions / gotchas"), calibration choices, mechanism abstractions

### Phase 1 outputs (direct inputs)
- [`docs/phase-2/realism-risks-register.md`](../../../docs/phase-2/realism-risks-register.md) — the 24 RSK-NN entries that the `related-RSK-ids` column references
- [`.planning/phases/01-register-inventory/01-CONTEXT.md`](../01-register-inventory/01-CONTEXT.md) — decisions D-01..D-10 plus Claude's Discretion items inherited as substrate; particularly D-04 (spike READMEs are evidence sources, not register inputs — same rule applies to coverage check)
- [`.planning/phases/01-register-inventory/01-02-SUMMARY.md`](../01-register-inventory/01-02-SUMMARY.md) — names the three Phase-2-facing EXP-NN slugs (`EXP-unresolved-output-read`, `EXP-coverage-non-welfare-columns`, `EXP-hash-diversity-policy-decision`), the four UNRESOLVED non-pinned suites, and the TEST-07a sub-requirement

### Source documents for claim enumeration (D-12)
- [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md) — authoritative mechanism-commit memo with publication framing; primary source for headline welfare and structural claims
- [`.planning/family-b-full-sweep-analysis-2026-05-14.md`](../../family-b-full-sweep-analysis-2026-05-14.md) — full-sweep characterisation; primary source for numeric and comparative welfare claims
- [`.planning/family-b-results-table-2026-05-14.md`](../../family-b-results-table-2026-05-14.md) — numeric results table; primary backing-job source for welfare claim rows
- [`.planning/mechanism-welfare-impact-2026-05-14.md`](../../mechanism-welfare-impact-2026-05-14.md) — empirical Family-B-vs-accumulator characterisation; sources the mechanism-independence claim and the four sign-flip cells (`d4_t50_w32`, `d8_t25_w32`, `x4_rb_quarter` × 2 arms)
- [`sim-rs/parameters/phase-2-sweep/suites/`](../../../sim-rs/parameters/phase-2-sweep/suites/) — seven goldens-pinned suite YAMLs + READMEs (M4 suites only) for per-suite framing
- [`docs/phase-2/cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md) — currently-annotated calibration audit; the source for calibration-claim citations (will be refreshed in Phase 4 / DOC-01 but Phase 2 may cite the current annotated form)

### Suite layout context
- [`sim-rs/parameters/phase-2-sweep/suites/.goldens/`](../../../sim-rs/parameters/phase-2-sweep/suites/.goldens/) — the 7 goldens-pinned hash directories; `golden-sha256` column values come from here
- [`CLAUDE.md`](../../../CLAUDE.md) §"Running the suites" — the 7 suites table; informs the M3-vs-M4 framing distinction
- The four UNRESOLVED non-pinned suites named in Phase 1 SUMMARY-2: `phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`

### Conventions and methodology
- [`.planning/research/STACK.md`](../../research/STACK.md) — Requirements Traceability Matrix idiom; `CLM-*` precedent from Leios `ImpactAnalysis.md`
- [`.planning/research/FEATURES.md`](../../research/FEATURES.md) — table-stakes / differentiator / anti-feature catalogue; especially CRIT-2 menu-collapsing prevention via non-welfare property columns
- [`.planning/research/PITFALLS.md`](../../research/PITFALLS.md) — CRIT-2 details: non-welfare property columns prevent the menu collapsing into welfare-only advocacy
- [Leios `docs/ImpactAnalysis.md`](https://github.com/input-output-hk/ouroboros-leios/blob/main/docs/ImpactAnalysis.md) — `RSK-*`/`EXP-*`/`CLM-*` identifier convention; coverage-table shape precedent

</canonical_refs>

<code_context>
## Existing Code Insights

This is a documentation-only phase. No simulator code is created or modified. The "assets" are existing artefacts enumerated under canonical refs above.

### Reusable Assets

- **Register row schema** from Phase 1's `realism-risks-register.md` — the column-and-section format is the closest in-repo precedent for what coverage-check.md should look like. The denormalised one-row-per-entry pattern matches the per-(claim, option) row shape (D-13).
- **Suite goldens directory** at `sim-rs/parameters/phase-2-sweep/suites/.goldens/` — each pinned suite has a `.sha256` file with the `pricing_event_stream` hash. The `golden-sha256` column values come from these files; the executor can read the directory to populate the column.
- **Family-B results table** (`family-b-results-table-2026-05-14.md`) — already structured as a CLM-shaped table by mechanism arm × (job, seed) cell; many of its rows can be lifted into CLM-NN entries with minimal restructuring.

### Established Patterns

- **Stable, append-only identifiers** for cross-document traceability (REVIEW.md's `WR-NN`, `CR-NN`; register's `RSK-NN`; this phase's `CLM-NN`).
- **Date-anchored time-sensitive citations** for calibration claims — `(value, source, date-retrieved)` triples; matches the `cardano-realism-audit.md` annotation pattern.
- **Citation-with-section** for structural claims — file-path § "section-heading" format; matches the register's evidence-for / evidence-against pattern.

### Integration Points

- Phase 3 (Targeted Cheap Tests) reads `UNBACKED` rows to prioritise test work. Each `UNBACKED` row should name an `EXP-NN` slug in its `claim` cell (Claude's Discretion item) so the Phase 3 planner can map cheap tests to coverage rows.
- Phase 3 (COV-05 application) reads the BACKED rows, runs hash-diversity checks, and downgrades any row to WEAK where `distinct sha256 count < N_seeds`. This is COV-05's enforcement gate; the skeleton sets the gate semantics in the table header.
- Phase 4 (Refresh and Anchor) reads the `signal-source-anchoring` column. Rows marked `unanchored (RSK-window-length-32)` etc. become Phase 4 / DOC-03 anchor-or-disclose work items.
- Phase 5 (Handoff) reads BACKED + WEAK rows and selects the subset to cite in the CIP's Evidence section. The CIP author copies the chosen rows verbatim into the CIP.

</code_context>

<specifics>
## Specific Ideas

User-seeded claims that the executor should ensure are enumerated in addition to anything extracted from the four source documents in D-12:

- **Mechanism-independence (Family A vs Family B robustness).** Confidence method: paired Family-A-vs-Family-B welfare delta from `mechanism-welfare-impact-2026-05-14.md` (33-job sundaeswap-smoke; median |Δ%| ≈ 15 % on un-reserved arms, 0/3 and 0/2 sign-flips). One row per menu option; the un-reserved arms land BACKED-WEAK (single-seed evidence at N=1 from the 33-job smoke), the partitioned and RB-reserved arms land WEAK with disclosure of the four sign-flip cells.
- **Reorg-safety by construction (Family B closes WR-1).** Structural claim. Confidence method: by-construction citation to `family-b-decision-2026-05-14.md` §"What changed" + `mechanism-design.md` §"Chain-derived controller". One row per menu option; all five options BACKED via construction (no empirical job needed).
- **Calibration: `rb-generation-probability = 0.05` anchored to mainnet `activeSlotsCoeff`.** Calibration claim. Confidence method: mainnet-anchored triple `(0.05, Cardano mainnet activeSlotsCoeff, 2026-05-14 retrieval)` from `cardano-realism-audit.md` §"RB cadence". One row per menu option; all five options BACKED.
- **Calibration: `topology-realistic-100.yaml` stakes are mass-stratified epoch-582 mainnet snapshot.** Calibration claim. Confidence method: mainnet-anchored triple from spike 006. One row per menu option; all five options BACKED.
- **Anti-bribery property per menu option.** Four rows for the four CIP menu options + 1 control row. The executor populates the `anti-bribery` column with the appropriate enum value per option based on the mechanism's structure (un-reserved arms: informal; RB-reserved arms: stronger property; control: absent). Cite `mechanism-design.md` sections.
- **Standard-user-fee-drift-exposure per menu option.** As above; the executor populates this column per option. Both-dynamic arms expose standard users to drift; priority-only arms do not perturb the standard-lane fee.
- **Implementation-complexity per menu option.** All five options share most implementation cost; the differences are in: validity-rule presence (RB-reserved variants need PriorityOnly validity), partition-activation tracking (partitioned variant needs the `partition_activated` flag on EB), un-reserved priority-only signal-source choice (low LoC). Executor estimates LoC delta from `sim-rs/sim-core/src/tx_pricing/` and the linear-leios block-production additions.

The executor should NOT take these as a closed list — additional claims surfaced by reading the family-B trio (or that come up naturally) should also be enumerated. The role of this seed is to ensure the four structural/calibration classes (mechanism-independence, reorg-safety, calibration anchoring, anti-bribery/drift/complexity per option) are not missed when scanning welfare-heavy source documents.

</specifics>

<deferred>
## Deferred Ideas

- **Filling BACKED rows with paired-bootstrap BCa confidence intervals.** Phase 3 work — depends on `paired_bootstrap.rs` (TEST-01) and the multi-seed runs (TEST-02..06). Phase 2 leaves these rows `UNBACKED` with `confidence-method = TBD Phase 3` annotation.
- **Hash-diversity gate application.** Phase 3 work — the skeleton names the gate semantics in the table header but does not run the gate against any row (since BACKED rows don't exist yet).
- **Refresh of `signal-source-anchoring` rows from `unanchored` to `mainnet-data-cited`.** Phase 4 / DOC-03 work — for any of the four un-anchored controller knobs that acquire an external anchor via the 2-hour literature search at Phase 4 open.
- **Promotion of any unpinned demand-regime suite to goldens-pinned.** Out of scope per REQ-COV-04 and PROJECT.md.
- **Cross-reference index automation.** Phase 1 deferred. Still deferred.
- **CIP Evidence-section row selection.** Phase 5 / HAND-01 work.

No reviewed-todo deferrals (the `cross_reference_todos` step returned an empty matches set).

</deferred>

---
*Phase: 2-Coverage Check Skeleton*
*Context gathered: 2026-05-15*
