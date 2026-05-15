# Smoke comparison — realistic-100 curve vs uniform-100 (sundaeswap × 3 arms, seed=1)

Date: 2026-05-14
Context: First post-topology-curve smoke. Comparing today's
`run-smoke-batch.sh` output (12:45) against today's pre-curve 100n-full
run (08:16). Same simulator commit; only per-node `stake` values
differ. New topology is `topology-realistic-100.yaml` (mainnet-derived,
top-1 = 1.97 %, Nakamoto = 35); prior is `topology.default.yaml`
(uniform stake = 100, Nakamoto = 50).

Sources:
- NEW: `sim-rs/output/phase-2/smoke/sundaeswap-{both-dynamic,priority-only,singlelane}/<job>/<job>/1/run_summary.json`
- PRIOR (seed=1 only): `sim-rs/output/phase-2/sundaeswap-{both-dynamic,priority-only,singlelane}-20260513-081627-100n-full/<job>/1/run_summary.json`

## TL;DR

The realistic stake curve produces **structural** shifts, not just
noise. (1) **Slot battles went UP, not down**: 1 across all 33 jobs
prior → 41 across the same jobs after the curve. The hypothesis that
skewed stake reduces slot battles is FALSIFIED by this smoke; the
mechanism is the opposite — concentrated stake at a few producers
makes their bodies more likely to be the contested ones that
representative-side validate (the WR-1 narrowing condition).
(2) **All 33 pricing-event-stream hashes flip** — every pair of (job,
seed=1) is bit-different. (3) **No welfare-sign flips** in any
direction: every (prior, new) net-utility pair stays positive. (4)
**Pricing-arm rank order is preserved** in qualitative terms
(unreserved still highest-throughput, partitioned still
priority-saturated, singlelane baseline still beats most EIP-1559
tunings on net utility), but magnitudes shift heavily — e.g.
`partitioned_x4` throughput −30 %, `unreserved_x16` (both-dynamic)
fees paid +621 % with txs included −58 %. Phase-2 conclusions
survive directionally but **quantitative deltas in the prior
analysis must not be cited without re-running with the realistic
curve**.

## Inventory

Job-set intersection is **complete** in all three arms:
- both-dynamic: 10 jobs in both (4 partitioned_x16 × {full, rb_half,
  rb_quarter, rb_third}, 4 partitioned_x4 × same, 2 unreserved × {x4,
  x16}). No misses either side.
- priority-only: 15 jobs in both (4 rb_reserved_x{4,8,16} × {full,
  rb_half, rb_quarter, rb_third} = 12 + 3 unreserved × {x4, x8,
  x16}). No misses either side.
- singlelane: 8 jobs in both (`baseline_flat_fee` + 7 EIP-1559
  variants). No misses either side.

Total comparable (job, seed=1) pairs: **33**.

Note: the prior run included seeds 2 and 3 as well; this comparison
restricts to seed=1 in the prior to match the smoke, as specified.

## Per-job comparison

Columns:
- **Sub** = `total_txs_submitted`
- **Inc** = `total_txs_included`
- **Fee Δ** = relative change in `total_fees_paid_lovelace`
- **NU Δ** = relative change in aggregate `net_utility_total`
  (Σ over components) — preserves sign
- **Bytes Δ** = relative change in Σ component `bytes_included`
- **RV-ratio Δ** = relative change in `retained_value_total /
  included_value_lovelace_total` (welfare efficiency)
- **SB(new)** = `slot_battles_count` in the new run
- **Notes** — flags substantial shifts (>±25 %) or qualitative
  changes

All deltas are (new − prior) / prior. Numbers in the table are
rounded to one decimal; sub/inc are absolute new-vs-prior. Hashes
were SHA-256 different in every row.

### Sundaeswap both-dynamic

