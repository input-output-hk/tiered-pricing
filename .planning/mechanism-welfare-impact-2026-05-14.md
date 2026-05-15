# Mechanism welfare-impact characterization — accumulator (2-step) vs chain-derived (1-step)

Date: 2026-05-14

Context: The bug-2 investigation
([.planning/chain-derived-bug2-investigation.md](chain-derived-bug2-investigation.md))
established that the accumulator implementation effectively performed
~2 controller steps per RB-EB pair (RB-publish sample + deferred
EB-validation sample), while chain-derived performs 1 step per
canonical block — the EIP-1559-faithful cadence. This report
quantifies the welfare-grade impact across the 33-job sundaeswap
smoke (seed=1, 2000 slots) to inform the Family A (preserve 2-step)
vs Family B (commit to 1-step) decision.

## TL;DR

- **4 of 33 jobs flip welfare sign** between A (accumulator) and B
  (chain-derived) — two single-lane EIP-1559 jobs at low-to-mid D
  (`d4_t50_w32`, `d8_t25_w32`) and two `*_x4_rb_quarter` jobs (an
  identical pair under `priority-only-rb-reserved` and
  `two-lane-partitioned`).
- **21 of 33 jobs are "meaningfully" different** (sign flip OR
  |Δ%|>25). The median magnitude shift among same-sign jobs is
  +35.4% with p75 = 67.2%. The single-lane arm absorbs the worst
  shift (median 72.8% with two flips); the un-reserved arms
  (priority-only and both-dynamic) are the most stable (median
  15-17%, no flips).
- **Reactivity (D) predicts shift magnitude**: D=4 flips, the D=8
  cohort has |Δ%| around 50-100% with one flip, D=16 holds sign with
  |Δ%|=43%. Hypothesis confirmed: more reactive controllers (lower
  D) diverge most between mechanisms because the second step per
  RB-EB pair amplifies in proportion to the per-step coefficient.
- **`baseline_flat_fee` is byte-identical** between A and B (hash,
  net_utility, inclusion count) — the comparison harness is sound
  and the flat-fee path is mechanism-independent as expected.

## Data sources

- Run A (accumulator, ~2-step/RB-EB pair, pre-refactor):
  `/home/will/git/arc-tiered-pricing/sim-rs/output/phase-2/smoke/sundaeswap-{singlelane,priority-only,both-dynamic}/`
  (33 (job, seed=1) pairs)
