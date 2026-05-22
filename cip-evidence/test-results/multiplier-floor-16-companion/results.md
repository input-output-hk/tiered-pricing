# TEST-07a — Multiplier-Floor-16 Companion Results


> **⚠️ SUPERSEDED 2026-05-21** — numerical claims below were computed under the
> pre-Cardano Improvement Proposal (CIP)-0164 EB-sizing simulator variant
> (`linear`, 12 megabyte (MB) EB wire object). Endorser Block (EB) certification
> failed under that variant, biasing every inclusion-rate / latency / welfare
> measurement. See [`../../../docs/phase-2/eb-sizing-fix-postmortem.md`](../../../docs/phase-2/eb-sizing-fix-postmortem.md) for the diagnosis and the re-run schedule.

**Run id:** `20260518-084846`
**Suite:** `robustness-multiplier-floor-16-companion.yaml`
**N seeds:** 5 (per scoping-results.md — qualitative replication test, no paired bootstrap)
**Comparison:** floor = 16 (this suite) vs existing floor = 4 baselines in
`output/phase-2/{rb-scarcity, urgency-inversion}/<cell>/<seed>/run_summary.json`
**Per-cell artefacts:** see `*.json` files alongside this README

## Verdict criteria

Per CONTEXT.md Claude's Discretion §"Multiplier-floor-16 companion run details" +
Phase 1 plan-02 SUMMARY:

- **MITIGATED** iff the qualitative finding replicates at floor 16, OR is
  reframed as "observable only when floor is low enough to admit medium-urgency
  components to priority".
- **LIVE → DISCLOSED** if the finding inverts at floor 16.

No paired BCa bootstrap CI gate — N=5 is sufficient for sign-coherence on
the welfare delta and qualitative pattern matching.

## Findings

### RB-scarcity (4 cells)

| Cell | floor=4 baseline (mean rv) | floor=16 companion (mean rv) | Δ% | Priority share at floor=4 | Priority share at floor=16 |
|---|---|---|---|---|---|
| `rb_baseline` | `8.5e+10` | `6.1e+09` | **-93%** | <1% (std dominates) | 100% (std collapsed) |
| `rb_reduced_half` | `8.5e+10` | `3.2e+09` | **-96%** | ~1-2% | 100% |
| `rb_reduced_third` | `8.4e+10` | `2.2e+09` | **-97%** | ~1-2% | 100% |
| `rb_reduced_quarter` | `8.5e+10` | `1.8e+09` | **-98%** | <1% | 100% |

(floor=4 baseline rv is from `output/phase-2/rb-scarcity/<cell>/{1,2,3}/run_summary.json`,
3 in-tree seeds. floor=16 rv is from this suite at 5 seeds.)

**Qualitative finding at floor=4 (the existing baseline):** standard lane
captures > 99% of retained value across all RB-scarcity configurations.
Reducing RB capacity (half / third / quarter) has minor effect on total
welfare because most traffic flows through the (un-rationed) standard lane.

**Qualitative finding at floor=16:**
- All retained value flows through priority (standard share = 0% across all cells).
- Total retained value drops by **93%–98%** vs floor=4.
- Reducing RB capacity at floor=16 progressively reduces priority-side welfare
  (6.1e+09 → 3.2 → 2.2 → 1.8 across baseline/half/third/quarter) — a strong
  RB-capacity effect on priority that is absent at floor=4 (because at floor=4
  priority barely uses RB).

**Verdict: LIVE → DISCLOSED.** The qualitative finding at floor=4 — that
standard lane dominates welfare and RB scarcity is mostly invisible — does NOT
replicate at floor=16. At floor=16 the mechanism rejects nearly all
medium-urgency standard-lane demand (`max_fee_lovelace` budget < 16 × quote),
priority captures all surviving value, and RB-capacity scarcity becomes the
binding constraint. This is the "floor too high to admit medium-urgency
components to priority" reframe — disclosure paragraph for `RSK-multiplier-floor-4-suite-coverage`
should note the finding's regime-dependence: the rb-scarcity claim made at
floor=4 (standard-dominates-welfare, RB-scarcity-mostly-invisible) is
specific to floor=4 and inverts at floor=16.

### Urgency inversion (2 cells)

