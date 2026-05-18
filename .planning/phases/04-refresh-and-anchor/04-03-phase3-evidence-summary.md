# Phase 3 Evidence Summary — Consolidated for Wave 2

**Phase:** 04-refresh-and-anchor
**Plan:** 04-03
**Status:** phase-internal evidence consolidation for Wave 2 plans 04-04 / 04-05 / 04-06
**Date:** 2026-05-18

## Header

This artefact is the single consolidated extraction of Phase 3 evidence used by
Phase 4 Wave 2 plans:

- **Plan 04-04** (Cardano realism audit refresh — DOC-01) consumes the headline
  numerical findings, the TEST-03 / TEST-04 cell-level Bias-corrected and
  accelerated (BCa) 95% confidence intervals (CIs), and the TEST-07a
  multiplier-floor-16 regime-dependence narrative.
- **Plan 04-05** (validity-threats refresh — DOC-02) consumes the per-suite
  hash-diversity gate verdicts and the TEST-03 / TEST-04 cell verdicts that
  correspond to specific suites in the validity-threats per-suite trust matrix.
- **Plan 04-06** (realism-risks register edits) consumes the TEST-05 / TEST-06
  disclose-only fallback decision and the per-(realism-risk identifier (RSK))
  final-state recommendations.

Consolidating once here avoids three Wave-2 plans independently re-walking the
five Phase 3 results files at
`.planning/realism-tests/{multi-seed-variance, multiplier-floor-16-companion,
pool-number-sensitivity, run-length-steady-state, hash-diversity-gate}/results.md`.

**Abbreviations on first use** (per CLAUDE.md §"Conventions / gotchas"):

- **BCa** — Bias-corrected and accelerated (bootstrap confidence-interval method)
- **CIP** — Cardano Improvement Proposal
- **CI** — confidence interval
- **CLM** — claim identifier in `docs/phase-2/coverage-check.md`
- **EB** — endorser block
- **EIP-1559** — Ethereum Improvement Proposal 1559
- **IQR** — Inter-Quartile Range
- **PSE** — Paired Seed Evaluation
- **RB** — ranking block
- **RSK** — realism-risk identifier in `docs/phase-2/realism-risks-register.md`
- **SHA-256** — Secure Hash Algorithm 256-bit
- **rv** — `retained_value` (priority + standard total; the welfare scalar
  gating every Phase 3 verdict per CONTEXT.md D-24)

## TEST-03 sign-flip variance bands

Source: `.planning/realism-tests/multi-seed-variance/results.md`, Section
"TEST-03 — Sign-flip Variance Bands". Four (mechanism × demand × protocol)
cells where the 2026-05-14 Family A vs Family B welfare-impact
characterisation produced a sign-flip; each cell is re-run under committed
Family B at N=20 seeds and paired-bootstrapped against single-lane EIP-1559.

Verdict criteria (CONTEXT.md D-32): **BACKED** iff 95% CI excludes zero AND
hash-diversity gate passes; **WEAK** iff CI crosses zero; **re-run-needed**
iff hash-diversity gate fails.

| Cell | Verdict | BCa 95% CI on Δ rv | Median Δ | Sign-coherence | Distinct hashes |
|---|---|---|---|---|---|
| `cell_eip1559_d4_t50_w32` | BACKED | `[+3.38e+09, +1.35e+10]` | `+5.37e+09` | 0.75 | 20/20 PASS |
| `cell_eip1559_d8_t25_w32` | BACKED | `[+4.68e+08, +5.66e+09]` | `+7.81e+07` | 0.55 | 20/20 PASS |
| `cell_rb_reserved_x4_rb_quarter` | WEAK | `[-1.50e+09, +2.18e+09]` | `+1.50e+09` | 0.60 | 20/20 PASS |
| `cell_partitioned_x4_rb_quarter` | WEAK | `[-1.61e+09, +2.14e+09]` | `+1.50e+09` | 0.60 | 20/20 PASS |