- Run B (chain-derived fixed, 1-step/block, post-bug#1 fix):
  `/home/will/git/arc-tiered-pricing/sim-rs/output/phase-2/smoke/sundaeswap-batch-20260514-134450/sundaeswap-{singlelane,priority-only,both-dynamic}/`
  (same 33 jobs)
- baseline_flat_fee byte-identical between A and B: **YES**
  (`pricing_event_stream.sha256` matches, `net_utility` matches,
  `total_txs_included` matches). Sanity check passes.

D values: `eip1559_d{4,8,16}_*` jobs encode D directly in the name.
All two-lane jobs use `max-change-denominator: 8` (verified in
`parameters/phase-2-sweep/pricing/two_lane_*.yaml`), so D=8 across
the board for the two-lane sweep.

### Cross-arm duplicate-job artefact (read first)

The priority-only-rb-reserved and two-lane-partitioned arms share
the same RB-reserved-priority controller dynamics under the
sundaeswap_moderate demand profile. In particular:

- `rb_reserved_x4` and `partitioned_x4` have identical welfare
  numbers in both A and B; same for the x16 jobs and the
  `_rb_{half,third,quarter}` variants. The pricing YAMLs differ
  only in the `variant: rb-reserved-priority-only` vs
  `rb-reserved-both-dynamic` discriminator, which only matters when
  the standard quote actually moves off the floor (it doesn't here
  under moderate sundaeswap demand).
- B's pricing event hashes are byte-identical between the matched
  pairs (e.g. `partitioned_x16` and `rb_reserved_x16` both produce
  `fe33...26d9`). A's hashes differ slightly between the matched
  pairs but produce identical welfare aggregates.

This means several "flip" / "meaningful change" entries below
appear twice (once per arm). When counting **distinct welfare
findings**, the rb_reserved/partitioned duplicates collapse: the
33-job sweep is closer to ~25 distinct welfare cells. The per-job
counts below preserve all 33 entries (since both arms are reported
in the publication) but Q5b / Q5c arm-aggregates already reflect
the collapse.

## Per-job table

| Job | Arm | D | A.net_utility | B.net_utility | Sign | \|Δ%\| | A_inc/B_inc | A_sb/B_sb |
|---|---|---|---|---|---|---|---|---|
| baseline_flat_fee | single-lane | n/a | +1.645e+10 | +1.645e+10 | same | 0% | 10971/10971 | 1/1 |
| eip1559_d16_t50_w32 | single-lane | 16 | +8.478e+09 | +1.216e+10 | same | 43% | 3834/10207 | 3/1 |
| eip1559_d4_t50_w32 | single-lane | 4 | +2.034e+10 | -2.321e+10 | **flip** | 214% | 3443/4774 | 0/0 |
| eip1559_d8_t25_w32 | single-lane | 8 | +4.486e+09 | -2.103e+08 | **flip** | 105% | 1306/7193 | 1/2 |
| eip1559_d8_t50_w16 | single-lane | 8 | +8.635e+09 | +2.155e+08 | same | 98% | 3620/7219 | 2/2 |
| eip1559_d8_t50_w32 | single-lane | 8 | +2.983e+09 | +1.200e+08 | same | 96% | 1328/7210 | 1/2 |
| eip1559_d8_t50_w64 | single-lane | 8 | +9.579e+09 | +1.134e+08 | same | 99% | 4239/7210 | 1/2 |
| eip1559_d8_t75_w32 | single-lane | 8 | +1.103e+10 | +1.650e+10 | same | 50% | 4656/11490 | 1/2 |
| rb_reserved_x16 | priority-only-rb-reserved | 8 | +1.169e+10 | +2.484e+10 | same | 112% | 1023/3677 | 1/1 |
| rb_reserved_x16_rb_half | priority-only-rb-reserved | 8 | +6.225e+09 | +8.427e+09 | same | 35% | 617/2163 | 2/1 |
| rb_reserved_x16_rb_quarter | priority-only-rb-reserved | 8 | +4.033e+09 | +4.562e+09 | same | 13% | 412/1179 | 2/1 |
| rb_reserved_x16_rb_third | priority-only-rb-reserved | 8 | +4.912e+09 | +5.966e+09 | same | 21% | 482/1514 | 2/1 |
| rb_reserved_x4 | priority-only-rb-reserved | 8 | +1.222e+10 | +1.899e+10 | same | 55% | 1485/4974 | 0/1 |
| rb_reserved_x4_rb_half | priority-only-rb-reserved | 8 | +7.425e+09 | +1.241e+10 | same | 67% | 836/2206 | 1/2 |
| rb_reserved_x4_rb_quarter | priority-only-rb-reserved | 8 | +4.613e+09 | -4.325e+09 | **flip** | 194% | 411/1183 | 2/1 |
| rb_reserved_x4_rb_third | priority-only-rb-reserved | 8 | +5.546e+09 | +3.833e+09 | same | 31% | 640/1630 | 1/1 |
| rb_reserved_x8 | priority-only-rb-reserved | 8 | +1.198e+10 | +2.239e+10 | same | 87% | 1344/3910 | 1/1 |
| rb_reserved_x8_rb_half | priority-only-rb-reserved | 8 | +6.288e+09 | +1.207e+10 | same | 92% | 813/2180 | 1/2 |
| rb_reserved_x8_rb_quarter | priority-only-rb-reserved | 8 | +3.770e+09 | +3.576e+09 | same | 5% | 460/1211 | 1/1 |
| rb_reserved_x8_rb_third | priority-only-rb-reserved | 8 | +6.400e+09 | +5.814e+09 | same | 9% | 619/1636 | 1/1 |
| unreserved_x16 (priority-only) | priority-only-unreserved | 8 | +2.673e+10 | +2.265e+10 | same | 15% | 9243/10347 | 1/1 |
| unreserved_x4 (priority-only) | priority-only-unreserved | 8 | +2.150e+10 | +2.516e+10 | same | 17% | 8607/10701 | 1/1 |
| unreserved_x8 (priority-only) | priority-only-unreserved | 8 | +2.353e+10 | +2.395e+10 | same | 2% | 8885/10561 | 1/1 |
| partitioned_x16 | two-lane-partitioned | 8 | +1.169e+10 | +2.484e+10 | same | 112% | 1023/3677 | 1/1 |
| partitioned_x16_rb_half | two-lane-partitioned | 8 | +6.225e+09 | +8.427e+09 | same | 35% | 617/2163 | 2/1 |
| partitioned_x16_rb_quarter | two-lane-partitioned | 8 | +4.033e+09 | +4.562e+09 | same | 13% | 412/1179 | 2/1 |
| partitioned_x16_rb_third | two-lane-partitioned | 8 | +4.912e+09 | +5.966e+09 | same | 21% | 482/1514 | 2/1 |
| partitioned_x4 | two-lane-partitioned | 8 | +1.222e+10 | +1.899e+10 | same | 55% | 1485/4974 | 0/1 |
| partitioned_x4_rb_half | two-lane-partitioned | 8 | +7.425e+09 | +1.241e+10 | same | 67% | 836/2206 | 1/2 |
| partitioned_x4_rb_quarter | two-lane-partitioned | 8 | +4.613e+09 | -4.325e+09 | **flip** | 194% | 411/1183 | 2/1 |
| partitioned_x4_rb_third | two-lane-partitioned | 8 | +5.546e+09 | +3.833e+09 | same | 31% | 640/1630 | 1/1 |
| unreserved_x16 (both-dynamic) | two-lane-both-dynamic-unreserved | 8 | +3.653e+10 | +3.331e+10 | same | 9% | 2798/10079 | 1/1 |
| unreserved_x4 (both-dynamic) | two-lane-both-dynamic-unreserved | 8 | +2.032e+10 | +2.467e+10 | same | 21% | 7207/10690 | 1/1 |

## Aggregate stats

### By sign

| Outcome | Count / 33 | Jobs |
|---|---|---|
| Welfare-sign flip (A pos → B neg) | 4 | `eip1559_d4_t50_w32`, `eip1559_d8_t25_w32`, `rb_reserved_x4_rb_quarter`, `partitioned_x4_rb_quarter` (last two are duplicate cells) |
| Same sign, >25% magnitude shift | 17 | see Q5 detail |
| Same sign, 5-25% magnitude shift | 8 | unreserved_x{4,8,16} × {priority-only, both-dynamic}, partitioned_x16_rb_quarter etc. |
| Same sign, ≤5% magnitude shift | 4 | `baseline_flat_fee` (0%), `unreserved_x8 (priority-only)` (2%), `rb_reserved_x8_rb_quarter` (5%), `rb_reserved_x8_rb_third` (9% — actually >5%, see exact table) |

Same-sign magnitude distribution (29 jobs, excludes the 4 flips):
median 35.4%, p25 15.2%, p75 67.2%, max 112.4%, min 0%.

### By reactivity D (single-lane EIP-1559 jobs only — 7 of 33)

| D | Job count | Sign flips | Median \|Δ%\| | Trend |
|---|---|---|---|---|
| 4 | 1 | 1 | 214% | Most reactive — flips |
| 8 | 5 | 1 | 97.5% | Mid-reactivity — high variance, one flip |
| 16 | 1 | 0 | 43% | Least reactive — sign holds, still large magnitude |

Hypothesis ("lower D ⇒ larger shift") confirmed empirically with
the caveat that D=8 dominates the singleton D=4 and D=16 samples.
The directional ordering (214% > 97.5% > 43%) is monotone in
reactivity. Sample sizes for D=4 and D=16 are 1 each — the trend
is suggestive, not statistically tight, but every EIP-1559 job at
D≤8 shows |Δ%|≥50%, and the only D=16 job sits at the bottom of
that range.

### By mechanism arm

| Arm | n | Sign flips | Median \|Δ%\| (same-sign) | Qualitative claim preserved? |
|---|---|---|---|---|
| single-lane | 8 | 2 | 72.8% (max 98.8%) | **No** — single-lane drops from rank 3 (A) to rank 5 (B); 2 of 7 EIP-1559 jobs flip negative |
| priority-only-rb-reserved | 12 | 1 | 35.4% (max 112.4%) | Yes — sign holds in 11/12, arm median moves +15% (6.26e+09 → 7.20e+09) |
| priority-only-unreserved | 3 | 0 | 15.2% (max 17.0%) | Yes — most stable arm; sign and magnitude both robust |
| two-lane-partitioned | 8 | 1 | 35.4% (max 112.4%) | Yes — mirrors priority-only-rb-reserved by construction under moderate demand |
| two-lane-both-dynamic (unreserved) | 2 | 0 | 15.1% (max 21.4%) | Yes — sign holds, magnitude stable |

### Arm ranking by median net_utility

A (accumulator) ranking, best to worst:
1. two-lane-both-dynamic (unreserved) — median +2.84e+10
2. priority-only-unreserved — median +2.35e+10
3. single-lane — median +9.11e+09
4. priority-only-rb-reserved — median +6.26e+09
5. two-lane-partitioned — median +5.89e+09

B (chain-derived) ranking, best to worst:
1. two-lane-both-dynamic (unreserved) — median +2.90e+10
2. priority-only-unreserved — median +2.39e+10
3. priority-only-rb-reserved — median +7.20e+09 (tied)
3. two-lane-partitioned — median +7.20e+09 (tied)
5. single-lane — median +1.68e+08 ← **collapses from rank 3 to rank 5**

The top two ranks (un-reserved arms) are preserved. The bottom of
the ranking changes: under A, single-lane sits above the RB-reserved
arms; under B, single-lane falls below them by two orders of
magnitude. This is the qualitative ranking change most consequential
to a publication.

## Family A vs B decision implications

### Family A (preserve 2-step / accumulator):

- Welfare numbers continue to reflect prior published findings.
- Single-lane EIP-1559 retains a credible welfare-positive narrative
  at low-to-mid D — jobs that flip to negative under B
  (`eip1559_d4_t50_w32`, `eip1559_d8_t25_w32`) stay positive.
- `*_x4_rb_quarter` jobs (priority-only-rb-reserved and matched
  partitioned) retain a welfare-positive result that flips under B.
- Reviewers will need to be told: "phase-2 implements a
  Cardano-specific 2-step-per-RB-EB-pair variant of EIP-1559
  controller cadence." The accumulator's update path emits the
  same controller step at both RB-publish and EB-validation
  moments, with the second step backed by the EB's `priced_bytes`
  rather than the RB's. This deviates from textbook EIP-1559 (1
  step per canonical block) and the deviation needs an
  explanation that satisfies a reviewer trained on the textbook
  model.