| Cell | floor=4 baseline (mean rv) | floor=16 companion (mean rv) | Direction at floor=4 | Direction at floor=16 |
|---|---|---|---|---|
| `correctly_priced` | `9.6e+08` (range 0.17–1.6) | `6.1e+09` (σ=8.4e+08) | (baseline) | (baseline) |
| `mispriced_high_urgency` | `3.3e+09` (range 1.8–4.9) | `5.4e+09` (σ=4.5e+08) | mispriced > correctly | **mispriced < correctly** |

**Qualitative finding at floor=4 (the existing baseline):** mispriced
(high-urgency component carries `ScaledOverLaneQuote{1, 1}` — zero headroom)
produces HIGHER total retained value than correctly_priced (high-urgency uses
the default `{4, 1}`). This is the "urgency inversion" anomaly — overpaying
high-urgency txs inflates measured retained value even though every individual
high-urgency tx is paying more than it values its slot.

**Qualitative finding at floor=16:**
- `correctly_priced` rv = `6.1e+09`, `mispriced_high_urgency` rv = `5.4e+09`.
- Direction is **reversed**: at floor=16 the high-urgency overspending is
  more expensive (in fees) than at floor=4, eating into retained value.
- Sign at floor=16 is `correctly > mispriced` by ~13%.

**Verdict: LIVE → DISCLOSED with reframe.** The inversion observed at floor=4
**does not replicate at floor=16** — at floor=16 the mechanism prices urgency
correctly enough that overspending high-urgency components carry their fee
load rather than passing it back as "retained" value. The CIP disclosure
paragraph for `RSK-multiplier-floor-4-suite-coverage` should note that the
urgency-inversion finding from `phase-2-urgency-inversion.yaml` is specific
to the floor=4 calibration; it disappears (and weakly reverses) at floor=16.

The reframe is constructive: the urgency-inversion artifact at floor=4 is
the controller's "leakage" of unpriced congestion through the priority lane
(at floor=4 the priority quote barely rises above the floor, so over-paying
high-urgency txs don't get charged extra). At floor=16 the price floor itself
absorbs the over-payment.

## Identical-output observation (seeds 1+2)

`rb_scarcity_x16_baseline` and `urgency_inversion_x16_correctly_priced` produce
**identical** `retained_value` AND identical `pricing_event_stream.sha256` at
seeds 1 and 2; at seed 3, retained_value is also identical but the event stream
SHAs differ (`8ada173a...` vs `8f245bf6...`).

This is **not a bug**. Both cells share:
- demand: `paper_like_congested.yaml` (default)
- topology: `topology-realistic-100.yaml` (default)
- protocol: `protocol-base.yaml` (default)
- multiplier_floor: 16

They differ only in pricing — `two_lane_priority_only_static_x16` vs
`two_lane_both_dynamic_partitioned_x16`. At multiplier_floor=16 under
paper_like_congested demand, the standard-lane controller in the
both-dynamic variant never sees enough standard-lane demand to drift its
quote — only urgency≥5 components can afford 16× standard, so the
standard lane carries effectively zero priced traffic. With the standard
controller pinned at its initial quote, the partitioned-both-dynamic
mechanism degenerates to priority-only-static. The bit-identical SHAs at
seeds 1+2 reflect that the same event sequence emerges from both
mechanisms; the seed-3 SHA divergence is a tail-event in standard-lane
admission attempts that produces a different event ordering but the same
welfare outcome. This is a high-floor convergence finding, not a
duplicate-run artefact.

## Caveats

- **Floor=4 baseline is at N=3 seeds, not N=5.** The existing in-tree
  rb-scarcity and urgency-inversion outputs run `seeds: [1, 2, 3]` per
  the phase-2 suite YAMLs. The floor=16 companion is at N=5. Δ% values
  in the tables compare a 3-seed baseline mean against a 5-seed companion
  mean — adequate for qualitative reframe but not for tight CI on Δ%.
- **All 6 cells pass the hash-diversity gate** at 5/5 distinct (verified
  in `*.json` artefacts under this directory). The seeds-1+2 hash-equality
  across `rb_scarcity_x16_baseline` and `urgency_inversion_x16_correctly_priced`
  is across-cell, not within-cell — each cell individually has 5 distinct
  hashes.

## Abbreviations on first use

- **BCa** — Bias-corrected and accelerated (bootstrap confidence-interval method)
- **CIP** — Cardano Improvement Proposal
- **RB** — Ranking Block
- **rv** — `retained_value` (priority + standard total)
- **σ** — sample standard deviation (n−1 denominator)