| Job | Sub (p→n) | Inc (p→n) | Fee Δ | NU Δ | Bytes Δ | RV-ratio Δ | SB(new) | Notes |
|---|---|---|---|---|---|---|---|---|
| partitioned_x16 | 44539→44229 | 1344→1023 | −53.6 % | −47.3 % | −31.2 % | −1.5 % | 1 | Throughput collapses; priority lane carried less load |
| partitioned_x16_rb_half | 44542→44229 | 686→617 | −45.4 % | −30.6 % | −16.4 % | −2.1 % | 2 | Same direction, half the magnitude |
| partitioned_x16_rb_quarter | 44542→44218 | 396→412 | +26.6 % | +2.6 % | +1.0 % | +8.5 % | 2 | Throughput stable; welfare efficiency up |
| partitioned_x16_rb_third | 44541→44229 | 462→482 | −39.7 % | −12.7 % | +0.1 % | −2.6 % | 2 | Mixed; fees down despite bytes flat |
| partitioned_x4 | 44538→44219 | 2350→1485 | −41.1 % | −29.3 % | −30.2 % | −10.0 % | 0 | x4 floor + un-RB-reduced: standard lane disappears (prior had 1.54B retained-value standard, new is 0) |
| partitioned_x4_rb_half | 44541→44230 | 1651→836 | −80.4 % | −15.4 % | −42.1 % | +12.4 % | 1 | Throughput half-gone but NU only −15 %; RV-ratio jumps |
| partitioned_x4_rb_quarter | 44541→44218 | 383→411 | −30.6 % | +45.3 % | +11.3 % | +4.1 % | 2 | NU up; **single-step shock = 1.41 (above the (D+1)/D bound for D=8); see Implications** |
| partitioned_x4_rb_third | 44541→44230 | 1377→640 | −22.3 % | −12.1 % | −43.6 % | −5.1 % | 1 | Throughput half-gone |
| unreserved_x16 | 44434→29527 | 6707→2798 | **+620.8 %** | +28.9 % | −54.8 % | +45.9 % | 1 | Submitted-tx count collapses 34 % — actors' lane-choice/LatencyEstimator state diverged; priority quote drove fees through the roof |
| unreserved_x4 | 44500→43253 | 6560→7207 | +82.7 % | −5.3 % | +8.1 % | +2.9 % | 1 | Throughput up, fees up; **max p/s ratio jumps 262 → 910 (+248 %)** |

### Sundaeswap priority-only

The first 8 rows below are the priority-only counterparts to the
both-dynamic partitioned rows and have **byte-identical run
summaries** in every field except the pricing-event-stream hash —
this is genuine multi-floor saturation, not data reuse (the smoke
manifests confirm distinct simulator runs). At
`multiplier_floor = 16` with sundaeswap demand the partitioned-both-
dynamic and rb-reserved-priority-only mechanisms reduce to the same
observable trajectory because only the priority lane gets used.

| Job | Sub (p→n) | Inc (p→n) | Fee Δ | NU Δ | Bytes Δ | RV-ratio Δ | SB(new) | Notes |
|---|---|---|---|---|---|---|---|---|
| rb_reserved_x16 | 44539→44229 | 1344→1023 | −53.6 % | −47.3 % | −31.2 % | −1.5 % | 1 | Mirrors both-dynamic partitioned_x16 (incl. hash family) |
| rb_reserved_x16_rb_half | 44542→44229 | 686→617 | −45.4 % | −30.6 % | −16.4 % | −2.1 % | 2 | Mirror |
| rb_reserved_x16_rb_quarter | 44542→44218 | 396→412 | +26.6 % | +2.6 % | +1.0 % | +8.5 % | 2 | Mirror |
| rb_reserved_x16_rb_third | 44541→44229 | 462→482 | −39.7 % | −12.7 % | +0.1 % | −2.6 % | 2 | Mirror |
| rb_reserved_x4 | 44538→44219 | 2350→1485 | −41.1 % | −29.3 % | −30.2 % | −10.0 % | 0 | Mirror; x4 floor + sundaeswap demand means standard lane unused |
| rb_reserved_x4_rb_half | 44541→44230 | 1651→836 | −80.4 % | −15.4 % | −42.1 % | +12.4 % | 1 | Mirror |
| rb_reserved_x4_rb_quarter | 44541→44218 | 383→411 | −30.6 % | +45.3 % | +11.3 % | +4.1 % | 2 | Mirror; single-step shock 1.41 |
| rb_reserved_x4_rb_third | 44541→44230 | 1377→640 | −22.3 % | −12.1 % | −43.6 % | −5.1 % | 1 | Mirror |
| rb_reserved_x8 | 44541→44230 | 2243→1344 | +24.3 % | −28.4 % | −33.4 % | −7.2 % | 1 | Throughput drops, fees still up — quote drift |
| rb_reserved_x8_rb_half | 44541→44230 | 750→813 | +29.3 % | −0.7 % | +7.5 % | −5.6 % | 1 | NU essentially unchanged |
| rb_reserved_x8_rb_quarter | 44542→44229 | 460→460 | +37.7 % | −10.8 % | −1.0 % | −4.7 % | 1 | Same-throughput, different fee mix |
| rb_reserved_x8_rb_third | 44541→44219 | 566→619 | +32.7 % | +21.2 % | +6.9 % | +8.7 % | 1 | NU and throughput both up |
| unreserved_x16 | 44529→43966 | 8112→9243 | +26.2 % | −8.6 % | +12.4 % | −1.6 % | 1 | NU very stable here — un-reserved + both-tier serves more inclusions |
| unreserved_x4 | 44534→43939 | 7874→8607 | +17.1 % | −4.0 % | +7.6 % | +8.0 % | 1 | **max p/s ratio 262 → 900 (+244 %)** |
| unreserved_x8 | 44534→43943 | 8130→8885 | +3.9 % | −7.8 % | +7.4 % | +3.8 % | 1 | **max p/s ratio 393 → 1050 (+168 %)** |