Summary: both single-lane EIP-1559 sign-flip cells produce a statistically
significant positive welfare delta vs the (d8, t50, w32) baseline; the two
RB-quarter cells produce real-but-noisy positive medians whose CIs straddle
zero (variance-dominated at N=20). All four cells pass the COV-05
hash-diversity gate at 20/20 distinct pricing-event-stream SHA-256 values.

## TEST-04 canonical menu-item variance bands

Source: `.planning/realism-tests/multi-seed-variance/results.md`, Section
"TEST-04 — Canonical Menu-Item Variance Bands". Four canonical menu options
(one per CIP mechanism arm at `multiplier_floor = 4` under
`sundaeswap_moderate` demand) plus a single-lane EIP-1559 control. Five cells
total; the control (`control_eip1559_d8_t50_w32`) is listed in the
hash-diversity gate report at
`.planning/realism-tests/hash-diversity-gate/results.md` under the TEST-04
table and shares its event-stream artefact with the TEST-03 baseline
`control_eip1559_d8_t50_w32_base`.

| Cell | Verdict | BCa 95% CI on Δ rv | Median Δ | Sign-coherence | Distinct hashes |
|---|---|---|---|---|---|
| `menu_rb_reserved_priority_only_static_x4` | BACKED | `[-6.02e+09, -1.00e+09]` | `-4.15e+09` | 0.65 | 20/20 PASS |
| `menu_unreserved_priority_only_static_x4` | BACKED | `[+4.28e+09, +8.49e+09]` | `+6.66e+09` | 0.90 | 20/20 PASS |
| `menu_rb_reserved_both_dynamic_x4` | BACKED | `[-5.95e+09, -8.87e+08]` | `-4.15e+09` | 0.65 | 20/20 PASS |
| `menu_unreserved_both_dynamic_x4` | BACKED | `[+5.65e+09, +1.09e+10]` | `+7.95e+09` | 0.90 | 20/20 PASS |
| `control_eip1559_d8_t50_w32` (control) | n/a | (zero by construction; paired baseline) | n/a | n/a | 20/20 PASS |

### Headline finding (verbatim from Phase 3 SUMMARY §"Headline finding")

At `multiplier_floor = 4` under `sundaeswap_moderate` demand at N=20 seeds:

- **Un-reserved menu arms materially outperform single-lane EIP-1559**:
  priority-only un-reserved Δ rv = +6.66e+09 (95% BCa CI [+4.28e+09,
  +8.49e+09]); both-dynamic un-reserved Δ = +7.95e+09 (CI [+5.65e+09,
  +1.09e+10]). Sign-coherence 0.90 across 20 seeds.
- **RB-reserved menu arms underperform single-lane EIP-1559**: priority-only
  RB-reserved Δ = −4.15e+09 (CI [−6.02e+09, −1.00e+09]); both-dynamic
  RB-reserved (partitioned) Δ = −4.15e+09 (CI [−5.95e+09, −8.87e+08]). This
  REFUTES the Phase 1 / Phase 2 single-seed framing that "two-lane mechanisms
  outperform single-lane EIP-1559" — that framing holds only for the
  un-reserved variants under this calibration.
- The cross-arm duplicate-job artefact (partitioned ≡ rb-reserved welfare
  under `sundaeswap_moderate × floor=4`) replicates at N=20 because the
  standard quote never drifts off the multiplier floor.

## TEST-07a multiplier-floor-16 companion

Source: `.planning/realism-tests/multiplier-floor-16-companion/results.md`.
Six cells (4 rb-scarcity + 2 urgency-inversion) at `multiplier_floor = 16`
compared against the existing in-tree `multiplier_floor = 4` baselines at
N=3 seeds. Verdict criteria are qualitative replication-vs-inversion (no
paired BCa CI gate; N=5 is sufficient for sign-coherence).

### Per-cell floor=4 vs floor=16 comparison