- Jobs that **require** the 2-step behavior to remain
  welfare-positive (i.e. flip under B):
  - `eip1559_d4_t50_w32` (A=+2.03e+10 → B=-2.32e+10)
  - `eip1559_d8_t25_w32` (A=+4.49e+09 → B=-2.10e+08)
  - `rb_reserved_x4_rb_quarter` / `partitioned_x4_rb_quarter`
    (A=+4.61e+09 → B=-4.32e+09)

### Family B (commit to 1-step / chain-derived):

- Welfare numbers shift: single-lane arm aggregate drops 73%
  (8.20e+10 → 2.21e+10) and falls to last in the ranking;
  RB-reserved arms gain ~30% (8.51e+10 → 1.19e+11); un-reserved
  arms are roughly stable.
- Jobs that flip welfare sign (now welfare-negative): same 4 listed
  above (or 3 distinct cells if you collapse the rb_reserved /
  partitioned duplicate).
- Jobs that preserve qualitative claim (sign + |Δ%|≤25%): 12 of 33.
  Jobs that preserve sign at any magnitude: 29 of 33.
- Strongest publication framing: "we adopt textbook EIP-1559
  controller cadence — one step per canonical block." This is a
  cleaner story for a reviewer who would otherwise ask "why two
  steps per RB-EB pair?" The trade is a sharper drop in single-lane
  welfare claims and a re-ordering of the mechanism ranking.