### Sundaeswap singlelane

| Job | Sub (p→n) | Inc (p→n) | Fee Δ | NU Δ | Bytes Δ | RV-ratio Δ | SB(new) | Notes |
|---|---|---|---|---|---|---|---|---|
| baseline_flat_fee | 44571→44258 | 10069→10971 | +9.0 % | +3.0 % | +9.1 % | −7.8 % | 1 | Stable baseline; **standard_included_value 125M → 8.5B (+6735 %)** — lane-attribution shifted not throughput |
| eip1559_d16_t50_w32 | 39895→31762 | 5428→3834 | −4.6 % | −29.6 % | −35.7 % | +30.9 % | 3 | **Slot battles 1 → 3**; submission drops 20 % (actor self-throttle on quote drift); RV-ratio jumps 30 % |
| eip1559_d4_t50_w32 | 29927→7909 | 4424→3443 | **+719.7 %** | +80.3 % | −7.3 % | −12.1 % | 0 | **Massive controller drift**: D=4 is most-reactive setting, less stake-spread→stronger fill-rate signal→quote explodes; submissions collapse 74 % |
| eip1559_d8_t25_w32 | 37501→16256 | 5140→1306 | −50.9 % | −62.2 % | −76.6 % | +18.3 % | 1 | **target=25 % is most-clampable**; submissions collapse 57 %, throughput follows |
| eip1559_d8_t50_w16 | 31480→18967 | 3594→3620 | +351.8 % | +5.3 % | +5.6 % | +9.0 % | 2 | Inclusions flat; price gets dragged up |
| eip1559_d8_t50_w32 | 38597→16446 | 5811→1328 | −17.5 % | **−78.0 %** | −79.1 % | +14.6 % | 1 | Worst NU collapse; the "canonical D=8 t=50 w=32" config |
| eip1559_d8_t50_w64 | 40614→21032 | 7094→4239 | +236.4 % | −33.3 % | −44.8 % | +79.9 % | 1 | Long window = sluggish controller, takes a beating |
| eip1559_d8_t75_w32 | 42870→39196 | 5660→4656 | +4.5 % | −13.6 % | −22.8 % | +22.0 % | 1 | target=75 % is most-resistant — smallest delta |

## Slot-battle activity (WR-1 live status check)

| Arm | Total prior slot battles (seed=1) | Total new slot battles | Direction |
|---|---|---|---|
| both-dynamic (10 jobs) | 0 | 13 | UP |
| priority-only (15 jobs) | 0 | 18 | UP |
| singlelane (8 jobs) | 1 | 10 | UP |
| **All 33 jobs** | **1** | **41** | **41× increase** |

**This contradicts the prior hypothesis.** The note in the task brief
expected the realistic curve to *reduce* slot battles because "more
deterministic winner." That isn't what happened. The likely
mechanism:

- Under uniform stake, the lottery is essentially uniform across 100
  nodes — every slot has ~5 % chance to win at any node, and when
  a slot battle occurs (two sibling RBs in the same slot from
  different producers), each of the 99 other nodes sees one body
  arrive first by header diffusion and rejects the second's
  header before validating the body. The `slot_battles_count`
  metric (which only fires when **both** sibling bodies fully
  validate at the representative) almost never triggers.
- Under the realistic curve, ~35 % of the stake sits on the top-50
  pools and the top-1 has stake share 1.97 % vs 1.00 % uniform.
  When two large-stake producers do collide in a slot (which now
  happens more, because their per-slot lottery probability is
  proportionally higher), the body sizes from large producers
  arrive at the representative with shorter latency tails (closer
  to one another in arrival time), so the late-race subset of
  battles grows. The metric only captures the late-race subset,
  and that subset grows when producer-side bandwidth/latency is
  concentrated at high-stake nodes.