| Cell | floor=4 mean rv | floor=16 mean rv | Δ% | Notes |
|---|---|---|---|---|
| `rb_baseline` | `8.5e+10` | `6.1e+09` | **−93%** | floor=4: <1% priority share; floor=16: 100% priority share |
| `rb_reduced_half` | `8.5e+10` | `3.2e+09` | **−96%** | same pattern |
| `rb_reduced_third` | `8.4e+10` | `2.2e+09` | **−97%** | same pattern |
| `rb_reduced_quarter` | `8.5e+10` | `1.8e+09` | **−98%** | same pattern |
| `urgency_correctly_priced` | `9.6e+08` (baseline) | `6.1e+09` (baseline) | n/a | baseline reference |
| `urgency_mispriced_high_urgency` | `3.3e+09` | `5.4e+09` | floor=4: mispriced > correctly; floor=16: **mispriced < correctly** by ~13% |

### Regime-dependence finding

Two qualitative findings landed LIVE → DISCLOSED with reframe (per the source
results.md verdict lines: "Verdict: LIVE → DISCLOSED" for rb-scarcity at
floor=16; "Verdict: LIVE → DISCLOSED with reframe" for urgency-inversion at
floor=16):

(a) **The rb-scarcity finding at `multiplier_floor = 4`** — "standard lane
dominates welfare across all RB-scarcity configurations; reducing RB capacity
(half / third / quarter) has minor effect on total welfare because most
traffic flows through the un-rationed standard lane" — does NOT replicate at
floor=16. At floor=16, the mechanism rejects nearly all medium-urgency
standard-lane demand (`max_fee_lovelace` budget below `16 × quote`); priority
captures all surviving value; total retained value drops by 93%–98% vs
floor=4; and RB-capacity scarcity becomes the binding constraint
(progressive reduction in priority-side welfare across baseline / half /
third / quarter, which is invisible at floor=4 because priority barely uses
RB).

(b) **The urgency-inversion finding at `multiplier_floor = 4`** — "mispriced
(high-urgency `ScaledOverLaneQuote{1, 1}`) > correctly priced (high-urgency
`{4, 1}`); overpaying high-urgency txs inflates measured retained value
because the priority quote barely rises above the floor and over-payments
don't get charged extra" — WEAKLY REVERSES at floor=16. At floor=16, the
high-urgency over-spending is more expensive in fees than at floor=4 (the
price floor itself absorbs the over-payment), so the direction flips to
correctly priced > mispriced by ~13%.

### Cross-cell SHA-256 identity at seeds 1+2 (the high-floor duplicate-job artefact)

The cells `rb_scarcity_x16_baseline` and `urgency_inversion_x16_correctly_priced`
produce **identical** `retained_value` AND identical
`pricing_event_stream.sha256` at seeds 1 and 2 (`749ecfe6c0e3dec0...` and
`4c36fdc8200c79c9...`). At seed 3, `retained_value` is also identical but the
event stream SHA-256 hashes differ (`8ada173a...` vs `8f245bf6...`).

Both cells share `paper_like_congested.yaml` demand, `topology-realistic-100.yaml`,
`protocol-base.yaml`, and `multiplier_floor = 16`; they differ only in
pricing (`two_lane_priority_only_static_x16` vs
`two_lane_both_dynamic_partitioned_x16`). At `multiplier_floor = 16` under
`paper_like_congested` demand, the standard-lane controller in the
both-dynamic variant never sees enough standard-lane demand to drift its
quote (only urgency≥5 components can afford `16 × standard`, so standard
carries effectively zero priced traffic). With the standard controller
pinned at its initial quote, partitioned-both-dynamic degenerates to
priority-only-static.

This is **distinct from but mechanistically related to** the floor=4
cross-arm duplicate-job artefact that Plan 04-04 also narrates (the
TEST-04 finding that `menu_rb_reserved_both_dynamic_x4` ≡
`menu_rb_reserved_priority_only_static_x4` welfare). Both artefacts share
the same underlying mechanism: **the standard-lane controller is pinned at
the multiplier floor under congested demand, so partitioned-both-dynamic
degenerates to priority-only-static**. At floor=4 the pinning is caused by
RB-reserved partition limiting standard-lane traffic; at floor=16 the
pinning is caused by `max_fee_lovelace` budget rejecting medium-urgency
demand at the high floor.

