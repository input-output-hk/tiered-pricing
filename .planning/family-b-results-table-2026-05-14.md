# Family B full-sweep — consolidated results table

Date: 2026-05-14
Run-id: 20260514-160045
Mechanism: Chain-derived (EIP-1559-faithful) on topology-realistic-100.yaml
Coverage: 19 suites, 468 (job, seed) pairs, 0 failures

## How to read this table

Each row in the primary table is one (arm × demand-regime) cell, aggregating across all jobs and seeds whose pricing YAML matches the arm. **Median** is reported instead of mean for heavy-tailed metrics (net utility, retained value, latency). `inclusion rate %` is total included / total submitted across the cell. `lane% priority` is `priority_included / (priority_included + standard_included)` over the lane-tagged component counts. `latency urgency-{low,mid,high}` buckets the components by their value-half-life (low = >= 1h, mid = ~5-30 min, high = <= ~2 min); see the demand mapping section. `hash-diversity` is the count of distinct `pricing_event_stream_sha256` values in the cell — a sanity-check that seeds actually diverge.

**Caveat on latency-by-lane.** The per-component `latency_blocks_observations` array is not separated by lane in the run-summary schema; a single median per cell / per bucket is the closest available proxy. Priority bytes vs standard bytes split is computed by allocating each component's `bytes_included` proportionally to its `priority_included` / `standard_included` tx counts. **Caveat on latency unit.** Despite the field name `latency_blocks_observations`, the values are floats (not integer block counts) — this is the slot-derived inclusion delay normalised by some block-period factor in the metrics collector, so values < 1 are common.

## Demand-profile urgency mappings

Component_index → urgency bucket per demand profile. Buckets are derived from the `half-life-seconds` log-normal medians in each demand YAML (the simulator does not store a discrete urgency field; half-life is the only urgency-shaping knob).

### paper_like_moderate / realistic / congested / mispriced (3 components)

| component_index | label                                   | half-life median | urgency bucket |
|----------------:|:----------------------------------------|:-----------------|:---------------|
| 0               | hard-deadline / arb tail                | 60s              | high           |
| 1               | active DeFi                             | 5 min            | mid            |
| 2               | patient traffic                         | 1 hour           | low            |

### sundaeswap_moderate (11 components)

| component_index | label                          | half-life median | urgency bucket |
|----------------:|:-------------------------------|:-----------------|:---------------|
| 0               | background simple-transfer (1d) | 1d               | low            |
| 1               | background staking/gov (2d)    | 2d               | low            |
| 2               | background moderate-transfer (6h) | 6h               | low            |
| 3               | background routine-swap (30m)  | 30m              | mid            |
| 4               | background larger-defi (15m)   | 15m              | mid            |
| 5               | background small-yield (1h)    | 1h               | low            |
| 6               | dex retail casual (30m)        | 30m              | mid            |
| 7               | dex retail eager (5m)          | 5m               | mid            |
| 8               | dex retail fomo (10m)          | 10m              | mid            |
| 9               | arb dex (60s)                  | 60s              | high           |
| 10              | arb whale (120s)               | 120s             | high           |

## Primary table: per-arm × per-demand (the headline)

Cells with `n pairs = 0` have no jobs in this arm × demand combination (e.g., RB-reserved priority-only does not exist in the EIP-1559 robustness/smoothing suites).

