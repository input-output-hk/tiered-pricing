# Smoke comparison — chain-derived vs accumulator (sundaeswap × 3 arms, seed=1, same realistic topology)

Date: 2026-05-14
Context: Validating the WR-1 fix (spike 007 chain-derived refactor).
Comparing today's post-refactor smoke batch
(`sundaeswap-batch-20260514-130113`, run at 14:04) against today's
pre-refactor smoke (`sundaeswap-{both-dynamic,priority-only,
singlelane}/`, run at 12:45). Same topology
(`topology-realistic-100.yaml`, mainnet-derived), same demand
(`sundaeswap_moderate.yaml`), same protocol params, same seed=1,
same `default-slots: 2000`. The only changed input is the simulator
source (chain-derived refactor staged as uncommitted edits on
`sim-core/src/tx_pricing/{mod,single_lane,two_lane,window}.rs`,
`sim-core/src/sim/linear_leios.rs`, `sim-core/src/model.rs` — 5
controller-touching files, +1683/−839 LoC).

Sources:
- NEW: `sim-rs/output/phase-2/smoke/sundaeswap-batch-20260514-130113/sundaeswap-{both-dynamic,priority-only,singlelane}/<job>/<job>/1/run_summary.json`
- PRIOR: `sim-rs/output/phase-2/smoke/sundaeswap-{both-dynamic,priority-only,singlelane}/<job>/<job>/1/run_summary.json`

## TL;DR