Within-cell hash-diversity at TEST-07a is 5/5 distinct per cell (all six
cells pass at N=5); the cross-cell identity is an across-cell observation
that does not violate the within-cell COV-05 gate.

## TEST-05 / TEST-06 disclosure-fallback decision

### Source artefact status

- `.planning/realism-tests/pool-number-sensitivity/results.md` carries
  status: **"DATA-GAP (insufficient coverage; defer to Phase 4
  disclosure)"** with 35 / 1650 runs (≈2.1%) completed at the over-scoped
  batch id; the cut TEST-05 suite (165 runs) was not re-launched before
  the run was stopped. Coverage table: 35 completed, 1607 pending, 8
  running (interrupted). The re-run recipe in the file is
  `scripts/run-phase-3-suites.sh 1
  parameters/phase-2-sweep/suites/phase-3-pool-number-sensitivity.yaml`
  (~50 min wall-clock at `-P 8`).
- `.planning/realism-tests/run-length-steady-state/results.md` carries
  status: **"PARTIAL (only 1 of 4 menu arms has data)"** with 31 / 120
  runs (≈26%) completed; only the `rb_reserved_priority_only` arm has
  complete data at 2000 / 4000 slots. The re-run recipe in the file is
  `scripts/run-phase-3-suites.sh 1
  parameters/phase-2-sweep/suites/phase-3-run-length.yaml` (~56 min
  wall-clock at `-P 8`).

### Per-CONTEXT.md disposition

Per 04-CONTEXT.md `<deferred>` "Re-running TEST-05 / TEST-06 inside Phase 4":
re-runs are user-managed and **out of scope for Phase 4 execution**. The
fallback is **disclose-only**.

Per 04-CONTEXT.md D-34 the verdict rule is: MITIGATED iff data lands inside
thresholds (Δ% < seed-IQR for TEST-05; per-half rolling-mean difference
inside seed-IQR for TEST-06); otherwise **LIVE → DISCLOSED** via the Phase 1
plan-02 disclosure-paragraphs.

With re-run-not-in-scope-for-Phase-4 the deterministic Phase 4 outcome for
the three affected register entries is:

- **`RSK-pool-count`** → **LIVE → DISCLOSED** (TEST-05 data-gap →
  disclose-only fallback per CONTEXT.md `<deferred>`). The existing draft
  fallback `disclosure-paragraph` in the register is **load-bearing**; no
  rewrite is required. Plan 04-06's action is verdict-flip-only (LIVE →
  DISCLOSED).
- **`RSK-calibration-stale-stake-snapshot`** → **LIVE → DISCLOSED** (same
  disposition; the register's `scope-of-resolution` field for this entry
  explicitly cross-references the same TEST-05 dependency as
  `RSK-pool-count`, so the two flip together under the same disclose-only
  fallback). The existing draft fallback `disclosure-paragraph` is
  load-bearing; no rewrite required. Plan 04-06's action is
  verdict-flip-only.
- **`RSK-steady-state-run-length`** → **LIVE → DISCLOSED** (TEST-06
  data-gap → disclose-only fallback). The register's current
  `disclosure-paragraph` field for this entry reads "TBD — drafted in
  Phase 4 if test verdict lands as DISCLOSED" per the Phase 1 plan-02
  two-track convention, so Plan 04-06 **must draft new prose** for the
  disclosure-paragraph (cannot rely on an existing draft fallback).

## Register entries Phase 4 touches

Five RSK entries are within Phase 4's editing surface. Per-entry final-state
recommendation:

1. **`RSK-pool-count`**
   - Final verdict: **DISCLOSED** (was LIVE)
   - Disclosure-paragraph action: **no rewrite**; existing draft fallback is
     load-bearing
   - Plan 04-06 action: **verdict-flip-only**
   - Trigger: TEST-05 data-gap → disclose-only fallback per CONTEXT.md
     `<deferred>`

2. **`RSK-calibration-stale-stake-snapshot`**
   - Final verdict: **DISCLOSED** (was LIVE)
   - Disclosure-paragraph action: **no rewrite**; existing draft fallback is
     load-bearing
   - Plan 04-06 action: **verdict-flip-only**
   - Trigger: overlap with TEST-05 disposition per the register's
     `scope-of-resolution` cross-reference