**Implication for WR-1 deferral status:**
- WR-1 is the M1 known limitation (no pricing rollback on fork
  resolution). Prior threat model said this is **dormant** because
  `slot_battles_count` was ~0. The new evidence is that under the
  realistic curve **WR-1 is no longer dormant** at the
  representative — non-zero counts appear in **29 of 33 jobs**, with
  per-run counts of 1–3.
- 41 total slot battles over 33 jobs × 1000-slot runs = ~0.13 % of
  slot positions experience late-race pricing contamination at the
  representative. Still small in absolute terms, but the upper-bound
  framing of `orphaned_pricing_samples` says this is an upper bound
  on the M1 rollback-gap impact — it is no longer trivially zero.
- WR-1 should be **re-classified from "dormant" to "live but
  bounded"** in the phase-2 audit. The bound is `≤ orphaned_pricing_
  samples / pricing_ticks ≈ 0.001` per run.

Sub-finding: the singlelane `eip1559_d16_t50_w32` job hit
`slot_battles_count = 1` even in the prior uniform-stake run. So
slot-battle-zero was never literally true everywhere — but it became
qualitatively non-zero across the suite under the curve.

## Welfare direction flips

**No sign flips.** Every (prior NU, new NU) pair is positive in
both. The most-extreme NU change in either direction is
`eip1559_d8_t50_w32` (−78 %), which stays comfortably positive
(prior 1.35 × 10¹⁰ → new 2.98 × 10⁹). The most-positive NU change
is `eip1559_d4_t50_w32` (+80 %), prior 1.13 × 10¹⁰ → new 2.03 × 10¹⁰.

The most consequential **rank-order** effects:
- **partitioned_x4** vs **partitioned_x4_rb_quarter**: under the
  prior topology, partitioned_x4 had higher NU than the quarter
  variant. Under the new topology partitioned_x4 NU drops 29 %
  while quarter goes UP 45 %, so the gap narrows substantially but
  partitioned_x4 still leads (1.22 × 10¹⁰ vs 4.61 × 10⁹). No flip.
- **unreserved_x4** vs **unreserved_x16** (both-dynamic): prior NU
  was unreserved_x16 (2.83 × 10¹⁰) > unreserved_x4 (2.15 × 10¹⁰).
  New NU is unreserved_x16 (3.65 × 10¹⁰) > unreserved_x4
  (2.03 × 10¹⁰). Gap widens, no flip.
- **eip1559_d4 vs eip1559_d8 vs eip1559_d16** (singlelane NU rank):
  prior: d8_w32 (1.354e10) ≈ d8_w64 (1.435e10) > d16 (1.203e10) >
  d8_t25 (1.187e10) ≈ d4 (1.128e10). New: **d4 jumps to top**
  (2.034e10), d8_w16 = d8_t50w32 fall (2.98e9 and 8.64e9
  respectively). **The "good D" recommendation flips** — under
  uniform stake D=8 looked best, under realistic stake D=4 (more
  reactive controller) looks best because it can absorb the
  larger fill-rate shocks the producer concentration creates.

## Structural vs stochastic

**Structural (consistent direction across multiple jobs in an arm):**
- **Submission counts down ~0.7 % across most partitioned jobs** —
  a small but consistent shift, suggesting actors are
  self-throttling slightly more under the new lane-quote
  trajectories. Likely real, not noise.
- **`max_priority_over_standard_ratio` jumps in unreserved
  variants** — both both-dynamic unreserved_x4 (+248 %) and
  priority-only unreserved_x{4,8,16} (+244 %, +168 %, +93 %) move
  in the same direction. The realistic curve is driving the
  priority controller into higher coefficient regimes because
  producer concentration concentrates the priority demand into the
  same large-producer windows.
- **Slot-battle counts are non-zero almost everywhere** in the new
  data. 29 / 33 jobs report SB ≥ 1. Definitely structural — the
  curve changes the per-slot collision probability.
- **Both-dynamic standard-lane retained value collapses to 0** in
  `partitioned_x4`, `partitioned_x4_rb_half`, `partitioned_x4_rb_third`
  (was 1.5–2.1B prior). The standard lane stops getting hit by
  inclusions under the curve at floor=4 — possibly because
  priority quote got high enough that lane-choice math abandons
  standard, or because EBs are being saturated by priority
  candidates only.

**Stochastic (single-job outlier or opposite direction within an
arm):**
- `partitioned_x4_rb_quarter` and `partitioned_x4_rb_third` have
  opposite NU directions (+45 % vs −12 %) despite both being x4
  rb-reduced variants. Suggests one of these is on the right side
  of a controller bifurcation and the other isn't. With seed=1
  only, this could shrink under a multi-seed average. Worth
  re-checking.
- `eip1559_d4_t50_w32` (+80 % NU, +720 % fees) is the most extreme
  delta in the singlelane arm and stands alone in direction. Could
  be a single-seed outlier near a regime boundary; needs seeds 2/3
  to confirm direction.
- `partitioned_x4_rb_quarter` single-step shock jumps to 1.41
  vs the prior 1.26 (controller's `(D+1)/D` bound for D=8 is
  1.125 nominally; the priority-side bound is `(D'+1)/D'` for D'
  = 4 = 1.25 — 1.41 may indicate the multiplier-floor enforcement
  kicked in to push priority's quote up). Worth one careful read
  of the controller invariants, not a bug to flag immediately.

## Implications for phase-2 conclusions

1. **Mechanism ranking is qualitatively preserved.** Un-reserved
   variants still produce more throughput than rb-reserved /
   partitioned variants, baseline-flat-fee still has the highest
   raw inclusion count of any singlelane config, and EIP-1559
   controllers still successfully shed regret to priority lane.
2. **Magnitudes from the prior 100n-full analysis are NOT safe to
   cite.** Throughput in the most-stressed partitioned jobs is
   half what it was; fees in some unreserved jobs are 6–8 × what
   they were; the "best D" in the singlelane EIP-1559 sweep
   flipped. The prior write-up's per-job numbers should be
   re-generated against the realistic topology before
   external publication.
3. **The phase-2 mechanism design conclusions (priority-only-static
   wins on simplicity, both-dynamic wins on responsiveness) are
   still defensible directionally** because (a) no welfare sign
   flipped, (b) sign of the relative-utility delta between
   un-reserved and reserved variants is preserved, (c) priority
   lane controller still saturates under high demand. But the
   write-up needs the realistic-curve numbers, with a disclosure
   that the uniform-stake numbers had a non-trivial topology bias.
4. **WR-1 needs re-framing** from "dormant assumption" to "live
   threat at the realistic topology, bounded by `orphaned_pricing_
   samples / pricing_ticks ≈ 10⁻³` per run." See `slot-battle
   activity` section above.
5. **The standard-lane disappearance in some partitioned-x4
   variants** (3 of 4 jobs with NU > 0 but standard_retained_value =
   0 under the new curve) deserves a separate look. Either (a)
   lane-choice math is correctly identifying that standard isn't
   profitable here, or (b) something is keeping standard txs out
   of EBs even when standard demand exists. The first is benign;
   the second would be a partition-trigger / sample-attribution
   regression worth a focused unit test.

## Caveats

- **Single seed.** This is seed=1 only on both sides. The prior
  data has seeds 2 and 3 available; matching the smoke at multi-
  seed would tighten the structural-vs-stochastic call on the
  outliers (`eip1559_d4_t50_w32`, `partitioned_x4_rb_quarter`).
- **Sundaeswap-specific demand.** The actor profile is heavy on the
  high-value low-urgency end of the sundaeswap demand mix; results
  may not generalise to `paper_like_*` profiles without a
  parallel smoke. The original phase-2 suite goldens
  (`phase-2-eip1559-robustness`, `phase-2-two-lane-both-dynamic`,
  etc.) use the paper-like demand and have not been re-run on the
  realistic curve.
- **Topology rotation only.** Locations, latencies, bandwidths,
  producer counts are unchanged — but the per-node bandwidth
  bandwidth-per-stake ratio implicitly drops at top-1 (top-1 has
  1.97 % stake but the same ~5 % of upstream bandwidth as any
  other node), which may bias inclusion latency against top-1's
  own txs. Worth checking that the realistic topology's bandwidth
  assignment is mainnet-faithful, not just the stake assignment.
- **Suite goldens are not yet re-run.** All seven suite goldens
  under `parameters/phase-2-sweep/suites/.goldens/` were last
  regenerated on the uniform topology. They will flip when run on
  the realistic curve; an `UPDATE_GOLDENS=1` rebuild is needed
  before any post-curve PR.
- **No simulator commit changed** between prior and new — these
  deltas are 100 % topology-driven. The 33 hash flips confirm the
  pricing event streams diverged from slot ≥ 1 on every job, which
  is the expected effect of a producer-rotation change.