**Verdict: REGRESSION — chain-derived diverges from accumulator on
zero-slot-battle jobs.** The refactor is not behaving as the spike
007 spec predicted ("trajectories should be very close to today's,
differences are exactly the orphan-block removals"). Three
load-bearing observations: (1) the **only** zero-slot-battle job in
both runs (`eip1559_d4_t50_w32`) shows a **welfare sign flip**
(+2.03e10 → −1.39e10) with a 10× drop in final `c_priority`
(130 553 → 12 412) — chain-derived was supposed to converge to the
accumulator value here, not flip the sign. (2) 32 of 33 jobs have
slot-battle counts ≥ 1 on both sides, so a clean "orphan-only"
attribution is unobservable, but 30/33 jobs show throughput up >5 %
(mean inclusions +114 %), 25/33 show `c_priority` up >5 %, and only
2/33 stay within ±5 % on inclusions. (3) `baseline_flat_fee` is
byte-identical (same hash, same inclusions, same welfare, same
controller endpoint), confirming the test harness and the
non-controller block plumbing are unchanged. The signal is squarely
in the controller. **WR-1 cannot yet be reclassified as resolved
from this evidence; first the d4 divergence needs to be diagnosed
because it tells us chain-derived is computing something different
even when WR-1 should be inactive.**

## Inventory

| Arm | New jobs | Prior jobs | Comparable pairs |
|---|---|---|---|
| both-dynamic | 10 | 10 | 10 |
| priority-only | 15 | 15 | 15 |
| singlelane | 8 | 8 | 8 |
| **Total** | **33** | **33** | **33** |

All 33 (job, seed=1) pairs present in both runs; no inventory
mismatches. Slot count = 2001 ticks (slot 0–2000) in both. Component
count = 11 (sundaeswap moderate profile) in both. JSON keys
identical between prior and new `run_summary.json`. The script's
nested layout (`<job>/<job>/1/` doubled-name pattern) is the same on
both sides.

**Pricing-event-stream hashes: 1/33 match.** The single match is
`sundaeswap-singlelane / baseline_flat_fee` (no controller activity
→ no pricing event diff possible). The other 32 differ — expected
under any controller refactor.

## Slot-battle activity (sanity check A — chain dynamics)

| Arm | Prior total | New total | Net |
|---|---|---|---|
| both-dynamic (10 jobs) | 13 | 11 | −2 |
| priority-only (15 jobs) | 18 | 17 | −1 |
| singlelane (8 jobs) | 10 | 11 | +1 |
| **All 33 jobs** | **41** | **39** | **−2** |

Aggregate counts are essentially unchanged (−2 of 41). Per-job
shifts are small (±1 each) and balanced. RB-production timing is
**not** systematically different between accumulator and
chain-derived — chain dynamics survived the refactor.

13 of 33 jobs have a per-job slot-battle delta of ±1; the remaining
20 are exactly equal. No job has |Δ|≥2. Consistent with the spike's
expectation that VRF lottery wins are unaffected.

**`orphaned_pricing_samples` tracks `slot_battles_count` exactly**
in both runs — i.e. one orphan sample per slot battle on both
sides. This is the right shape: every slot battle yields exactly
one orphan in the metrics. (Under chain-derived the field still
gets emitted because the simulator still walks the losing block's
samples for metric purposes; the canonical-chain controller no
longer mutates from them. The metric is now diagnostic-only, not a
contamination upper bound, but its presence is fine.)

Conclusion: Check A passes — same chain, same slot-battle profile.

## Controller convergence (sanity check B — the WR-1 fix's expected impact)

Final-tick (`slot=2000`) values of `c_priority` and `c_standard`
from `time_series.csv`. The two cases to look at separately are
zero-SB jobs (chain-derived must equal accumulator) and positive-SB
jobs (chain-derived must differ from accumulator by the orphan
contribution).

**Zero-slot-battle jobs (both sides) — must converge identically:**

| Job | Prior c_priority | New c_priority | Match? |
|---|---|---|---|
| `eip1559_d4_t50_w32` | 130 553 | 12 412 | **NO (−90.5 %)** |

There is exactly **one** zero-SB-both-sides job and it does **not**
match. This is the headline regression signal. The accumulator and
chain-derived implementations are not equivalent in the absence of
slot battles. The trajectory diverges from slot ~100 onward:

| slot | prior `c_priority` | new `c_priority` | ratio |
|---|---|---|---|
| 0 | 44 | 44 | 1.00 |
| 100 | 109 | 69 | 0.63 |
| 200 | 172 | 109 | 0.63 |
| 300 | 3196 | 521 | 0.16 |
| 500 | 19 826 | 3076 | 0.16 |
| 1000 | 12 720 | 250 741 | 19.7 |
| 1300 | 2425 | 2 968 217 | 1224 |
| 1700 | 21 874 | 219 538 | 10.0 |
| 2000 | 130 553 | 12 412 | 0.10 |

The two trajectories are completely uncorrelated by mid-run — peaks
at different slots, valleys at different slots. This is not a
deferred-by-one-tick offset; it is genuinely different controller
dynamics.

**Positive-slot-battle jobs (32 jobs) — expected to differ slightly
by the orphan contribution, but observed to differ wildly:**

Among 32 positive-SB jobs:
- `c_priority` increased >5 %: 24
- `c_priority` decreased >5 %: 7
- `c_priority` within ±5 %: 1 (`unreserved_x4` both-dynamic, +21 %
  — actually outside the band; only `baseline_flat_fee` truly
  matches, and it has c_priority=44 on both)

Largest swings (absolute magnitude):
- `eip1559_d8_t50_w64` (singlelane): 155 → 141 489 (+91 183 %)
- `eip1559_d8_t75_w32` (singlelane): 5382 → 0 (−100 %, runs against
  the era floor)
- `rb_reserved_x8_rb_third` (priority-only): 1877 → 54 028 (+2778 %)
- `rb_reserved_x16_rb_quarter` (priority-only): 1758 → 45 486 (+2487 %)
- `rb_reserved_x16_rb_half` (priority-only): 838 → 20 852 (+2388 %)
- `partitioned_x4_rb_third` (both-dynamic): 14 935 → 254 894 (+1607 %)
- `partitioned_x16_rb_third` (both-dynamic): 1559 → 26 457 (+1597 %)

These are **far larger** than what removing 1–2 orphan samples per
job (out of 2001 pricing ticks, i.e. ~10⁻³ of samples) could
plausibly produce. The expected per-job magnitude of WR-1
contamination removal is sub-percent; observed is 200–25 000 %.

Conclusion: Check B fails — chain-derived is producing materially
different controller state than accumulator, well beyond what the
WR-1 fix alone explains.

## Per-job welfare comparison

Columns:
- **SB(p→n)** = slot_battles_count, prior→new
- **Inc(p→n)** = total_txs_included, prior→new
- **NU prior / NU new** = Σ component net_utility_total (signed)
- **NU Δ%** = (new − prior) / |prior| × 100
- **c_prio p→n** = final-tick c_priority, prior→new

### Sundaeswap singlelane

| Job | SB(p→n) | Inc(p→n) | NU prior | NU new | NU Δ% | c_prio p→n | c_prio Δ% |
|---|---|---|---|---|---|---|---|
| baseline_flat_fee | 1→1 | 10971→10971 | 1.65e+10 | 1.65e+10 | +0.0% | 44→44 | +0.0% |
| eip1559_d16_t50_w32 | 3→1 | 3834→6183 | 8.48e+09 | 1.15e+10 | +36.1% | 24226→5365 | −77.9% |
| eip1559_d4_t50_w32 | 0→0 | 3443→3005 | 2.03e+10 | **−1.39e+10** | **−168.6%** | 130553→12412 | −90.5% |
| eip1559_d8_t25_w32 | 1→2 | 1306→4261 | 4.49e+09 | 8.03e+09 | +78.9% | 284967→234691 | −17.6% |
| eip1559_d8_t50_w16 | 2→2 | 3620→4279 | 8.63e+09 | 8.00e+09 | −7.3% | 5454→60106 | +1002.1% |
| eip1559_d8_t50_w32 | 1→2 | 1328→4275 | 2.98e+09 | 8.00e+09 | +168.2% | 18886→128114 | +578.4% |
| eip1559_d8_t50_w64 | 1→2 | 4239→4275 | 9.58e+09 | 8.00e+09 | −16.5% | 155→141489 | +91183.2% |
| eip1559_d8_t75_w32 | 1→1 | 4656→7266 | 1.10e+10 | 1.36e+10 | +23.7% | 5382→0 | −100.0% |

### Sundaeswap priority-only

| Job | SB(p→n) | Inc(p→n) | NU prior | NU new | NU Δ% | c_prio p→n | c_prio Δ% |
|---|---|---|---|---|---|---|---|
| rb_reserved_x16 | 1→1 | 1023→3576 | 1.17e+10 | 2.46e+10 | +110.8% | 704→2152 | +205.7% |
| rb_reserved_x16_rb_half | 2→1 | 617→1874 | 6.22e+09 | 9.04e+09 | +45.2% | 838→20852 | +2388.3% |
| rb_reserved_x16_rb_quarter | 2→1 | 412→972 | 4.03e+09 | 5.09e+09 | +26.3% | 1758→45486 | +2487.4% |
| rb_reserved_x16_rb_third | 2→1 | 482→1291 | 4.91e+09 | 6.43e+09 | +30.9% | 1559→26457 | +1597.0% |
| rb_reserved_x4 | 0→1 | 1485→3993 | 1.22e+10 | 1.83e+10 | +49.8% | 1300→12501 | +861.6% |
| rb_reserved_x4_rb_half | 1→2 | 836→1736 | 7.42e+09 | 1.16e+10 | +55.8% | 3126→10015 | +220.4% |
| rb_reserved_x4_rb_quarter | 2→1 | 411→827 | 4.61e+09 | **4.65e+08** | **−89.9%** | 296343→777858 | +162.5% |
| rb_reserved_x4_rb_third | 1→1 | 640→987 | 5.55e+09 | 6.22e+09 | +12.2% | 14935→254894 | +1606.7% |
| rb_reserved_x8 | 1→1 | 1344→3701 | 1.20e+10 | 2.19e+10 | +82.5% | 1509→854 | −43.4% |
| rb_reserved_x8_rb_half | 1→2 | 813→1937 | 6.29e+09 | 1.15e+10 | +82.9% | 16263→4675 | −71.3% |
| rb_reserved_x8_rb_quarter | 1→1 | 460→895 | 3.77e+09 | 5.46e+09 | +44.8% | 39473→154900 | +292.4% |
| rb_reserved_x8_rb_third | 1→1 | 619→1229 | 6.40e+09 | 7.20e+09 | +12.5% | 1877→54028 | +2778.4% |
| unreserved_x16 | 1→1 | 9243→9933 | 2.67e+10 | 2.27e+10 | −15.0% | 1169→8925 | +663.5% |
| unreserved_x4 | 1→1 | 8607→9230 | 2.15e+10 | 2.29e+10 | +6.6% | 1887→9305 | +393.1% |
| unreserved_x8 | 1→1 | 8885→9580 | 2.35e+10 | 2.34e+10 | −0.5% | 1559→7803 | +400.5% |

### Sundaeswap both-dynamic

| Job | SB(p→n) | Inc(p→n) | NU prior | NU new | NU Δ% | c_prio p→n | c_prio Δ% |
|---|---|---|---|---|---|---|---|
| partitioned_x16 | 1→1 | 1023→3576 | 1.17e+10 | 2.46e+10 | +110.8% | 704→2152 | +205.7% |
| partitioned_x16_rb_half | 2→1 | 617→1874 | 6.22e+09 | 9.04e+09 | +45.2% | 838→20852 | +2388.3% |
| partitioned_x16_rb_quarter | 2→1 | 412→972 | 4.03e+09 | 5.09e+09 | +26.3% | 1758→45486 | +2487.4% |
| partitioned_x16_rb_third | 2→1 | 482→1291 | 4.91e+09 | 6.43e+09 | +30.9% | 1559→26457 | +1597.0% |
| partitioned_x4 | 0→1 | 1485→3993 | 1.22e+10 | 1.83e+10 | +49.8% | 1300→12501 | +861.6% |
| partitioned_x4_rb_half | 1→2 | 836→1736 | 7.42e+09 | 1.16e+10 | +55.8% | 3126→10015 | +220.4% |
| partitioned_x4_rb_quarter | 2→1 | 411→827 | 4.61e+09 | **4.65e+08** | **−89.9%** | 296343→777858 | +162.5% |
| partitioned_x4_rb_third | 1→1 | 640→987 | 5.55e+09 | 6.22e+09 | +12.2% | 14935→254894 | +1606.7% |
| unreserved_x16 | 1→1 | 2798→10160 | 3.65e+10 | 3.23e+10 | −11.7% | 294272→15936 | −94.6% |
| unreserved_x4 | 1→1 | 7207→9265 | 2.03e+10 | 2.26e+10 | +11.4% | 7688→9312 | +21.1% |

Note the priority-only and both-dynamic arms share `_x16` / `_x4`
+ `_rb_*` rows by job-name parity at `multiplier_floor = 16` /
`floor = 4` — the same pattern observed in the prior smoke
comparison (sundaeswap demand saturates priority lane only, so
partitioned-both-dynamic ≡ rb-reserved-priority-only on
sundaeswap, and the chain-derived run preserves this equivalence
exactly).

## Welfare-sign flips

**One.** `sundaeswap-singlelane / eip1559_d4_t50_w32`:
- prior NU = **+2.034e+10** (positive)
- new NU = **−1.395e+10** (negative)
- prior SB = 0, new SB = 0 — **no slot battles on either side**

This is the most damning data point in the whole comparison. Under
the spike 007 spec, a zero-SB job must converge to identical
controller state, and net utility (a derived metric) should also
match. Observed delta is a magnitude flip, not a small drift.

Two additional jobs cross −80 % NU but stay positive:
- `partitioned_x4_rb_quarter` (both-dynamic) and the equivalent
  `rb_reserved_x4_rb_quarter` (priority-only) both drop from
  4.61e9 to 4.65e8 (−89.9 %). Both had SB=2 prior → SB=1 new,
  so this could partially be the WR-1 fix, but the magnitude is
  far larger than the orphan-removal hypothesis predicts (one
  orphan over a 2001-tick run cannot move welfare by 90 %).

## Structural vs stochastic

**Structural (consistent direction across multiple jobs in an arm):**

- **Inclusions broadly UP across both-tier mechanisms.** In
  both-dynamic and priority-only arms, every partitioned/rb_reserved
  job sees `total_txs_included` rise by 50–250 %. Mean inclusion
  delta across all 33 jobs = +114 %. The chain-derived refactor is
  systematically admitting more txs and/or charging less per tx in
  partitioned mechanisms. With 30/33 jobs up >5 %, only 1/33 down,
  this is not noise.
- **`max_priority_over_standard_ratio` collapses in `unreserved_x16`
  (both-dynamic) from 597 to ~15 936** (computed as c_prio/c_std at
  tip — note c_std also moved, from 18 392 to 996). The
  un-reserved-priority-only arm shows the inverse: ratios climb
  3–8 ×.
- **`c_priority` higher final value in 24/32 controllers** (mean
  +400 %+ on partitioned jobs at floor=4 or floor=16). The
  controllers are running hotter at chain tip under chain-derived,
  not cooler.

**Stochastic (one-off outliers):**

- `eip1559_d4_t50_w32`: NU flip with 0 SB, c_priority 10× lower
  and welfare hugely negative. Outlier in magnitude AND direction;
  cannot be a stochastic effect because the seed is fixed and the
  chain dynamics didn't change.
- `eip1559_d8_t50_w64`: c_priority went from 155 → 141 489
  (+91 183 %). The d8/w=64 sluggish-window setting was apparently
  pinned to a near-zero quote on accumulator and tracks differently
  under chain-derived. The trajectory walk required to interpret
  this is beyond a smoke comparison.
- `eip1559_d8_t75_w32`: c_priority went 5382 → 0, suggesting the
  controller hit the era-floor / zero-min under chain-derived but
  not accumulator.

## Validation verdict

**REGRESSION.** The empirical evidence does not match the spike 007
prediction. Specifically:

1. **Check A (chain dynamics unchanged) passes.** Slot-battle counts
   are ±2 in aggregate, ±1 per-job. The chain-derived refactor did
   not accidentally change RB production timing.
2. **Check B (controller convergence on zero-SB jobs) fails.** The
   only zero-SB-both-sides job (`eip1559_d4_t50_w32`) shows c_priority
   diverging 10×, welfare flipping sign. The spike spec said the
   math is unchanged in steady state; observation contradicts this.
3. **Welfare deltas exceed orphan-contribution bound.** With ~10⁻³
   of pricing samples being orphans, the spec-implied welfare delta
   per job is sub-percent. Observed median is several tens of
   percent; tails go to ±170 %.
4. **`baseline_flat_fee` matches byte-for-byte.** This rules out
   non-controller plumbing (slot lottery, propagation,
   distribution sampling, tx generation, event encoding) as the
   source. The shift is in the chain-derived controller computation
   itself.

**What's likely happening** (hypotheses for follow-up debug; not
asserted from this data alone):

