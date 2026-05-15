# Phase 1: Register Inventory - Context

**Gathered:** 2026-05-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Build `docs/phase-2/realism-risks-register.md` — the single source of truth for every realism risk affecting the CIP's claims. Phase 1 inventories every risk-shaped statement across the existing audit-trail (six source documents + seven spike READMEs), de-duplicates them into stable `RSK-NN` entries with Wohlin-four-fold categories, populates `scope-of-resolution` fields that will discipline Phase 3's cheap tests, and writes CIP-pasteable `disclosure-paragraph` prose for entries that will land as DISCLOSED.

The phase is pure documentation work — no simulator code is touched. Inputs are existing artefacts; output is one new file plus a thin redirect-stub in Phase 4 for `validity-threats.md`.

Requirements covered: REG-01, REG-02, REG-03, REG-04, REG-05.

</domain>

<decisions>
## Implementation Decisions

### Relationship to existing audit-trail documents

The single architectural decision of this phase. Each source doc has a defined fate:

- **D-01:** `docs/phase-2/validity-threats.md` (spike-005 output) is **split** across the new register and the new coverage check. Per-threat risk descriptions become `RSK-NN` entries in the register; per-claim trust ratings become `CLM-NN` rows in the Phase 2 coverage check. In Phase 4 (DOC-02), `validity-threats.md` is refreshed into a thin pointer document — it introduces the validity-threats framework and redirects to the register + coverage check. No content lives in two places.

- **D-02:** `docs/phase-2/cardano-realism-audit.md` **complements** the register. It is refreshed in Phase 4 (DOC-01) to be calibration-provenance only: `(value, source, date-retrieved)` triples per knob, no risk verdicts. Its current NEEDS-DISCLOSURE sections — fee-as-maxFee-envelope reinterpretation, the four un-anchored controller knobs — are pulled into the register as `RSK-NN` entries during this phase; the register's `disclosure-paragraph` field does the disclosure work. Each doc has exactly one job: audit holds parameter provenance, register holds risk inventory.

- **D-03:** `.planning/REVIEW.md` and `.planning/codebase/CONCERNS.md` **are sourced from but not superseded**. The register references relevant REVIEW.md findings (WR-2, WR-7 in particular) and CONCERNS.md items inside `evidence-for` / `evidence-against` fields. REVIEW.md and CONCERNS.md retain their original working-artefact roles — they're internal-process documents (code-review and codebase-mapping respectively), not CIP-facing. CONCERNS.md continues to be regeneratable by `/gsd-map-codebase` without disturbing the register.

- **D-04:** `.planning/mechanism-welfare-impact-2026-05-14.md` and the seven [`docs/phase-2` spike READMEs](.planning/spikes/) are **evidence sources only**, not register inputs. Register entries point at them in `evidence-for` / `evidence-against` fields. Spike verdicts (VALIDATED / NEEDS-DISCLOSURE / RECOMMENDED / ADOPT) are preserved in-place as the audit trail; they are not re-expressed as register verdicts. The register is the inventory of unresolved risks; the spikes are the resolved-or-disclosed audit trail behind them.

### Decisions carried forward from initialization questioning

- **D-05:** ID convention is Leios-style `RSK-NN` for register entries and `EXP-NN` for register-flagged experiments. `CLM-NN` for the Phase 2 coverage check (separate namespace). Append-only — identifiers never renumber.

- **D-06:** Verdict vocabulary is exactly four values: LIVE / DORMANT / MITIGATED / DISCLOSED. No half-states; cells where the verdict is ambiguous default to LIVE (most-conservative).

- **D-07:** Wohlin four-fold categorisation: construct / internal / external / conclusion. Multi-category tagging is allowed where a risk genuinely straddles categories (Wohlin literature convention; informal but standard).

- **D-08:** Required fields per `RSK-NN` entry (per REG-02): `id`, `title`, `category`, `description`, `evidence-for`, `evidence-against`, `scope-of-resolution`, `verdict`, `disclosure-paragraph` (the last is mandatory only for DISCLOSED entries; optional placeholder for LIVE entries that may land as DISCLOSED later).

- **D-09:** The four already-named LIVE entries from research are mandatory presents (REG-04): pool-count sensitivity, single-seed precision, un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source), substrate scope.

- **D-10:** `RSK-pool-count` carries the locked `scope-of-resolution` text "Δ% < seed-IQR (Inter-Quartile Range) of same job at 100 pools establishes MITIGATED" in its entry before Phase 3 begins (REG-05). This is the canonical test of "the threshold-before-the-test" discipline that PITFALLS MOD-5 calls for.

### Claude's Discretion

The user explicitly delegated these to the planner / executor with reasonable defaults named:

