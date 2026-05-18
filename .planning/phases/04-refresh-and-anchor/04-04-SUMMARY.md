---
phase: 04-refresh-and-anchor
plan: 04
subsystem: docs
tags: [docs, audit-refresh, calibration-provenance, disclosure-statements, dual-purpose]
requires:
  - .planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md (per-sub-knob anchor decisions + draft audit-section copy)
  - .planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md (headline numerical findings + TEST-07a regime-dependence)
  - docs/phase-2/realism-risks-register.md (RSK-NN load-bearing disclosure paragraphs cross-referenced from the audit)
provides:
  - docs/phase-2/cardano-realism-audit.md (refreshed in single authoritative voice; 500 lines; 12 (value, source, date-retrieved) triples; 20 RSK-NN cross-references; Phase 3 narrative integrated)
affects:
  - docs/phase-2/cardano-realism-audit.md
tech-stack:
  added: []
  patterns:
    - dual-purpose document pattern per CONTEXT.md D-39 (audit carries engineering-report-voice summaries; canonical CIP-pasteable prose lives in realism-risks-register.md per RSK-NN entries)
    - (value, source, date-retrieved YYYY-MM-DD) triple format applied uniformly to all calibration values
    - cross-reference to register's RSK-NN entries replaces in-audit duplication of CIP-pasteable disclosure paragraphs
key-files:
  created: []
  modified:
    - docs/phase-2/cardano-realism-audit.md
decisions:
  - "D-38 honored: stripped three historical annotation banners (2026-05-13, 2026-05-14, 2026-05-13 corrected); folded their content into authoritative prose"
  - "D-39 honored: §Recommended disclosure statements regenerated against Phase 3 evidence (un-reserved-outperform; RB-reserved-underperform; multiplier_floor regime-dependence; cross-arm duplicate-job artefact)"
  - "Claude's Discretion §Multiplier-floor regime-dependence narration honored: §Pricing-controller calibration disclosure item 2 LEADS with TEST-07a regime-dependence finding"
  - "Claude's Discretion §Audit narrative ordering honored: default four-section ordering preserved (Verdict by category / What lines up / What needs disclosure / Recommended disclosure statements)"
  - "Final line count compacted to exactly 500 lines (objective target 300-400; verification cap 500); deviation noted below"
metrics:
  duration: ~1.5h elapsed
  completed: 2026-05-18T13:21:00Z
  tasks: 3
  files_modified: 1
  commits: 3
---

# Phase 4 Plan 04: Cardano-realism audit refresh (DOC-01 + DOC-03 audit-side) Summary

Full rewrite of `docs/phase-2/cardano-realism-audit.md` per CONTEXT.md D-38 (single authoritative voice; banners stripped; calibration values as (value, source, date-retrieved) triples; substrate-scope paragraph included) and D-39 (Recommended disclosure statements regenerated against Phase 3 evidence with un-reserved-outperform / RB-reserved-underperform / multiplier_floor regime-dependence narrative).

## What changed

### Banner-stripping diff

The three historical annotation banners were stripped:

| Banner | Original location | Disposition |
|---|---|---|
| `> **[Annotation added 2026-05-13]**` (topology re-pointing) | Header, lines 9–21 of pre-rewrite | Removed; content folded into TL;DR and What-lines-up sections naming `topology-realistic-100.yaml` as the operational topology with epoch-582 mainnet snapshot triple |
| `> **Update 2026-05-14:**` (chain-derived Family B commit) | §Pricing-controller calibration, lines ~134-148 of pre-rewrite | Removed; content folded into TL;DR (Family B framing) + §Pricing-controller calibration preamble (controller advances exactly once per canonical block, reorg-safe by construction) |
| `> **[Corrected 2026-05-13]**` (single-producer → multi-producer disclosure) | §Topology and actor model, lines ~209-218 of pre-rewrite | Removed; entire §Topology and actor model rewritten for the 100-node multi-producer reality; single-producer disclosure items 1-2 replaced with 100-node topology / honest-producer items |

The "single-producer topology (N=1) vs mainnet ~3,000 SPOs is the strongest abstraction" framing was completely removed from §Topology and actor model and §Recommended disclosure statements §"On topology" — replaced with the operational `topology-realistic-100.yaml` framing.

### (value, source, date-retrieved YYYY-MM-DD) triple format

12 `date-retrieved YYYY-MM-DD` triples present in the refreshed audit, covering:

1. `rb-generation-probability = 0.05` (date-retrieved: 2026-05-14)
2. `rb-body-max-size-bytes = 90112` (date-retrieved: 2026-05-14)
3. `min-fee-a = 44` (date-retrieved: 2026-05-14)
4. `min-fee-b = 155381` (date-retrieved: 2026-05-14)
5. `maxTxSize = 16384` (date-retrieved: 2026-05-14)
6. `mempool-max-total-size-bytes = 24 MB` (date-retrieved: 2026-05-14)
7. `D = 8` (date-retrieved: 2026-05-13)
8. `target = 0.5` (date-retrieved: 2026-05-13)
9. Leios CIP-0164 Table 7 knobs (six values; date-retrieved: 2026-05-13)
10. `topology-realistic-100.yaml` (date-retrieved: 2026-05-14)
11. Window length 32 motivating-citation: Reijsbergen et al. AFT 2021 (date-retrieved: 2026-05-13)
12. Q1 2026 mainnet demand-mix order-of-magnitude estimate (date-retrieved: 2026-05-13)

The triple format is consistent across the audit: `(value, source: <citation>, date-retrieved: YYYY-MM-DD)`.

### Plan 04-01 per-sub-knob anchor narration

§"Pricing-controller calibration" was rewritten to fold Plan 04-01's per-sub-knob Draft audit-section copy:

| Sub-knob | Disposition | Narration source |
|---|---|---|
| Window length 32 (capacity-varying signals) | **ANCHORED** | Reijsbergen et al. AFT 2021 §"Short-term oscillation" + Leonardos et al. AFT 2021 (bounded-oscillation theoretical) + Liu et al. CCS 2022 (empirical counter-bound) |
| Multiplier-floor 4 (in `phase-2-rb-scarcity`, `phase-2-urgency-inversion`) | **DISCLOSED** | No external anchor for second-lane multiplier; CLAUDE.md §"Calibration choices" calibration accommodation; TEST-07a regime-dependence at floor 16 |
| Multiplier-floor 16 (spec default) | **DISCLOSED** | `mechanism-design.md` line 155 / 290 spec-internal "strong price-discrimination" rationale; no external anchor |
| Lane-signal-source choices (option 1 un-reserved; EB-bytes denominator both-dynamic) | **DISCLOSED** | Spec leaves choices open at `mechanism-design.md` lines 207–211 and 238 |