a. **Quote-timing semantics differ.** Under accumulator, the
   producer at slot N admits/charges against the controller state
   *as it stood after slot N−1's apply* — i.e., the parent's
   post-update quote. Under chain-derived, the producer at slot N
   computes its **own** `derived_quote` as `f(parent_quote,
   parent_window ⊕ parent_samples)`, then admits/charges against
   the **new** quote (i.e. the block's own derived value). This is
   what the spike spec describes ("the values used for tx
   admissibility are the new block's own `derived_quote`") — but
   it is **not** "the same math in steady state": it shifts every
   block's update one block earlier in the pipeline. Over 2001
   ticks that compounds.

b. **Window aggregate semantics differ.** Accumulator's
   `CapacityWeightedWindow` was mutated in `apply_priced_block` —
   i.e., after a block's body was processed. Chain-derived's window
   aggregate at slot N is supposed to include samples from the last
   N canonical blocks. If the implementation off-by-one's the
   inclusion of the parent's own samples, the controller is reading
   a window that's one block stale or one block ahead.

Either of these would be enough to produce arbitrary divergence
even on zero-SB jobs. The d4_t50_w32 case is exactly the kind of
high-reactivity-controller setting where small phase shifts
compound into sign flips.

Recommendation: **before reclassifying WR-1 as resolved, add a
test that asserts chain-derived produces byte-identical
controller trajectories to accumulator on a zero-slot-battle
scenario**. The spike's proposed
`sibling_rbs_produce_identical_derived_quote` and
`slot_battle_does_not_contaminate_canonical_quote` tests are
necessary but not sufficient — the third test is "no-slot-battle
canonical chain produces same c_priority sequence" and the smoke
data says this currently fails.

## Implications for trust analysis

WR-1 cannot be empirically upgraded from "LIVE / disclosure-required"
to "RESOLVED" on this evidence. The structural argument from the
spike (chain-derived is statically immune to WR-1) is intact, but
the empirical evidence shows the implementation is doing something
*beyond* WR-1 resolution. Until the `eip1559_d4_t50_w32` zero-SB
divergence is diagnosed, MEDIUM-trust phase-2 claims that depended
on the accumulator's specific controller trajectory must remain
MEDIUM-trust, not because of WR-1 specifically, but because we
don't know which controller is "correct" between accumulator and
chain-derived. Both could be wrong; or chain-derived could be more
correct in a way the spike anticipated but did not quantify; or
the implementation might have a unit-bug (an off-by-one in window
inclusion, a doubled sample emission, an early/late update).

`docs/phase-2/validity-threats.md` row WR-1: hold at LIVE pending
diagnostic; add cross-reference to this smoke note.

`.planning/REVIEW.md` row WR-1: hold at LIVE / disclosure-required;
do not promote to RESOLVED.

The other phase-2 MEDIUM-trust claims that were planned to be
upgraded once WR-1 was empirically closed (the dependency was
spelled out in `.planning/spikes/007-chain-derived-controller/README.md`
table comparing publication framing) remain MEDIUM-trust.

## Caveats

- **Single seed.** All comparisons are seed=1. The
  `eip1559_d4_t50_w32` welfare flip is on a single trajectory; with
  seeds 2/3 the picture might soften (or get worse). Re-running both
  configurations across seeds 1–3 would tighten the
  structural-vs-stochastic call on the d4 outlier. Phase-2 suite
  goldens still hold seeds 1–3; the smoke was deliberately seed=1.
- **Sundaeswap demand only.** The actor profile is heavy on
  high-value-low-urgency components. The paper-like-* profiles
  could behave differently under chain-derived; this smoke does not
  test them.
- **Uncommitted simulator state.** The post-refactor smoke ran
  against `git status` showing `M sim-core/src/tx_pricing/{mod,
  single_lane,two_lane,window}.rs`, `M sim-core/src/sim/
  linear_leios.rs`, `M sim-core/src/model.rs` — i.e. the refactor
  is not yet a commit. If subsequent edits land before the diagnosis
  is done, this comparison may not be reproducible bit-for-bit; the
  binary the smoke used is not preserved.
- **No cross-arch verification.** All numbers are from the
  development machine (x86_64 / glibc). Cross-arch divergence is a
  separate, deferred concern unrelated to this smoke.
- **`orphaned_pricing_samples` is now diagnostic-only.** Under
  chain-derived the field is no longer a contamination upper bound
  for the canonical-chain controller (canonical-chain controller
  cannot be contaminated by construction). The metric still emits
  because the simulator still observes the losing block's samples,
  but its semantic shifts from "controller corruption upper bound"
  to "slot-battle occurrence proxy". Worth a docs update once the
  controller divergence is sorted.
- **The single-producer 3-arm sundaeswap suite was the original
  fast-screen; the M5 goldens have not yet been re-run.** All seven
  goldens under `parameters/phase-2-sweep/suites/.goldens/` will
  flip on the chain-derived refactor and should not be regenerated
  until the d4 divergence is understood, lest we bake in a
  potentially-buggy controller.