- Re-run cost: estimated 19 suites × N seeds × M jobs (whatever
  the full suite-level run is; the smoke is only 33 of 72 (job,
  seed) pairs).

## What "important metrics" actually mean here

Mapping back to the phase-2 publication's claim structure:

1. **"Dynamic pricing produces welfare > flat-fee baseline."** This
   claim is at risk under B for single-lane only.
   `baseline_flat_fee` net_utility = +1.645e+10. Under B, every
   EIP-1559 single-lane job except `eip1559_d8_t75_w32` (+1.65e+10)
   and `eip1559_d16_t50_w32` (+1.22e+10) is *worse* than baseline
   (some by orders of magnitude — `d8_t50_w{16,32,64}` all hover
   near zero). Under A, four of seven EIP-1559 single-lane jobs
   beat baseline. **The "single-lane EIP-1559 beats flat-fee"
   claim is fragile under B and survives under A.**

2. **"Two-lane mechanisms outperform single-lane."** Already true
   in both A and B at the un-reserved end. Under B it becomes
   *also* true at the RB-reserved end (RB-reserved jumps over
   single-lane in the ranking). Family B *strengthens* this claim.

3. **"RB-reservation provides welfare guarantees under capacity
   reduction."** The `*_rb_quarter` family is the smallest-capacity
   stress test. Under A, all `*_rb_quarter` jobs are welfare-positive
   (+3.77e+09 to +4.61e+09). Under B, the `x4_rb_quarter` jobs flip
   negative (–4.32e+09) — the most aggressive multiplier-floor (4)
   combined with the harshest capacity reduction produces a
   negative-welfare outcome under faithful EIP-1559 cadence. **The
   "RB-reservation holds under capacity stress" claim is weaker
   under B for the multiplier-floor-4 configuration.** The x8 and
   x16 floors at quarter-capacity remain positive under both A and
   B.

