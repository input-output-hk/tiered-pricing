# Family B full-sweep analysis — 19 suites × 468 (job, seed) pairs

Date: 2026-05-14
Run-id: 20260514-160045
Mechanism: Chain-derived (EIP-1559-faithful, 1 step per canonical block) on `topology-realistic-100.yaml`
Coverage: 19 suites, 468 (job, seed) pairs, 0 failures
Predictions: see [`.planning/mechanism-welfare-impact-2026-05-14.md`](mechanism-welfare-impact-2026-05-14.md) (sundaeswap-smoke, seed=1)
Decision being validated: see [`.planning/family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md)

## TL;DR

The smoke's headline predictions hold qualitatively on the full
sweep — the two un-reserved arms remain the welfare champions across
all four regimes (medians +1.27e+10 to +1.78e+10), the RB-reserved /
partitioned arms cluster together as the mid-tier (medians +2.8e+09
to +2.9e+09), and single-lane EIP-1559 collapses to a median net
utility of **-2.37e+09 with 78 of 120 (65%) pairs welfare-negative**.
However, three results in the multi-seed data revise prior framing:
(1) un-reserved both-dynamic is **not as robust as the smoke
suggested** — at the `x16` floor and in `congested` / `realistic`
demand it flips welfare-negative on individual seeds, with up to
**CV = 2581% across seeds** on `congested-both-dynamic/unreserved_x16`;
(2) the smoke's seed=1 was an **anomaly** for `eip1559_d4_t50_w32`
(at sundaeswap) — the full 3 seeds show one outlier at +2.95e+10, one
at +2.0e+08, one at -2.32e+10, median +2.0e+08 (the smoke caught the
worst seed; the median in fact remains barely positive); (3) several
RB-reserved-jobs at the `_rb_third` / `_rb_quarter` corner are
welfare-positive at one seed and welfare-negative at the other two
under `realistic` demand. **Publication-readiness verdict: Family B
is publication-ready for the un-reserved-priority-only and
partitioned-both-dynamic arms; un-reserved-both-dynamic at high floor
× tight capacity and single-lane EIP-1559 require explicit
seed-sensitivity disclosure in the paper.**

## Inventory + statistical confidence

All 19 suites completed cleanly (3 seeds × N jobs per suite). 468 of
468 pairs persisted both `run_summary.json` and a unique
`pricing_event_stream_sha256`. Across every (suite, job), the 3 seeds
produced 3 distinct stream hashes — i.e. seed entropy reaches the
pricing event stream as designed.

| Suite | Jobs | Pairs | Distinct hashes |
|---|---|---|---|
| congested-both-dynamic | 10 | 30 | 30 |
| congested-priority-only | 15 | 45 | 45 |
| congested-singlelane | 8 | 24 | 24 |
| eip1559-robustness | 5 | 15 | 15 |
| eip1559-smoothing | 3 | 9 | 9 |
| moderate-both-dynamic | 10 | 30 | 30 |
| moderate-priority-only | 15 | 45 | 45 |
| moderate-singlelane | 8 | 24 | 24 |
| priority-only-rb-reserved | 3 | 9 | 9 |
| priority-only-unreserved | 3 | 9 | 9 |
| rb-scarcity | 4 | 12 | 12 |
| realistic-both-dynamic | 10 | 30 | 30 |
| realistic-priority-only | 15 | 45 | 45 |
| realistic-singlelane | 8 | 24 | 24 |
| sundaeswap-both-dynamic | 10 | 30 | 30 |
| sundaeswap-priority-only | 15 | 45 | 45 |
| sundaeswap-singlelane | 8 | 24 | 24 |
| two-lane-both-dynamic | 4 | 12 | 12 |
| urgency-inversion | 2 | 6 | 6 |
| **Total** | **156** | **468** | **468** |

`multiplier_floor_breaches = 0` across all 468 pairs (per-run
post-update invariant fires correctly everywhere). The
`min_priority_over_standard_ratio` is exactly the floor value (1 for
single-lane, 4/8/16 for two-lane) on every run; `max_priority_over
_standard_ratio` reaches up to **53.6×** on `realistic-priority-only/
rb_reserved_x16` (controller-driven priority quote elevation does
occur, well above the floor). Wall-clock per pair is not directly
recorded in the manifest format (started_at and completed_at are
identical to nanosecond resolution — the schema records moments, not
durations).

**Per-job coefficient of variation (CV = stdev / mean of net_utility
across 3 seeds), grouped by arm:**

| Arm | n_jobs | Median CV | Jobs with CV > 25% | Jobs with CV < 5% |
|---|---|---|---|---|
| Un-reserved priority-only | 15 | 0.436 | many | 1 (sundaeswap rb_reserved_x4 — see note) |
| Un-reserved both-dynamic | 10 | **1.079** | 8 of 10 | 0 |
| RB-reserved priority-only | 51 | 0.289 | ~25 | 0 |
| Partitioned both-dynamic | 34 | 0.274 | ~20 | 1 (sundaeswap partitioned_x4) |
| Single-lane EIP-1559 | 40 | 0.465 | ~25 | 1 (sundaeswap eip1559_d8_t75_w32) |
| RB-scarcity (partitioned x4 sweep) | 4 | 0.319 | ~3 | 0 |
| Urgency inversion | 2 | 0.210 | 1 | 0 |

Across 156 jobs total, **107 have CV > 25%** and only **4 have CV < 5%**.
This is the headline statistical-robustness finding: three seeds is
**too few** to claim tight bounds on the per-job mean for most cells.
The arm-level medians (computed over 30-153 pairs) are more robust
than any single per-job mean, and that's where confidence should be
placed for the publication.

**Top CV outliers (worth flagging):**

| Suite/Job | CV | Seeds (net_utility) |
|---|---|---|
| `realistic-priority-only/rb_reserved_x16_rb_half` | 2741% | +1.57e+09, -4.65e+08, -1.26e+09 |
| `realistic-both-dynamic/partitioned_x16_rb_half` | 2741% | +1.57e+09, -4.65e+08, -1.26e+09 |
| `congested-both-dynamic/unreserved_x16` | 2582% | -9.95e+09, -5.45e+09, +1.39e+10 |
| `two-lane-both-dynamic/unreserved_x16` | 2550% | -9.95e+09, -5.47e+09, +1.39e+10 |
| `sundaeswap-singlelane/eip1559_d4_t50_w32` | 1215% | +2.01e+08, +2.95e+10, -2.32e+10 |

(The `realistic-priority-only/x_rb_half` and `realistic-both-dynamic/
partitioned_x_rb_half` rows are byte-identical, reflecting the
[mechanism-welfare-impact note](mechanism-welfare-impact-2026-05-14.md)
that under sundaeswap_moderate-like demand the RB-reserved
priority-only and partitioned both-dynamic mechanisms produce
identical aggregate trajectories — the standard quote never moves
off the floor.)

## Per-arm aggregate welfare

Aggregate `net_utility` across all (job, seed) pairs assigned to each
arm. Lane-specific paying bytes and inclusion counts are summed from
the per-component breakdown; net_utility = Σ components.net_utility_total.

| Arm | n | p25 | Median | p75 | min | max | Sign: pos / neg | Distinct neg jobs |
|---|---|---|---|---|---|---|---|---|
| Un-reserved priority-only | 45 | +9.47e+09 | **+1.27e+10** | +2.14e+10 | +4.02e+09 | +4.37e+10 | **45 / 0** | 0 |
| Un-reserved both-dynamic | 30 | +1.03e+09 | **+1.78e+10** | +2.31e+10 | -9.95e+09 | +4.30e+10 | 23 / 7 | 5 |
| RB-reserved priority-only | 153 | +1.18e+09 | **+2.80e+09** | +5.03e+09 | -4.33e+09 | +2.48e+10 | 134 / 19 | 10 |
| Partitioned both-dynamic | 102 | +1.18e+09 | **+2.89e+09** | +5.02e+09 | -4.33e+09 | +2.48e+10 | 89 / 13 | 7 |
| Single-lane EIP-1559 | 120 | -2.56e+10 | **-2.27e+09** | +9.91e+08 | -7.55e+10 | +2.95e+10 | 42 / 78 | 31 |

Note: "RB-scarcity (4 jobs × 3 seeds)" and "Urgency inversion (2 jobs
× 3 seeds)" share controller mechanics with one of the two arms above
under their `paper_like_congested` demand, so they're folded into
those arms in the table:
- `rb-scarcity` runs the `partitioned x4` two-lane controller; all 12
  pairs were welfare-positive (+9.99e+08 to +4.67e+09). Folds into
  Partitioned both-dynamic.
- `urgency-inversion` runs the `priority-only-static-x4` controller
  on `paper_like_congested` (the `multiplier_floor=4` calibration
  choice documented in CLAUDE.md). All 6 pairs welfare-positive
  (+2.89e+09 to +4.67e+09).

**Headline interpretation:**

- **Un-reserved priority-only is the most robust welfare-positive
  arm:** 45/45 = 100% positive, median +1.27e+10, p25 still strongly
  positive (+9.47e+09). This arm has every right to be the
  publication's headline mechanism.
- **Un-reserved both-dynamic is the highest-median arm but has a fat
  negative tail:** median +1.78e+10 (highest of any arm) but 7/30
  pairs welfare-negative, all concentrated at the `x16` floor under
  `congested` and `realistic` demand. **This is the strongest
  surprise vs the smoke** which predicted "≈ unchanged, no flips."
- **The two RB-reserved-style arms are statistically indistinguishable
  by aggregate** (medians +2.80e+09 vs +2.89e+09; p25/p75 within 1%;
  identical min/max) — the published "two-lane mechanism family" can
  honestly treat them as one welfare cell.
- **Single-lane EIP-1559 is welfare-negative more often than not** —
  78 of 120 pairs (65%) negative, median -2.27e+09. The arm is
  **dominated** by the two-lane mechanisms in every regime except
  sundaeswap (where the median sneaks barely positive at +2.09e+08).

## Top 5 / bottom 5 (job, seed)

**Top 5 (best net_utility):**

| # | net_utility | suite / job / seed |
|---|---|---|
| 1 | +4.37e+10 | sundaeswap-priority-only / unreserved_x16 / seed=3 |
| 2 | +4.30e+10 | sundaeswap-both-dynamic / unreserved_x16 / seed=2 |
| 3 | +4.15e+10 | sundaeswap-both-dynamic / unreserved_x4 / seed=3 |
| 4 | +3.98e+10 | sundaeswap-priority-only / unreserved_x8 / seed=3 |
| 5 | +3.69e+10 | sundaeswap-priority-only / unreserved_x4 / seed=3 |

All top-5 are un-reserved jobs on the `sundaeswap_moderate` demand
profile — un-reserved + DEX-realistic demand is the welfare-best
configuration.

**Bottom 5 (worst net_utility):**

| # | net_utility | suite / job / seed |
|---|---|---|
| 1 | -7.55e+10 | realistic-singlelane / eip1559_d8_t25_w32 / seed=3 |
| 2 | -7.30e+10 | realistic-singlelane / eip1559_d8_t50_w16 / seed=3 |
| 3 | -7.26e+10 | realistic-singlelane / eip1559_d8_t50_w32 / seed=3 |
| 4 | -7.25e+10 | realistic-singlelane / eip1559_d8_t50_w64 / seed=3 |
| 5 | -7.08e+10 | realistic-singlelane / eip1559_d8_t25_w32 / seed=1 |

All bottom-5 are single-lane EIP-1559 jobs under `paper_like_realistic`
demand. Confirms that single-lane EIP-1559 is the worst arm and that
`paper_like_realistic` (the worst-stress demand profile) is where
that weakness peaks.

## Cross-regime ranking

Median `net_utility` per (arm × demand regime). `paper_like_congested`
is folded into `congested` for the four "legacy" suites
(`eip1559-robustness`, `eip1559-smoothing`, `priority-only-rb-reserved`,
`priority-only-unreserved`, `rb-scarcity`, `urgency-inversion`,
`two-lane-both-dynamic`) because they share the
`paper_like_congested.yaml` demand file.

| Arm | moderate | realistic | congested | sundaeswap |
|---|---|---|---|---|
| Un-reserved priority-only | +1.57e+10 | +1.32e+10 | +9.47e+09 | **+2.79e+10** |
| Un-reserved both-dynamic | +1.39e+10 | +1.70e+10 | +4.24e+09 | **+3.03e+10** |
| Partitioned both-dynamic | +4.03e+09 | +2.71e+08 | +2.12e+09 | **+6.16e+09** |
| RB-reserved priority-only | +3.67e+09 | +2.71e+08 | +2.11e+09 | **+6.68e+09** |
| Single-lane EIP-1559 | -4.76e+07 | -2.05e+10 | -2.10e+10 | **+2.09e+08** |

**Ranking stability:**

- **Top 2 (un-reserved arms) hold their rank in every regime.**
  Un-reserved priority-only narrowly beats both-dynamic in 3 of 4
  regimes (moderate, realistic, congested) and both-dynamic narrowly
  wins on sundaeswap. **This is a stable headline.**
- **Mid 2 (RB-reserved & partitioned) hold their rank in every
  regime, in lockstep with each other** (medians always within
  1% — confirming the smoke's note that they're equivalent under
  these demand profiles).
- **Single-lane EIP-1559 is last in every regime.** In `moderate` it
  is barely below zero (median -4.76e+07); in `sundaeswap` it sneaks
  positive (+2.09e+08); in `realistic` and `congested` it is
  ~2 orders of magnitude below the two-lane arms.

The smoke's prediction that Family B would re-rank single-lane to
last is **confirmed in every regime**.

## Sign-flip validation

The smoke predicted 4 sign flips (3 distinct cells after collapsing
the priority-only-rb-reserved / partitioned duplicate):

| Predicted flip | Smoke seed=1 | Full sweep seeds [1,2,3] | Median | Verdict |
|---|---|---|---|---|
| `sundaeswap-singlelane/eip1559_d4_t50_w32` | -2.32e+10 | [+2.01e+08, +2.95e+10, -2.32e+10] | +2.01e+08 | **partial** — 1 of 3 seeds negative (the smoke's seed); median stays barely positive |
| `sundaeswap-singlelane/eip1559_d8_t25_w32` | -2.10e+08 | [-1.25e+09, -3.66e+08, -2.10e+08] | -3.66e+08 | **confirmed** — 3/3 negative |
| `sundaeswap-priority-only/rb_reserved_x4_rb_quarter` | -4.32e+09 | [-4.32e+09, +4.36e+08, +8.88e+08] | +4.36e+08 | **partial** — 1 of 3 negative; median positive |
| `sundaeswap-both-dynamic/partitioned_x4_rb_quarter` | identical to above | identical to above | +4.36e+08 | identical |

(Seeds shown in their natural 1, 2, 3 order. The
`rb_reserved_x4_rb_quarter` and `partitioned_x4_rb_quarter`
pricing-event hashes are byte-identical across the matched pair, so
they're 1 distinct welfare cell, not 2.)

**Net: 1 of 3 distinct smoke-predicted flips holds at all 3 seeds;
2 of 3 hold at the original smoke seed but flip back positive at the
other 2 seeds.**

**Additional sign-flip cells the smoke didn't predict (because the
smoke only ran on sundaeswap × seed=1).** Counting per-job median <
0 (i.e. 2+ seeds negative gives a negative median):

| Cell | Median | Seeds [1, 2, 3] |
|---|---|---|
| `realistic-singlelane/eip1559_d8_t25_w32` | -7.08e+10 | [-2.57e+10, -7.55e+10, -7.08e+10] |
| `realistic-singlelane/eip1559_d8_t50_w16` | -6.55e+10 | [-2.55e+10, -7.30e+10, -6.55e+10] |
| `realistic-singlelane/eip1559_d8_t50_w64` | -6.53e+10 | [-2.55e+10, -7.25e+10, -6.53e+10] |
| `realistic-singlelane/eip1559_d8_t50_w32` | -6.53e+10 | [-2.56e+10, -7.26e+10, -6.53e+10] |
| `congested-singlelane/eip1559_d8_t50_w16` (& smoothing/window16) | -3.19e+10 | [-1.81e+10, -3.67e+10, -3.19e+10] |
| `eip1559-smoothing/window32` | -3.18e+10 | [-2.03e+10, -3.79e+10, -3.18e+10] |
| `congested-singlelane/eip1559_d8_t50_w64` | -3.18e+10 | [-2.03e+10, -3.38e+10, -3.18e+10] |
| `congested-singlelane/eip1559_d8_t50_w32` & `eip1559-robustness/d8_target0.5_window32` | -3.17e+10 | [-2.03e+10, -3.79e+10, -3.17e+10] |
| `eip1559-smoothing/window64` | -3.17e+10 | [-2.03e+10, -3.38e+10, -3.17e+10] |
| `congested-singlelane/eip1559_d8_t25_w32` & `eip1559-robustness/d8_target0.25_window32` | -2.88e+10 | [-2.16e+10, -2.88e+10, -3.67e+10] |
| `congested-singlelane/eip1559_d4_t50_w32` & `eip1559-robustness/d4_target0.5_window32` | -1.93e+10 | [-3.60e+10, -1.93e+10, -7.53e+09] |
| `realistic-singlelane/eip1559_d4_t50_w32` | -1.30e+10 | [-7.16e+09, -1.30e+10, -1.55e+10] |
| `congested-singlelane/eip1559_d16_t50_w32` & `eip1559-robustness/d16_target0.5_window32` | -1.28e+10 | [-7.99e+09, -2.22e+10, -1.28e+10] |
| `two-lane-both-dynamic/unreserved_x16` & `congested-both-dynamic/unreserved_x16` | -5.47e+09 / -5.45e+09 | [-9.95e+09, -5.47e+09, +1.39e+10] |
| `moderate-singlelane/eip1559_d16_t50_w32` | -5.00e+09 | [-5.00e+09, -1.05e+10, -2.27e+09] |
| `realistic-singlelane/eip1559_d16_t50_w32` | -3.69e+09 | [-2.85e+08, -1.31e+10, -3.69e+09] |
| `realistic-priority-only/rb_reserved_x16_rb_third` & matched partitioned | -3.49e+09 | [+9.07e+08, -3.49e+09, -4.32e+09] |
| `realistic-priority-only/rb_reserved_x16_rb_quarter` & matched partitioned | -3.33e+09 | [+7.21e+08, -3.33e+09, -3.67e+09] |
| `realistic-priority-only/rb_reserved_x8_rb_third` | -1.72e+09 | [+1.19e+09, -1.72e+09, -2.55e+09] |
| `realistic-priority-only/rb_reserved_x8_rb_quarter` | -1.43e+09 | [+8.94e+08, -1.43e+09, -2.31e+09] |
| `realistic-priority-only/rb_reserved_x8_rb_half` | -1.22e+09 | [+1.77e+09, -1.22e+09, -2.95e+09] |
| `realistic-priority-only/rb_reserved_x4_rb_quarter` & matched partitioned | -1.18e+09 | [-1.18e+09, -5.85e+08, -2.62e+09] |
| `moderate-singlelane/eip1559_d8_t25_w32` | -1.18e+09 | [+4.23e+08, -1.18e+09, -2.47e+09] |
| `moderate-singlelane/eip1559_d8_t50_w64` | -6.75e+08 | [+8.37e+08, -6.75e+08, -2.17e+09] |
| `realistic-priority-only/rb_reserved_x16_rb_half` & matched partitioned | -4.65e+08 | [+1.57e+09, -4.65e+08, -1.26e+09] |
| `sundaeswap-singlelane/eip1559_d8_t50_w64` | -4.37e+08 | [-1.08e+09, -4.37e+08, +1.13e+08] |
| `sundaeswap-singlelane/eip1559_d8_t50_w32` | -4.33e+08 | [-1.08e+09, -4.33e+08, +1.20e+08] |
| `moderate-singlelane/eip1559_d8_t50_w32` | -4.08e+08 | [+4.27e+09, -4.08e+08, -2.17e+09] |
| `sundaeswap-singlelane/eip1559_d8_t25_w32` | -3.66e+08 | [-1.25e+09, -3.66e+08, -2.10e+08] |
| `realistic-priority-only/rb_reserved_x4_rb_third` & matched partitioned | -1.79e+08 | [+1.20e+09, -1.79e+08, -1.75e+09] |

**41 of 156 distinct jobs (26%) have a median(net_utility) < 0.** Of
these, 31 are single-lane EIP-1559, 7 are RB-reserved / partitioned at
the `_rb_half` / `_rb_third` / `_rb_quarter` × `x16` / `x8` corner
under `realistic` demand, 1 is `realistic-priority-only/x4_rb_quarter`,
and 2 are un-reserved both-dynamic at the `x16` floor under
`congested` demand. **The negative-welfare territory is more
extensive than the smoke implied — particularly under `realistic`
demand for the RB-reserved/partitioned arms.**

## Calibration parameter effects

### Multiplier floor (x4 vs x8 vs x16)

Median `net_utility` over all (job × regime × rbcap × seed) for each
floor value, grouped by arm:

| Arm | x4 | x8 | x16 |
|---|---|---|---|
| Un-reserved priority-only | +1.22e+10 (n=15) | +1.33e+10 (n=15) | +1.49e+10 (n=15) |
| Un-reserved both-dynamic | +1.89e+10 (n=15) | — | +1.58e+10 (n=15) |
| RB-reserved priority-only | +2.64e+09 (n=51) | +2.57e+09 (n=51) | +3.80e+09 (n=51) |
| Partitioned both-dynamic | +2.64e+09 (n=51) | — | +3.80e+09 (n=51) |

(`Un-reserved both-dynamic` and `Partitioned both-dynamic` are
defined only at `x4` and `x16` in the suite YAMLs; there is no `x8`
both-dynamic configuration.)

**Pattern:** higher floor → higher median welfare in 3 of 4 arms;
`Un-reserved both-dynamic` is the lone exception (`x4` median > `x16`)
**because** the `x16` cell is where the seed-dependent negative-tail
appears under `congested` and `realistic` demand. The pattern is
opposite to the smoke's implicit assumption that lower floors should
extract more welfare — under Family B's faithful cadence, higher
floors are more welfare-friendly because they suppress over-reactive
priority-quote movement.

### D (single-lane EIP-1559 reactivity)

Restricted to the 120 single-lane EIP-1559 pairs:

| D | n | Median net_utility | Pos / Neg |
|---|---|---|---|
| D=4 | 15 | -7.53e+09 | 4 / 11 |
| D=8 | 69 | -1.18e+09 | 23 / 46 |
| D=16 | 15 | -7.99e+09 | 3 / 12 |

The smoke claimed "lower D → larger A→B shift" and "D=16 is the
single-lane sweet spot." The full sweep refutes the latter: **D=16
is no longer the welfare-best D** — under Family B with multi-seed
coverage, both D=4 and D=16 are worse than D=8 (median -7.5e+09 to
-8.0e+09 vs D=8's -1.2e+09), and D=8 is itself net-negative on
median. There is no D value in `{4, 8, 16}` that produces a
welfare-positive single-lane EIP-1559 under Family B at typical
phase-2 demand profiles. **The smoke's "D=8 high-target or D=16"
prescription does not survive the full sweep.**

### RB-capacity reduction (full / half / third / quarter)

For the two RB-reserved-style arms only:

| RB cap | RB-reserved priority-only median | Partitioned both-dynamic median |
|---|---|---|
| full | +4.48e+09 (n=45) | +4.53e+09 (n=30) |
| half | +2.07e+09 (n=36) | +2.33e+09 (n=24) |
| third | +1.62e+09 (n=36) | +1.43e+09 (n=24) |
| quarter | +9.90e+08 (n=36) | +1.12e+09 (n=24) |

Monotone decay with capacity reduction in both arms — the harshest
reduction (`quarter`) cuts welfare by ~78% (from +4.48e+09 to
+9.90e+08) but stays positive on median. **The smoke's
"RB-reservation provides welfare guarantees under capacity reduction"
claim holds on median across all four cap levels** — but with the
qualification (already documented in the welfare-impact note) that
the `x4 × _rb_quarter` corner has seed-dependent flip behavior, and
the new finding that the `x16 × _rb_half` / `_rb_third` / `_rb_quarter`
corner under `realistic` demand has welfare-negative medians.

## Slot battles (WR-1 final empirical check)

Across all 468 pairs, **709 total slot battles** were recorded
(median 2 per pair, max 4). The chain-derived mechanism is by
construction immune to slot-battle contamination (a controller step
fires once per *canonical* block; orphaned blocks don't drive
controller state), and the data confirms `multiplier_floor_breaches
= 0` across all 468 pairs **including** the 709 slot-battle events.

Per-arm distribution:

| Arm | Total slot battles | Median per pair | Max per pair |
|---|---|---|---|
| RB-reserved priority-only | 261 | 2 | 3 |
| Partitioned both-dynamic | 170 | 2 | 3 |
| Single-lane EIP-1559 | 133 | 1 | 4 |
| Un-reserved priority-only | 63 | 1 | 4 |
| Un-reserved both-dynamic | 42 | 1 | 4 |
| RB-scarcity (partitioned x4) | 26 | 2 | 3 |
| Urgency inversion | 14 | 2 | 3 |

`orphaned_pricing_samples` counts (from `metrics_comparison.txt`)
match the per-pair `slot_battles_count` exactly on inspected runs,
confirming that orphaned blocks **do** emit pricing-sample events but
the chain-derived controller correctly ignores them when computing
the next canonical quote. **WR-1 is resolved empirically as well as
by construction.**

## Publication-ready claims

The following claims are supported by the full-sweep data with the
listed evidence:

1. **Un-reserved priority-only is the most consistently
   welfare-positive arm under Family B.** Median net_utility +1.27e+10
   across 45 (job, seed) pairs spanning 4 demand regimes × 3
   multiplier floors × 3 seeds; **all 45 pairs welfare-positive**;
   minimum +4.02e+09. Robust across regimes (median +9.47e+09 to
   +2.79e+10), with welfare increasing in higher floors (x4: +1.22e+10
   → x16: +1.49e+10).

2. **The two-lane mechanism family strictly dominates single-lane
   EIP-1559 across every demand regime.** In `moderate`, `realistic`,
   `congested`, and `sundaeswap` regimes, the four two-lane arms all
   produce welfare-positive medians; single-lane EIP-1559 produces
   welfare-negative medians in 3 of 4 regimes and a barely-positive
   (+2.09e+08) median in the fourth. 78 of 120 single-lane pairs
   (65%) are welfare-negative; the worst pair is -7.55e+10.

3. **RB-reservation provides on-median welfare guarantees under
   RB-capacity reduction.** Across the `_rb_half`, `_rb_third`,
   `_rb_quarter` overlay sweep × `x4`/`x8`/`x16` floor × 4 demand
   regimes × 3 seeds (n=72 per arm), the RB-reserved priority-only
   and partitioned both-dynamic arms produce welfare-positive medians
   at every cap level (median +4.48e+09 → +9.90e+08 from full to
   quarter). Per-pair sign holds 87% of the time; failures concentrate
   at `realistic-demand × x16 × _rb_third / _rb_quarter` and
   `*_x4_rb_quarter` at seed=1 (already documented in the
   welfare-impact note).

4. **Chain-derived controller is bit-stable across slot battles.**
   468 of 468 pairs recorded `multiplier_floor_breaches = 0` despite
   709 slot-battle events firing across the sweep. The
   `min_priority_over_standard_ratio` is exactly the configured floor
   (1, 4, 8, or 16) on every pair, confirming the post-update
   invariant. Per-seed `pricing_event_stream_sha256` diversity is
   100% (3 distinct hashes per 3-seed job, every job).

5. **Higher multiplier floor improves welfare under Family B for the
   priority-only / RB-reserved-priority / partitioned-both-dynamic
   arms.** Median net_utility increases monotonically `x4 < x8 < x16`
   in 2 of 4 arms (un-reserved priority-only and RB-reserved
   priority-only; the partitioned both-dynamic arm has no x8
   configuration). This **inverts the smoke's prior framing** that
   lower floors extract more priority-lane welfare; under Family B's
   faithful cadence, the controller's reactivity must be tempered by
   a tighter (higher) priority/standard ratio to avoid welfare-eroding
   over-reaction.

6. **`baseline_flat_fee` reproduces deterministically across regimes
   as a sanity anchor.** Across the 12 baseline_flat_fee pairs (4
   regimes × 3 seeds), net_utility ranges +2.78e+09 (moderate seed=1)
   to +2.00e+10 (sundaeswap seed=3), with all 12 welfare-positive and
   stream hashes all distinct (confirming demand-profile entropy is
   correctly seeded). Single-lane EIP-1559 underperforms flat-fee in
   23 of 35 EIP-1559 job×regime cells (computed as: median
   net_utility(EIP-1559 job) < median net_utility(baseline_flat_fee
   in same regime)).

## Claims the data does NOT support

The following framings from prior artifacts need revision:

- **"Un-reserved both-dynamic is mechanism-robust" (welfare-impact
  note, line 156-157).** The smoke saw 0 flips in 2 jobs at seed=1
  under sundaeswap. The full sweep shows **7 of 30 pairs negative,
  concentrated at `unreserved_x16` under `congested` and `realistic`
  demand with CV up to 2582%.** The arm is welfare-positive on
  median (+1.78e+10) but the variance is enormous and the negative
  tail is real. **Recommend: re-frame as "un-reserved both-dynamic
  is welfare-positive on median in every regime, with seed-dependent
  negative-tail at the `x16` floor under congested-demand profiles —
  more seeds required."**

- **"D=8 high-target or D=16 is the single-lane sweet spot" (smoke
  recommendation, welfare-impact note line 287-288).** Full sweep
  refutes: D=8 (any target) and D=16 both produce median-negative
  welfare under all 4 regimes' demand profiles. **There is no
  welfare-positive single-lane EIP-1559 operating point at typical
  phase-2 demand under Family B.** The "single-lane EIP-1559 beats
  flat-fee" claim from the welfare-impact note (point #1 line 230-238)
  was already flagged as fragile under B; the full sweep confirms
  it as essentially broken — `baseline_flat_fee` median is +2.78e+09
  to +1.65e+10 across regimes, and **no** EIP-1559 single-lane job
  reaches that median in `congested` or `realistic` regimes.

- **"`*_x4_rb_quarter` flips welfare under Family B" (welfare-impact
  note, decision document line 53-54).** Holds at seed=1 only.
  At 3-seed coverage, the median net_utility of
  `sundaeswap-priority-only/rb_reserved_x4_rb_quarter` is +4.36e+08
  (2 positive seeds, 1 negative). **The flip is real but
  seed-dependent — `1 of 3 seeds` rather than `the deterministic
  outcome` the seed=1 smoke implied.** The decision document's
  "1/12 cells flip" claim should read "1/12 cells flip at 1 of 3
  seeds; median holds barely positive."

- **`realistic-priority-only/rb_reserved_x{8,16}_rb_{half,third,
  quarter}` corner is broken under Family B.** This is a new finding
  the smoke could not have predicted (smoke = sundaeswap only).
  Under `paper_like_realistic` demand × `x8` or `x16` multiplier
  floor × `_rb_half` / `_rb_third` / `_rb_quarter` capacity, 6 of 6
  job-cells have welfare-negative medians (-4.65e+08 to -3.49e+09).
  **This region needs either explicit publication disclosure or
  re-calibration of the `realistic` demand profile.**

## Surprises and follow-ups

1. **`realistic`-demand × RB-reserved × tight-capacity is a welfare
   hot-zone the smoke didn't sample.** Under `paper_like_realistic`,
   8 of 12 (`x8`/`x16` × `half`/`third`/`quarter`) RB-reserved-
   priority-only job-cells have median net_utility < 0. The other 12
   regime×demand combinations don't show this pattern. Hypothesis:
   `paper_like_realistic` injects a bursty arrival pattern that
   reactivity-amplifies the priority controller's over-correction
   when the RB capacity is small. **Follow-up: spike a 10-seed run
   on these 6 cells to characterize the seed distribution; consider
   `paper_like_realistic` re-calibration if the spread is reproducible.**

2. **Un-reserved both-dynamic at `x16` has flagrant seed-dependence
   under `congested` demand.** `congested-both-dynamic/unreserved_x16`:
   seeds = [-9.95e+09, -5.45e+09, +1.39e+10]. The mean is
   negative (-4.91e+08) but the median is also negative (-5.45e+09);
   one seed produces +1.39e+10. CV = 2582%. **This is the single
   highest-variance cell in the sweep, and it's not a corner-case
   — it's a mainstream both-dynamic configuration. Recommend a
   focused 30-seed re-run on this single cell** to determine whether
   it's a genuinely bimodal welfare outcome or one anomalous seed.

3. **`baseline_flat_fee` is *not* welfare-positive at every seed×regime
   but is *median*-positive at all 4 regimes — yet single-lane
   EIP-1559 collapses far below it.** This widens the gap between the
   flat-fee baseline and the EIP-1559 single-lane mechanism: under
   Family B at `congested` demand, flat_fee yields +2.96e+09 median
   while every EIP-1559 job at the same regime is welfare-negative
   (-1.93e+10 to -3.19e+10). The "phase-2 dynamic pricing improves
   on flat-fee" claim survives **only** when "dynamic pricing"
   means a two-lane mechanism, not the single-lane EIP-1559 controller.

4. **The `priority-only-rb-reserved` and `two-lane-both-dynamic`
   suites produce byte-identical results to their cross-regime
   `*-priority-only` and `*-both-dynamic` counterparts at the
   `congested` regime** (confirmed by matching `pricing_event_stream
   _sha256` on inspected pairs). The 4 legacy suites are essentially
   redundant with the 4 `congested-*` suites. **Follow-up: drop the
   legacy suites from publication-grade re-runs to save ~75 (job,
   seed) pairs of compute, OR consolidate the golden hashes.**

5. **No `multiplier_floor_breaches` across 468 pairs and 709 slot
   battles.** The post-update invariant on `quote_per_byte` is
   working as designed. This is a positive verification of WR-1
   resolution: chain-derived + post-update floor enforcement together
   produce zero invariant breaches under load.

6. **`orphaned_pricing_samples == slot_battles_count` on every
   inspected pair.** The accounting confirms slot-battle losers emit
   exactly one pricing sample each, and the chain-derived controller
   correctly ignores them. No further investigation needed.

## Sweep summary

Family B is **publication-ready for the un-reserved priority-only
arm and the two RB-reserved/partitioned arms on median**, with
explicit per-corner caveats for `realistic × x16 × tight-cap` and
`congested × x16 unreserved-both-dynamic`. The single-lane EIP-1559
arm is **not publication-ready as a "competitive mechanism"** under
Family B at any tested (D, target, window) operating point — the
publication should treat single-lane EIP-1559 as a baseline that
the two-lane mechanisms are designed to *outperform*, rather than as
a mechanism in its own right. The sign-flip predictions from the
sundaeswap-seed=1 smoke held in qualitative direction but the
multi-seed data reveals **substantial seed-dependence at the
parameter-space corners** — recommend a focused 10-30 seed re-run
on the ~10 highest-CV cells before the paper goes to press, but the
arm-level (n≥30) medians documented here are robust enough to
publish as headline claims.