3. **`RSK-steady-state-run-length`**
   - Final verdict: **DISCLOSED** (was LIVE)
   - Disclosure-paragraph action: **Plan 04-06 draft required**; the
     register's current state for this entry is "TBD — drafted in Phase 4
     if test verdict lands as DISCLOSED"
   - Plan 04-06 action: **verdict-flip + draft new disclosure-paragraph**
   - Trigger: TEST-06 data-gap → disclose-only fallback

4. **`RSK-un-anchored-controller-knobs`**
   - Final verdict: **gated on Plan 04-01 outcomes** — MITIGATED iff all
     four sub-knobs (window-length-32, multiplier-floor-4,
     multiplier-floor-16, lane-signal-source) land ANCHORED; otherwise
     **LIVE → DISCLOSED with per-sub-knob granularity** per CONTEXT.md D-36
   - Disclosure-paragraph action: **Plan 04-06 rewrite required**; the
     rewrite is per Plan 04-01's draft register prose blocks (each sub-knob
     gets refined per-knob prose for ANCHORED-vs-DISCLOSED state)
   - Plan 04-06 action: **rewrite required; gated on Plan 04-01**
   - Trigger: Plan 04-01 literature-search outcomes

5. **`RSK-multiplier-floor-4-suite-coverage`**
   - Final verdict: **DISCLOSED** (was LIVE) — the register's draft fallback
     anticipated MITIGATED via TEST-07a, but TEST-07a's verdict landed LIVE
     → DISCLOSED with reframe (regime-dependence on floor=4-vs-floor=16),
     so the entry flips to DISCLOSED rather than MITIGATED
   - Disclosure-paragraph action: **Plan 04-06 rewrite required**; the
     rewrite must cite the regime-dependence finding from TEST-07a
     (rb-scarcity inversion + urgency-inversion weak reversal at floor=16)
     rather than the originally-expected qualitative-replication framing
   - Plan 04-06 action: **rewrite required; reframe per TEST-07a**
   - Trigger: TEST-07a results landing as LIVE → DISCLOSED with reframe
     rather than MITIGATED

## Headline numerical findings for Plan 04-04 (audit refresh)

CIP-pasteable form ready for the refreshed audit's "Recommended disclosure
statements" section (per CONTEXT.md D-39 dual-purpose regeneration and D-42
methodology-overview cross-references):

- **Un-reserved menu arms materially outperform single-lane EIP-1559 at
  `multiplier_floor = 4` under `sundaeswap_moderate` demand at N=20
  seeds**: priority-only un-reserved Δ retained_value = +6.66e+09 (95%
  BCa CI [+4.28e+09, +8.49e+09]); both-dynamic un-reserved Δ = +7.95e+09
  (CI [+5.65e+09, +1.09e+10]). Sign-coherence 0.90 across 20 seeds.
  (Source: TEST-04, `.planning/realism-tests/multi-seed-variance/results.md`.)

- **RB-reserved menu arms underperform single-lane EIP-1559 under the same
  calibration**: priority-only RB-reserved Δ = −4.15e+09 (95% BCa CI
  [−6.02e+09, −1.00e+09]); both-dynamic RB-reserved (partitioned) Δ =
  −4.15e+09 (CI [−5.95e+09, −8.87e+08]). Sign-coherence 0.65 across 20
  seeds. This REFUTES the pre-Phase-3 single-seed framing that "two-lane
  mechanisms outperform single-lane EIP-1559" — that framing holds only
  for the un-reserved variants under this calibration.
  (Source: TEST-04.)

- **The cross-arm duplicate-job artefact (partitioned ≡ RB-reserved welfare
  under `sundaeswap_moderate × floor=4`) replicates at N=20 seeds** because
  the standard-lane controller never drifts off the multiplier floor at
  `multiplier_floor = 4` under this demand profile (priority-only-static
  and partitioned-both-dynamic share identical median Δ and overlapping
  CIs). The same mechanism — standard controller pinned at the floor →
  partitioned-both-dynamic collapses to priority-only-static — explains the
  cross-cell SHA-256 identity observed at floor=16 between
  `rb_scarcity_x16_baseline` and `urgency_inversion_x16_correctly_priced`
  in TEST-07a.
  (Source: TEST-04 cross-cell pattern + TEST-07a cross-cell SHA-256
  observation.)