Umbrella verdict for `RSK-un-anchored-controller-knobs`: LIVE → DISCLOSED (only 1 of 4 sub-knobs anchors; per the register's `scope-of-resolution` rule, MITIGATED requires all four ANCHORED).

The Claude's Discretion §"Multiplier-floor regime-dependence narration (DOC-01 and register)" instruction was honored: §"Pricing-controller calibration" disclosure item 2 LEADS with the TEST-07a regime-dependence finding (rb-scarcity finding inverts at floor 16; urgency-inversion finding weakly reverses at floor 16). The opening sentence per the plan's mandate ("multiplier-floor 4 is a calibration accommodation chosen to surface controller drift at moderate priority demand. TEST-07a found that at multiplier-floor 16, the `phase-2-rb-scarcity` finding inverts ('standard dominates welfare' → 'priority captures everything; total welfare collapses 93–98%') and the `phase-2-urgency-inversion` finding weakly reverses ('mispriced > correctly priced' → 'correctly priced > mispriced by ~13%')") is present.

### Phase 3 evidence integration (Plan 04-03 → §Recommended disclosure statements)

New paragraph "On the menu-item welfare distinction" added per CONTEXT.md D-39, sourced verbatim-with-editorial-polish from Plan 04-03's §"Headline numerical findings":

- Un-reserved menu arms materially outperform single-lane EIP-1559: priority-only Δ rv = +6.66e+09 (95% BCa CI [+4.28e+09, +8.49e+09]); both-dynamic Δ = +7.95e+09 (CI [+5.65e+09, +1.09e+10]); sign-coherence 0.90 across 20 seeds
- RB-reserved menu arms underperform single-lane EIP-1559 under the same calibration: priority-only RB-reserved Δ = −4.15e+09 (CI [−6.02e+09, −1.00e+09]); both-dynamic RB-reserved (partitioned) Δ = −4.15e+09 (CI [−5.95e+09, −8.87e+08])
- Cross-arm duplicate-job artefact (partitioned ≡ RB-reserved welfare at sundaeswap_moderate × multiplier_floor = 4) replicates at N = 20 because the standard-lane controller never drifts off the multiplier floor
- Phase 3 hash-diversity gate: 17 of 17 BACKED-eligible cells pass at distinct-hash count = N

The "On controller calibration" paragraph in §Recommended disclosure statements also LEADS with the TEST-07a regime-dependence finding before naming the four sub-knob anchor verdicts and closing with the Family B core-parameter framing.

### Substrate-scope umbrella paragraph

Added at the close of §"Topology and actor model" per CONTEXT.md D-38 + ROADMAP.md success criterion #1, summarising the three substrate-scope sub-points (a) `f64` in non-pricing code paths; (b) propagation-model fidelity; (c) utility-maximising actor model. The summary is engineering-report-voice (dual-purpose with the canonical CIP-pasteable prose at `RSK-substrate-scope`).

### Recommended next steps refresh

Old items removed (out of scope or superseded):
- "M6 (already on this branch)" — superseded by TEST-05 disclose-only fallback per Plan 04-03
- "Optional: strategic-actor demand profile" — out of scope per PROJECT.md; covered by `RSK-substrate-scope` sub-point (c)
- "Optional: body-derivable `partition_activated`" — out of scope; covered by `RSK-partition-activated-honest-producer`
- "Optional: calibration-sensitivity suite for window length" — sub-knob 1 now ANCHORED via Plan 04-01

Items kept:
- "Hard dependency to flag in any publication" — refund-CIP dependency
- "Documentation residual" — CIP-0164 Table 7 cross-reference

Items added:
- Pool-count sensitivity disclose-only per `RSK-pool-count` with TEST-05 re-run recipe pointer
- Run-length steady-state disclose-only per `RSK-steady-state-run-length` with TEST-06 re-run recipe pointer

### Abbreviation-on-first-use audit

The header carries a single explicit expansion block listing 16 abbreviations (CIP, EIP-1559, RB, EB, SPO, BCa, CI, IQR, CCS, AFT, SODA, MEV, eUTxO, AIMD, UTC, EMA). Two additional abbreviations (NFT, DeFi) are expanded at first use in the body. All other abbreviations either resolve to header-block expansions or are already-expanded standard forms (MB, KB, YAML).

## Final document state

- **Line count:** 500 (objective target: 300-400 lines per CONTEXT.md D-38; verification cap: 500). The 500-line landing reflects a deliberate compaction trade-off: the must-haves are dense (12 triples + 4 sub-knob anchor decisions + 6 disclosure-statement paragraphs + 5 topology items + Phase 3 evidence prose + 20 RSK-NN cross-references). See the §Deviations section below for the line-count deviation note.
- **Section line counts:** TL;DR 48, Verdict by category 8, What lines up with mainnet 68, What needs disclosure 224 (Fee 35, Pricing-controller 82, Topology 80, Substrate-scope umbrella 15), What does NOT transfer 7, Recommended disclosure statements 126, Recommended next steps 22, Evidence 5.
- **12 (value, source, date-retrieved YYYY-MM-DD) triples present**.
- **20 RSK-NN cross-references present** (well above the >= 5 verification threshold).
- **Single authoritative voice throughout**: no historical annotation banners; no "as of today" / "operational" tense-fragile qualifiers without paired date-retrieved markers.
- **Dual-purpose pattern honored**: every disclosure paragraph in §Recommended disclosure statements cross-references its canonical CIP-pasteable counterpart in `realism-risks-register.md`'s per-`RSK-NN` entries.

## Cross-references for downstream plans

- **Plan 04-07 (Wave 3 consistency review)**: this SUMMARY documents the audit's final state; Plan 04-07's regex audit `[a-z_-]+(...)?:?\s*[0-9.]+\s*\(.*source:.*date-retrieved:.*\d{4}-\d{2}-\d{2}\)` should match all 12 triples in the audit. The 4-line / 500-vs-400 deviation flagged below is for Plan 04-07's reviewer attention.
- **Phase 5 (CIP-author handoff)**: §"Recommended disclosure statements" is the CIP paste-target. The six paragraphs (fee-field semantics / controller calibration / topology / demand modelling / mempool sizing / menu-item welfare distinction) are ready for paste-with-light-editing into the CIP's Limitations section.

## Deviations from CONTEXT.md / Plan 04-01 / Plan 04-03 guidance

### Deviation 1: Final line count 500 vs objective target 300-400

- **CONTEXT.md guidance:** D-38 objective text says "approximate length target: 410 → 300-400 lines"; the plan's `<verification>` block accepts up to 500 (`test $(wc -l < ...) -le 500`).
- **Final outcome:** The audit lands at exactly 500 lines, inside the verification bound but at the upper edge.
- **Reason for the 100-line overshoot vs the 300-400 objective:** The must-haves are dense: (a) 12 (value, source, date-retrieved) triples in §"What lines up with mainnet" and the Fee + Topology subsections, each triple inline in 2-4 lines of surrounding context per the plan's format-consistency requirement; (b) four sub-knob anchor decisions in §"Pricing-controller calibration", each carrying its own triple + 4-5 sentences of motivating-citation or conditional-on-X frame + register cross-reference per Plan 04-01's Draft audit-section copy; (c) six disclosure-statement paragraphs in §"Recommended disclosure statements" plus a new "On the menu-item welfare distinction" paragraph carrying four bullets per Plan 04-03's Headline numerical findings; (d) substrate-scope umbrella paragraph and 5 topology items per the refreshed Topology section; (e) 20 RSK-NN cross-references threaded through every disclosure section.
- **Compaction work done:** Multi-pass compaction trimmed the file from an initial 644 lines (after Tasks 1-3 first-pass writes) down to 500 by removing duplicate prose between the in-body sections and the §"Recommended disclosure statements" paragraphs, tightening triple-format wrapping, removing verbose "Defensible because" prose where the register cross-reference carries the load-bearing rationale, and dropping decorative parenthetical asides. Further compaction below 500 would require dropping must-have content (a triple, a sub-knob anchor decision, or a disclosure-statement paragraph), which would fail the verification.
- **Disposition:** ACCEPTED as a deliberate trade-off within the verification bound. Plan 04-07's consistency review may request further compaction if it surfaces specific bloat; the audit's structure is otherwise sound.

### Deviation 2: 2024–2026 arXiv follow-up pass not executed by Plan 04-01

Plan 04-01 explicitly recorded that the optional 2024–2026 arXiv follow-up pass was not executed because web-fetch tooling was unavailable in the executor environment. Plan 04-04 inherits this state: the per-sub-knob anchor narration in §"Pricing-controller calibration" reflects Plan 04-01's cut-off-recorded verdicts (1 ANCHORED via Reijsbergen / Leonardos / Liu / Azouvi; 3 DISCLOSED). Plan 04-07 may at its option run the follow-up pass and re-grade; the disposition recorded in this SUMMARY is robust to that re-grade in the direction of *more* anchors only (no anchor can flip back to disclose; only disclose can flip to anchor if a 2024–2026 paper surfaces).

- **Disposition:** ACCEPTED per Plan 04-01's recorded cut-off and D-37's open-ended-but-judgmental search-budget rule.

### Deviation 3: Out-of-scope items deferred per CONTEXT.md `<deferred>` were honored

- Replacing the audit's CIP-pasteable disclosure paragraphs with register disclosure-paragraph content verbatim — DEFERRED per CONTEXT.md `<deferred>`. The audit's §"Recommended disclosure statements" paragraphs retain their own engineering-report-voice prose; each cross-references its canonical CIP-pasteable counterpart in `realism-risks-register.md`'s `RSK-NN` entries. This is the dual-purpose document pattern per D-39.
- TEST-05 / TEST-06 re-runs — DEFERRED per CONTEXT.md `<deferred>`. The audit's §"Recommended next steps" carries the re-run recipe pointers but does not assert verdict improvements from non-executed re-runs.

## Self-Check: PASSED

**Files verified present:**

- `/home/will/git/arc-tiered-pricing/docs/phase-2/cardano-realism-audit.md` — exists, 500 lines, file-state matches the three task commits

**Commits verified present:**

- `da957ff` — `docs(04-04): Task 1 — strip 2026-05-13 banner...` — FOUND in git log
- `e3f7cca` — `docs(04-04): Task 2 — rewrite Pricing-controller calibration...` — FOUND in git log
- `0b1b904` — `docs(04-04): Task 3 — regenerate Recommended disclosure statements...` — FOUND in git log

**Automated verification checks (per the PLAN.md `<verification>` block):**

- Two annotation banners stripped — PASS
- `topology-realistic-100` named in TL;DR / Verdict / What-lines-up / Recommended-disclosure-statements — PASS
- At least 5 `(value, source, date-retrieved YYYY-MM-DD)` triples — PASS (12 present)
- Phase 3 headline findings (un-reserved outperform / RB-reserved underperform / multiplier_floor regime-dependence / cross-arm duplicate-job) in §Recommended disclosure statements — PASS
- §"Pricing-controller calibration" reflects Plan 04-01's per-sub-knob anchor decisions — PASS
- Substrate-scope paragraph included with cross-reference to `RSK-substrate-scope` — PASS
- At least 5 `RSK-NN` cross-references in §"Recommended disclosure statements" — PASS (20 across the whole doc; multiple in §Recommended disclosure statements)
- File lands between 300 and 500 lines — PASS (exactly 500)
- All abbreviations expand on first use — PASS (header expansion block + body expansions for NFT and DeFi)

**Plan-level TDD gate compliance:** Not applicable — Plan 04-04 type is `execute` (not `tdd`); no test infrastructure required for documentation refresh.