4. **"Un-reserved priority-only / both-dynamic deliver consistent
   welfare across the demand profile."** True under both A and B
   with median |Δ%| ≤ 17% per arm and no flips. **This claim is
   mechanism-robust.**

5. **"Reactivity (D) tunes welfare without breaking the
   mechanism."** Under A, low-D EIP-1559 (`d4`) was the
   second-highest single-lane welfare result. Under B, `d4` is the
   worst (–2.32e+10). The "reactivity sweet spot" story is
   different between the two mechanisms — under B, you want
   *less*-reactive controllers (D=8 high-target or D=16). Under A,
   more reactive was fine.

## Recommendation

The empirical pattern is that the **un-reserved arms are
mechanism-robust** (claims survive both A and B) and the
**single-lane EIP-1559 arm is mechanism-sensitive** (claim
direction depends on which cadence you publish). The
RB-reserved/partitioned arm sits in between (most jobs survive
both, but the harshest capacity-reduction × tightest multiplier
floor flips under B).

If the publication's primary contribution is the **two-lane
mechanism family**, Family B is the cleaner story to tell — it
preserves the headline claims, strengthens the "two-lane >
single-lane" ranking, and lets the spec section say "we adopt
textbook EIP-1559" without further qualification. The cost is that
4 of 33 cells flip negative and the single-lane EIP-1559 results
become substantially weaker than published.

If the publication's primary contribution is the **single-lane
EIP-1559 calibration** (the `D` × `target` × `window` sweep
suite), Family A is the result the prior data supports. Family B
will require either re-tuning the recommended (D, target) operating
point or accepting that the controller's welfare profile is
narrower than previously reported.

Data alone does not pick a winner — the choice depends on which
contribution the publication treats as headline and how much weight
reviewers will place on EIP-1559 cadence faithfulness. Both
positions are defensible.

The data does, however, **clearly indicate that doing nothing is
not a valid option**: presenting A's numbers without disclosing the
2-step cadence would mislead reviewers who assume textbook
EIP-1559, and presenting B's numbers without re-running suite-level
goldens (and updating the M5 hashes) would leave the repository in
a half-migrated state. Pick one and follow through.