- **The `multiplier_floor = 4` calibration is regime-dependent**: at
  `multiplier_floor = 16` (TEST-07a) the rb-scarcity finding inverts
  ("standard dominates welfare; RB scarcity mostly invisible" → "priority
  captures everything; total welfare collapses 93–98%") and the
  urgency-inversion finding weakly reverses ("mispriced > correctly priced"
  → "correctly priced > mispriced by ~13%"). The CIP's Limitations section
  should report the multiplier_floor regime-dependence explicitly: the
  un-reserved-outperform finding is conditional on the floor=4 calibration
  used in the `phase-2-rb-scarcity` and `phase-2-urgency-inversion`
  suites.
  (Source: TEST-07a,
  `.planning/realism-tests/multiplier-floor-16-companion/results.md`.)

- **Hash-diversity gate: 17 of 17 BACKED-eligible cells pass at distinct
  count = N seeds**. No cells downgraded to WEAK from gate failure; no
  cells marked re-run-needed. The cross-cell SHA-256 identity in TEST-07a
  (at floor=16) is across-cell and does not violate the within-cell
  COV-05 gate.
  (Source: COV-05 / `.planning/realism-tests/hash-diversity-gate/results.md`.)

## Cross-references for Wave 2 plans

| Sub-section | Consumed by |
|---|---|
| TEST-03 sign-flip variance bands (§"TEST-03 sign-flip variance bands") | Plan 04-04 (audit refresh: per-cell BCa CI in disclosure prose); Plan 04-05 (validity-threats per-suite refresh: the four TEST-03 cells correspond to suites `phase-2-eip1559-robustness.yaml` and `phase-2-rb-scarcity.yaml`) |
| TEST-04 canonical menu-item variance bands (§"TEST-04 canonical menu-item variance bands") | Plan 04-04 (audit §"Recommended disclosure statements" un-reserved-outperform / RB-reserved-underperform paragraphs); Plan 04-05 (validity-threats per-suite refresh: the five TEST-04 cells correspond to suites `phase-2-priority-only-rb-reserved.yaml`, `phase-2-priority-only-unreserved.yaml`, `phase-2-two-lane-both-dynamic.yaml`) |
| TEST-07a multiplier-floor-16 + regime-dependence (§"TEST-07a multiplier-floor-16 companion") | Plan 04-04 (audit §"Pricing-controller calibration" disclosure item 2 rewrite per CONTEXT.md Claude's-Discretion item "Multiplier-floor regime-dependence narration"); Plan 04-06 (`RSK-multiplier-floor-4-suite-coverage` disclosure-paragraph rewrite) |
| TEST-05 / TEST-06 disclosure-fallback decision (§"TEST-05 / TEST-06 disclosure-fallback decision") | Plan 04-06 (`RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, `RSK-steady-state-run-length` verdict flips and `RSK-steady-state-run-length` disclosure-paragraph draft) |
| Register entries Phase 4 touches (§"Register entries Phase 4 touches") | Plan 04-06 (full register edit checklist — 2 verdict-flip-only + 1 verdict-flip-plus-draft + 1 rewrite-gated-on-Plan-04-01 + 1 rewrite-reframe) |
| Headline numerical findings for Plan 04-04 (§"Headline numerical findings for Plan 04-04 (audit refresh)") | Plan 04-04 (audit §"Recommended disclosure statements" regeneration per CONTEXT.md D-39 and D-42); Plan 04-07 (consistency-review cross-reference integrity) |
| Hash-diversity gate 17/17 pass (within §"Headline numerical findings" and §"TEST-07a cross-cell SHA-256 identity") | Plan 04-05 (validity-threats per-suite trust verdicts: the 17/17 result licenses BACKED rows in the per-suite trust matrix that pass the COV-05 gate) |