| arm | demand | n pairs | med net_utility | med retained | med fees | sign+ | sign- | inclusion % | med pri-bytes | med std-bytes | lane%-pri | med lat overall | lat low | lat mid | lat high | slot battles | hash-div |
|-----|--------|--------:|----------------:|-------------:|---------:|------:|------:|------------:|--------------:|--------------:|----------:|----------------:|--------:|--------:|---------:|-------------:|---------:|
| single-lane-eip1559 | moderate | 24 | -47,606,694 | 7,503,560,309 | 8,456,140,455 | 12 | 12 | 27.7% | 3,342,257 | 1,917,552 | 64.1% | 13.28 | 18.45 | 13.85 | 10.70 | 50 | 24 |
| single-lane-eip1559 | realistic | 24 | -2.05e+10 | 1.41e+10 | 3.97e+10 | 6 | 18 | 4.6% | 5,957,967 | 33754.3 | 82.4% | 17.08 | 17.20 | 17.90 | 14.60 | 20 | 24 |
| single-lane-eip1559 | congested | 48 | -2.10e+10 | 1.26e+10 | 3.50e+10 | 9 | 39 | 2.1% | 5,494,008 | 0 | 98.4% | 13.30 | 11.40 | 14.70 | 11.45 | 38 | 35 |
| single-lane-eip1559 | sundaeswap | 24 | 208,463,515 | 2.35e+10 | 2.16e+10 | 15 | 9 | 31.1% | 4,795,444 | 845740.7 | 84.7% | 13.45 | 6.00 | 16.60 | 15.05 | 25 | 24 |
| unreserved-priority-only | moderate | 9 | 1.57e+10 | 1.94e+10 | 3,719,428,750 | 9 | 0 | 17.4% | 3,549,337 | 4,575,462 | 42.8% | 24.15 | 43.10 | 2.20 | 1.20 | 18 | 9 |
| unreserved-priority-only | realistic | 9 | 1.32e+10 | 2.30e+10 | 9,776,660,038 | 9 | 0 | 2.9% | 3,902,586 | 4,415,275 | 47.6% | 31.90 | 57.95 | 8.25 | 7.90 | 6 | 9 |
| unreserved-priority-only | congested | 18 | 9,467,706,951 | 1.39e+10 | 3,651,081,780 | 18 | 0 | 1.1% | 2,008,126 | 6,022,080 | 28.3% | 37.85 | 57.95 | 9.25 | 2.25 | 30 | 17 |
| unreserved-priority-only | sundaeswap | 9 | 2.79e+10 | 3.35e+10 | 7,175,681,192 | 9 | 0 | 25.0% | 4,820,171 | 3,874,348 | 46.4% | 19.55 | 8.85 | 19.55 | 13.30 | 9 | 9 |
| rb-reserved-priority-only | moderate | 36 | 3,674,167,405 | 4,597,116,120 | 914,907,192 | 36 | 0 | 2.3% | 669842.5 | 0 | 100.0% | 4.75 | 4.40 | 2.65 | 1.45 | 58 | 36 |
| rb-reserved-priority-only | realistic | 36 | 270,901,900 | 3,667,957,903 | 5,091,112,232 | 18 | 18 | 0.5% | 1,078,587 | 0 | 100.0% | 10.85 | 8.68 | 9.00 | 11.30 | 56 | 36 |
| rb-reserved-priority-only | congested | 57 | 2,114,081,849 | 3,291,531,359 | 1,150,760,683 | 57 | 0 | 0.2% | 793006.0 | 0 | 100.0% | 8.55 | 8.65 | 8.40 | 7.15 | 123 | 37 |
| rb-reserved-priority-only | sundaeswap | 36 | 6,675,035,196 | 1.12e+10 | 3,894,760,607 | 35 | 1 | 5.6% | 1,743,306 | 0 | 100.0% | 13.43 | 1.70 | 11.75 | 15.30 | 50 | 36 |
| unreserved-both-dynamic | moderate | 6 | 1.39e+10 | 2.07e+10 | 7,291,790,222 | 6 | 0 | 21.8% | 2,305,180 | 5,437,803 | 34.0% | 14.90 | 30.60 | 4.45 | 1.20 | 11 | 6 |
| unreserved-both-dynamic | realistic | 6 | 1.70e+10 | 3.29e+10 | 1.50e+10 | 5 | 1 | 3.3% | 5,033,636 | 3,148,303 | 59.5% | 14.43 | 45.25 | 11.40 | 8.25 | 6 | 6 |
| unreserved-both-dynamic | congested | 12 | 4,239,423,384 | 2.53e+10 | 2.04e+10 | 6 | 6 | 1.5% | 2,036,176 | 6,790,708 | 29.3% | 13.90 | 34.35 | 11.85 | 2.85 | 20 | 9 |
| unreserved-both-dynamic | sundaeswap | 6 | 3.03e+10 | 5.64e+10 | 1.90e+10 | 6 | 0 | 26.4% | 3,612,463 | 4,724,616 | 32.4% | 21.60 | 8.05 | 23.35 | 9.20 | 5 | 6 |
| partitioned-both-dynamic | moderate | 24 | 4,025,197,670 | 4,825,846,608 | 980,524,115 | 24 | 0 | 2.3% | 753246.0 | 0 | 100.0% | 3.55 | 4.40 | 2.70 | 1.30 | 33 | 24 |
| partitioned-both-dynamic | realistic | 24 | 270,901,900 | 3,849,684,020 | 5,091,112,232 | 12 | 12 | 0.5% | 1,080,716 | 0 | 100.0% | 10.85 | 8.95 | 9.15 | 11.25 | 38 | 24 |
| partitioned-both-dynamic | congested | 36 | 2,699,483,152 | 4,037,062,572 | 1,154,529,165 | 36 | 0 | 0.2% | 1,269,180 | 0 | 100.0% | 8.50 | 8.65 | 8.30 | 7.15 | 79 | 28 |
| partitioned-both-dynamic | sundaeswap | 24 | 6,160,612,991 | 1.12e+10 | 4,225,333,055 | 23 | 1 | 5.6% | 1,764,990 | 0 | 100.0% | 13.80 | 1.70 | 11.80 | 15.30 | 34 | 24 |