- **Entry granularity** — Cluster into ~20–30 thematic risks, not ~60 itemised per-source-doc statements. A risk that spans multiple source statements (e.g. "single-seed claims at publication precision" appears in PITFALLS CRIT-1, validity-threats §welfare-precision, and family-b-results-table seed-diversity column) becomes one `RSK-NN`. Per-source-doc-statement granularity is reserved for cases where the statements are genuinely different risks (rare).

- **EXP-NN ↔ TEST-NN alignment** — `EXP-NN` names align with `TEST-NN` REQ-IDs by suffix where applicable. Example mapping (illustrative, will be confirmed at register-build time): `EXP-pool-number` is realised by `TEST-05`; `EXP-sign-flip-variance` by `TEST-03`; `EXP-canonical-variance` by `TEST-04`; `EXP-run-length` by `TEST-06`. The register's `EXP-NN` field uses descriptive names; the cross-reference to `TEST-NN` is a separate column. This keeps register IDs readable while preserving REQUIREMENTS.md traceability.

- **Substrate-scope grouping** — One `RSK-NN` (provisionally `RSK-substrate-scope`) that names all three sub-points in the disclosure-paragraph: (a) upstream `f64` in non-pricing hot paths (lottery, propagation, distribution sampling), (b) propagation-model fidelity, (c) utility-maximising actor model (no adversarial / strategic bidders). One entry, one disclosure paragraph that enumerates the three sub-points. Each sub-point is individually citable from the CIP via the disclosure-paragraph anchor. Rationale: the three sub-points share a single mitigation-path (none — they're inherited substrate and out-of-scope for re-audit per PROJECT.md); separate entries would imply separate verdicts could land, which is misleading.

- **Wohlin borderline cases** — When a risk straddles categories (e.g. "single-seed claim" is both construct AND conclusion validity), the entry carries both tags. The Wohlin literature treats categories as overlapping rather than partitioning.

- **Disclosure-paragraph voice** — Match the Cardano CIP house style (CIP-0164's "Trade-offs & Limitations" section is the closest precedent). Engineering-report register; not academic-paper register. CIP author may rewrite during CIP drafting; the register's prose is a starting point, not the final wording.

- **Per-CLAUDE.md convention** — Every abbreviation expanded on first use in the register prose ("Cardano Improvement Proposal (CIP)", "Inter-Quartile Range (IQR)", "Paired Seed Evaluation (PSE)", etc.). This applies to register content, not just register metadata.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level
- `.planning/PROJECT.md` — Project context, core value, Active requirements, Out of Scope, Key Decisions
- `.planning/REQUIREMENTS.md` — REQ-IDs covered by this phase (REG-01 through REG-05); cross-reference for traceability
- `.planning/ROADMAP.md` §"Phase 1: Register Inventory" — Goal, dependencies, success criteria
- `CLAUDE.md` — Numeric-representation contract, abbreviation-on-first-use rule (under Conventions), calibration choices, mechanism abstractions

### Source documents to inventory (REG-01)
- `docs/phase-2/cardano-realism-audit.md` — Calibration-provenance audit; risk-shaped sections (fee-as-maxFee, 4 un-anchored knobs) source `RSK-NN` entries
- `docs/phase-2/validity-threats.md` — Spike-005 per-claim trust matrix; per-threat risk descriptions source `RSK-NN` entries (per-claim trust ratings go to Phase 2 coverage check)
- `.planning/REVIEW.md` — Fix Status table; WR-2, WR-7, and any LIVE / DEFERRED findings source `RSK-NN` entries via `evidence-for` / `evidence-against`
- `.planning/codebase/CONCERNS.md` — Comprehensive tech-debt / fragility inventory; risk-shaped items reference into register
- `.planning/mechanism-welfare-impact-2026-05-14.md` — Family B vs accumulator cadence evidence; the four sign-flip cells (`d4_t50_w32`, `d8_t25_w32`, `x4_rb_quarter` × 2 arms) anchor `RSK-single-seed-precision` evidence

### Spike READMEs (audit-trail evidence, not register inputs)
- `.planning/spikes/MANIFEST.md` — Index across all seven spikes with per-spike verdicts
- `.planning/spikes/001-rb-cadence-and-capacity/README.md` — VALIDATED
- `.planning/spikes/002-fee-structure-and-mempool-sizing/README.md` — NEEDS-DISCLOSURE (sources `RSK` for fee-as-maxFee)
- `.planning/spikes/003-pricing-controller-calibration/README.md` — NEEDS-DISCLOSURE (sources `RSK` for un-anchored knobs)
- `.planning/spikes/004-topology-and-actor-model/README.md` — NEEDS-DISCLOSURE (substrate scope for actor model and topology)
- `.planning/spikes/005-validity-threats/README.md` — RESOLVED (points at `docs/phase-2/validity-threats.md`)
- `.planning/spikes/006-curve-design/README.md` — RECOMMENDED (calibration-provenance, not risk)
- `.planning/spikes/007-chain-derived-controller/README.md` — ADOPT (Family B decision; closed; not register input)

### Research artefacts (methodology references)
- `.planning/research/SUMMARY.md` — Synthesized roadmap and recommended approach; explicitly identifies the 5 critical pitfalls and 4 named LIVE entries
- `.planning/research/STACK.md` — Leios `RSK-*`/`EXP-*` ID precedent, Wohlin four-fold framework, PSE methodology
- `.planning/research/FEATURES.md` — Table-stakes / differentiator / anti-feature catalogue for CIP evidence packages
- `.planning/research/ARCHITECTURE.md` — Register-plus-cheap-tests artefact architecture, build-order rationale
- `.planning/research/PITFALLS.md` — CRIT-1 through CRIT-5 with prevention strategies; CRIT-5 is `RSK-pool-count`'s motivation

### External precedent (for register format and voice)
- [Leios `docs/ImpactAnalysis.md`](https://github.com/input-output-hk/ouroboros-leios/blob/main/docs/ImpactAnalysis.md) — `RSK-*`/`EXP-*` ID convention precedent
- [CIP-0164 §"Trade-offs & Limitations"](https://cips.cardano.org/cip/CIP-0164) — Closest Cardano disclosure-paragraph house-style precedent
- Wohlin et al. *Experimentation in Software Engineering* — Construct / internal / external / conclusion validity taxonomy

### CPS being responded to
- `docs/phase-2/CPS-0023/README.md` — Cardano Problem Statement 23, "Urgency Signaling"; the CIP whose evidence base this milestone is building

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

This is a documentation-only phase. No code is reused or produced. The relevant "assets" are the existing audit-trail documents themselves, all enumerated under canonical refs above.

### Established Patterns

- **Per-claim trust matrix pattern** (validity-threats.md): per-suite trust block with 4-level rating scale. Phase 1's register inherits the *threat-side* (RSK rows); Phase 2's coverage check inherits the *claim-side* (CLM rows). The split in D-01 follows this internal structure.
- **NEEDS-DISCLOSURE verdict pattern** (cardano-realism-audit.md): per-knob audit table with values, mainnet comparison, delta, and a verdict column. Phase 1's register inherits the *verdict* shape but pivots from per-knob to per-risk granularity.
- **Spike README pattern** (`.planning/spikes/*/README.md`): each spike has Research / Comparison-table / Findings / Verdict sections. The register treats these as opaque evidence sources; each spike's verdict is cited but not re-expressed.
- **Fix Status table pattern** (REVIEW.md): per-finding rows with status (RESOLVED / APPLIED / DEFERRED / OPEN). The register's verdict field maps onto this for code-quality risks pulled from REVIEW.md (e.g. WR-2, WR-7) but normalises to its own four-value LIVE/DORMANT/MITIGATED/DISCLOSED vocabulary.

### Integration Points

The register integrates with the Phase 2 coverage check via the `related-RSK-ids` column on `CLM-NN` rows. Each LIVE `RSK-NN` becomes an input to Phase 3 test design via the `scope-of-resolution` field. Each DISCLOSED `RSK-NN` becomes an input to Phase 4 refresh via its `disclosure-paragraph`. Each MITIGATED `RSK-NN` becomes a Phase 5 handoff item (CIP author cites the mitigation in the Evidence section).

The Phase-4-only `validity-threats.md` refresh (D-01) cannot begin until the register entries that supersede its per-threat content are stable.

</code_context>

<specifics>
## Specific Ideas

- **Mandatory four LIVE entries** (D-09) — the four risks already named in research must appear with their exact framings: `RSK-pool-count` / `RSK-single-seed-precision` / `RSK-un-anchored-controller-knobs` (or split into 4 sub-RSKs if the planner chooses; either is fine) / `RSK-substrate-scope`.
- **Locked scope-of-resolution text** for `RSK-pool-count` (D-10) — must appear verbatim or in semantically-equivalent form: "Δ% < seed-IQR of same job at 100 pools establishes MITIGATED". Locked-before-test discipline is the canonical example of PITFALLS MOD-5 prevention.
- **The register is CIP-facing**, not internal — disclosure paragraphs must read as CIP-pasteable prose (engineering-report voice, abbreviations expanded on first use, no internal jargon like "WR-1" without context). Internal-process documents (REVIEW.md, CONCERNS.md) can stay internal; the register cannot.

</specifics>

<deferred>
## Deferred Ideas

The user did not raise scope-creep ideas during this discussion. The three gray areas left unselected from the initial multiSelect (entry granularity, EXP-NN ↔ TEST-NN alignment, substrate-scope grouping) were resolved under Claude's Discretion with named defaults rather than being deferred — they're decisions for Phase 1 execution, just delegated rather than discussed.

No reviewed-todo deferrals (the `cross_reference_todos` step returned an empty matches set).

</deferred>

---

*Phase: 1-Register Inventory*
*Context gathered: 2026-05-15*