## Per-arm aggregate (Table 2)

Same columns as the primary table, collapsed across all demand regimes.

| arm | n pairs | med net_utility | med retained | med fees | sign+ | sign- | inclusion % | lane%-pri | med lat overall | lat low | lat mid | lat high | slot battles | hash-div |
|-----|--------:|----------------:|-------------:|---------:|------:|------:|------------:|----------:|----------------:|--------:|--------:|---------:|-------------:|---------:|
| single-lane-eip1559 | 120 | -2,370,678,525 | 1.28e+10 | 2.24e+10 | 42 | 78 | 4.3% | 85.9% | 13.45 | 11.40 | 15.40 | 13.30 | 133 | 107 |
| unreserved-priority-only | 45 | 1.27e+10 | 1.74e+10 | 4,565,588,712 | 45 | 0 | 2.4% | 39.3% | 31.90 | 52.55 | 10.05 | 8.90 | 63 | 44 |
| rb-reserved-priority-only | 165 | 2,789,301,898 | 4,741,768,186 | 1,778,369,586 | 146 | 19 | 0.4% | 100.0% | 9.25 | 2.80 | 8.30 | 11.30 | 287 | 145 |
| unreserved-both-dynamic | 30 | 1.70e+10 | 2.78e+10 | 1.50e+10 | 23 | 7 | 3.0% | 36.4% | 14.80 | 34.75 | 12.15 | 7.20 | 42 | 27 |
| partitioned-both-dynamic | 108 | 3,020,093,995 | 5,314,634,065 | 1,990,775,215 | 95 | 13 | 0.5% | 100.0% | 9.15 | 3.18 | 8.25 | 11.25 | 184 | 100 |

## Latency by urgency component (Table 3)

Per-arm median observed latency at each component_index, with the urgency bucket attached. **Note**: component-index semantics differ between the 3-component (paper_like_*) and 11-component (sundaeswap) demand profiles, so any arm that mixes both demand families merges two different component-index meanings into one row. The `demands` column lists which demand families contributed.

| arm | comp_idx | demands | n_obs | median latency | urgency bucket |
|-----|---------:|---------|------:|---------------:|:---------------|
| single-lane-eip1559 | 0 | congested,moderate,realistic,sundae | 124,411 | 10.95 | mixed |
| single-lane-eip1559 | 1 | congested,moderate,realistic,sundae | 396,429 | 14.85 | mixed |
| single-lane-eip1559 | 2 | congested,moderate,realistic,sundae | 162,195 | 15.45 | mixed |
| single-lane-eip1559 | 3 | sundaeswap_moderate | 27,119 | 6.15 | mid |
| single-lane-eip1559 | 4 | sundaeswap_moderate | 18,827 | 6.40 | mid |
| single-lane-eip1559 | 5 | sundaeswap_moderate | 9,815 | 5.95 | low |
| single-lane-eip1559 | 6 | sundaeswap_moderate | 24,733 | 24.60 | mid |
| single-lane-eip1559 | 7 | sundaeswap_moderate | 30,262 | 18.50 | mid |
| single-lane-eip1559 | 8 | sundaeswap_moderate | 12,281 | 26.75 | mid |
| single-lane-eip1559 | 9 | sundaeswap_moderate | 9,720 | 16.25 | high |
| single-lane-eip1559 | 10 | sundaeswap_moderate | 5,359 | 13.30 | high |
| unreserved-priority-only | 0 | congested,moderate,realistic,sundae | 36,985 | 6.60 | mixed |
| unreserved-priority-only | 1 | congested,moderate,realistic,sundae | 129,814 | 7.25 | mixed |
| unreserved-priority-only | 2 | congested,moderate,realistic,sundae | 153,438 | 53.75 | mixed |
| unreserved-priority-only | 3 | sundaeswap_moderate | 9,364 | 7.20 | mid |
| unreserved-priority-only | 4 | sundaeswap_moderate | 7,128 | 3.60 | mid |
| unreserved-priority-only | 5 | sundaeswap_moderate | 3,442 | 8.45 | low |
| unreserved-priority-only | 6 | sundaeswap_moderate | 9,999 | 65.65 | mid |
| unreserved-priority-only | 7 | sundaeswap_moderate | 24,419 | 15.25 | mid |
| unreserved-priority-only | 8 | sundaeswap_moderate | 6,256 | 63.90 | mid |
| unreserved-priority-only | 9 | sundaeswap_moderate | 9,827 | 13.30 | high |
| unreserved-priority-only | 10 | sundaeswap_moderate | 4,167 | 13.30 | high |
| rb-reserved-priority-only | 0 | congested,moderate,realistic,sundae | 40,175 | 9.60 | mixed |
| rb-reserved-priority-only | 1 | congested,moderate,realistic,sundae | 115,126 | 6.90 | mixed |
| rb-reserved-priority-only | 2 | congested,moderate,realistic,sundae | 213 | 4.65 | mixed |
| rb-reserved-priority-only | 3 | sundaeswap_moderate | 4,523 | 1.45 | mid |
| rb-reserved-priority-only | 4 | sundaeswap_moderate | 12,158 | 1.70 | mid |
| rb-reserved-priority-only | 5 | sundaeswap_moderate | 319 | 1.60 | low |
| rb-reserved-priority-only | 6 | sundaeswap_moderate | 625 | 18.35 | mid |
| rb-reserved-priority-only | 7 | sundaeswap_moderate | 40,830 | 15.00 | mid |
| rb-reserved-priority-only | 8 | sundaeswap_moderate | 1,469 | 16.10 | mid |
| rb-reserved-priority-only | 9 | sundaeswap_moderate | 20,214 | 14.95 | high |
| rb-reserved-priority-only | 10 | sundaeswap_moderate | 8,678 | 15.95 | high |
| unreserved-both-dynamic | 0 | congested,moderate,realistic,sundae | 39,465 | 6.85 | mixed |
| unreserved-both-dynamic | 1 | congested,moderate,realistic,sundae | 119,642 | 11.20 | mixed |
| unreserved-both-dynamic | 2 | congested,moderate,realistic,sundae | 70,983 | 35.90 | mixed |
| unreserved-both-dynamic | 3 | sundaeswap_moderate | 6,294 | 6.65 | mid |
| unreserved-both-dynamic | 4 | sundaeswap_moderate | 4,418 | 5.45 | mid |
| unreserved-both-dynamic | 5 | sundaeswap_moderate | 2,337 | 7.60 | low |
| unreserved-both-dynamic | 6 | sundaeswap_moderate | 7,223 | 59.70 | mid |
| unreserved-both-dynamic | 7 | sundaeswap_moderate | 11,302 | 19.05 | mid |
| unreserved-both-dynamic | 8 | sundaeswap_moderate | 4,061 | 59.50 | mid |
| unreserved-both-dynamic | 9 | sundaeswap_moderate | 6,596 | 9.60 | high |
| unreserved-both-dynamic | 10 | sundaeswap_moderate | 3,320 | 8.25 | high |
| partitioned-both-dynamic | 0 | congested,moderate,realistic,sundae | 27,441 | 9.40 | mixed |
| partitioned-both-dynamic | 1 | congested,moderate,realistic,sundae | 80,067 | 6.95 | mixed |
| partitioned-both-dynamic | 2 | congested,moderate,realistic,sundae | 195 | 5.15 | mixed |
| partitioned-both-dynamic | 3 | sundaeswap_moderate | 3,158 | 1.55 | mid |
| partitioned-both-dynamic | 4 | sundaeswap_moderate | 7,708 | 1.80 | mid |
| partitioned-both-dynamic | 5 | sundaeswap_moderate | 225 | 1.70 | low |
| partitioned-both-dynamic | 6 | sundaeswap_moderate | 549 | 18.35 | mid |
| partitioned-both-dynamic | 7 | sundaeswap_moderate | 26,438 | 15.20 | mid |
| partitioned-both-dynamic | 8 | sundaeswap_moderate | 1,148 | 16.40 | mid |
| partitioned-both-dynamic | 9 | sundaeswap_moderate | 13,950 | 15.00 | high |
| partitioned-both-dynamic | 10 | sundaeswap_moderate | 6,017 | 15.95 | high |

## Welfare wins and losses (Table 4)

Top-5 and bottom-5 (arm × demand × multiplier-floor) sub-cells by median net_utility. `mfloor` is parsed from the job name (`x4`/`x8`/`x16`); for single-lane EIP-1559 the parsed token is the `D` parameter (`d4`/`d8`/`d16`).

### Top 5

| rank | arm | demand | mfloor | n pairs | med net_utility | med retained | inclusion % | lane%-pri |
|-----:|-----|--------|:-------|--------:|----------------:|-------------:|------------:|----------:|
| 1 | unreserved-both-dynamic | sundaeswap | 16 | 3 | 3.33e+10 | 6.24e+10 | 27.8% | 17.4% |
| 2 | unreserved-priority-only | sundaeswap | 16 | 3 | 2.81e+10 | 3.60e+10 | 24.7% | 42.8% |
| 3 | unreserved-priority-only | sundaeswap | 4 | 3 | 2.79e+10 | 3.29e+10 | 25.4% | 49.8% |
| 4 | unreserved-priority-only | sundaeswap | 8 | 3 | 2.78e+10 | 3.35e+10 | 25.1% | 46.3% |
| 5 | unreserved-both-dynamic | sundaeswap | 4 | 3 | 2.73e+10 | 3.37e+10 | 25.2% | 45.6% |

### Bottom 5

| rank | arm | demand | mfloor | n pairs | med net_utility | med retained | inclusion % | lane%-pri |
|-----:|-----|--------|:-------|--------:|----------------:|-------------:|------------:|----------:|
| 1 | single-lane-eip1559 | congested | — | 48 | -2.10e+10 | 1.26e+10 | 2.1% | 98.4% |
| 2 | single-lane-eip1559 | realistic | — | 24 | -2.05e+10 | 1.41e+10 | 4.6% | 82.4% |
| 3 | unreserved-both-dynamic | congested | 16 | 6 | -5,460,146,329 | 2.30e+10 | 1.6% | 22.7% |
| 4 | rb-reserved-priority-only | realistic | 8 | 12 | -162,593,526 | 3,494,668,826 | 0.5% | 100.0% |
| 5 | single-lane-eip1559 | moderate | — | 24 | -47,606,694 | 7,503,560,309 | 27.7% | 64.1% |

## Notes and caveats

- **Latency is not lane-separated** at the run-summary schema level — the per-component `latency_blocks_observations` array comingles both lanes' observations. Reported latency medians and the urgency-bucket breakdown are over both lanes.
- **Latency units** are floats, not integer block counts. Values are produced by the metrics collector; treat them as a relative ordering only.
- **Lane-share within sundaeswap arms** is dominated by components 6-10 (DEX retail and arbitrage), which the actor model routes to priority. This explains the elevated `lane%-pri` in sundaeswap rows.
- **`lane%-pri` in the `single-lane-eip1559` arm** reflects the actor's intended `posted_lane`, not a mechanism-level lane distinction. Single-lane mechanisms collapse fees to `Standard`, but the metrics collector still records the actor's posted-lane choice for accounting purposes — so this column is **not** directly comparable to the two-lane arms.
- **`mfloor` parsing** in Table 4 captures the multiplier-floor for two-lane arms and the EIP-1559 `D` parameter for the single-lane arm; the column is not directly comparable across arms but is informative within an arm.
- **Hash-diversity** of 3 within a 3-seed cell indicates seeds are fully diverging, as expected. Values < n pairs would indicate an unexpected seed-insensitivity.
