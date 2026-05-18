# TEST-02 Scoping Results — Wall-clock Calibration for Wave 2

**Run date:** 2026-05-18
**Source:** Phase 3 Plan 03-01 Task 2 (TEST-02 scoping run)
**Suite:** `parameters/phase-2-sweep/suites/phase-3-scoping.yaml`
**Manifest:** `sim-rs/output/phase-3/scoping-20260518-083210/manifest.json`

## Canonical-job choice

| Field | Value |
|---|---|
| Job name | `multiplier_x4` |
| Pricing | `parameters/phase-2-sweep/pricing/two_lane_priority_only_unreserved_x4.yaml` |
| Mechanism | Two-lane priority-only-static, un-reserved, multiplier_floor = 4 |
| Topology | `topology-realistic-100.yaml` (100 nodes, mainnet-stratified, epoch-582) |
| Protocol | `protocol-base.yaml` (Phase-2 baseline) |
| Demand | `paper_like_congested.yaml` |
| Slots | 2000 |

**Rationale (per CONTEXT.md D-22 / Claude's Discretion §"TEST-02 canonical job choice"):**
the un-reserved priority-only-static cell at floor = 4 is representative of the
menu (one of four mechanism arms) and is NOT one of the four sign-flip cells
(`d4_t50_w32`, `d8_t25_w32`, `x4_rb_quarter` under rb-reserved-priority or
partitioned arms). Picking a sign-flip cell for scoping would have biased the
wall-clock measurement by the very cells TEST-03 is designed to characterise.

## Wall-clock measurement (Bias-corrected and accelerated (BCa) bootstrap input)

Suite started at `2026-05-18T08:32:10Z`. Five seeds ran in parallel under
`experiment-suite run -P 8` (intra-suite parallelism = 8, cross-suite = 1 via
`scripts/run-phase-3-suites.sh`). Per-(job, seed) wall-clock derived from
manifest completion timestamps:

| seed | completed_at_utc | wall-clock from suite start | included | evicted | pricing_event_stream.sha256 (12-char prefix) |
|---|---|---|---|---|---|
| 5 | 08:33:39.781Z | 89.0 s | 6938 | 6026 | `9f62aacb842d` |
| 1 | 08:33:42.213Z | 91.4 s | 7738 | 6283 | `5533f2d2a2f3` |
| 4 | 08:33:44.383Z | 93.5 s | 7999 | 6473 | `032706f1054f` |
| 2 | 08:33:45.695Z | 94.9 s | 7452 | 6348 | `1b52c05dd350` |
| 3 | 08:33:50.376Z | 99.5 s | 8790 | 5733 | `6489fc81842a` |

**Mean wall-clock per (job, seed):** 93.7 s
**Max wall-clock per (job, seed):** 99.5 s
**Intra-suite parallelism used:** 8 (5 seeds < 8, so all 5 ran concurrently)
**All 5 hashes distinct:** seed-set diversity holds; the hash-diversity gate
(COV-05) input for this scoping run is clean.

## Chosen N for Wave 2

Apply the budget formula from CONTEXT.md Claude's Discretion §"TEST-02 target wall-clock":

> aim total compute per cell ≤ ~30 min × parallelism
> `N_target × mean_wall_clock / parallelism ≤ 30 min`

With `mean_wall_clock = 94 s`, `parallelism = 8`, budget = 30 min = 1800 s:

```
N_max = 1800 × 8 / 94 ≈ 153
```

The menu of allowed N values is `{20, 18, 15, 10}` (per CONTEXT.md). Even at the
ceiling N=20, predicted wall-clock per Wave 2 cell is

```
N × mean_wall_clock / parallelism = 20 × 94 / 8 ≈ 235 s ≈ 3.9 min
```

— well under the 30-min cap. Pick the largest N from the menu.

**Chosen N for Wave 2: N = 20**

This applies to the paired-bootstrap-gated tests (TEST-03 sign-flip variance,
TEST-04 canonical menu-item variance). The sensitivity / steady-state tests
(TEST-05, TEST-06) use smaller seed counts because they compare against
empirical Inter-Quartile Range (IQR) thresholds rather than tight BCa
confidence intervals:

| Test | N | Rationale |
|---|---|---|
| TEST-03 sign-flip variance | 20 | Paired BCa CI gates verdict; max-of-menu for tightest CI |
| TEST-04 canonical variance | 20 | Same as TEST-03 |
| TEST-05 pool-number sensitivity | 5 | Per-(job, profile) IQR computation; 5 seeds gives a defensible IQR |
| TEST-06 run-length / steady-state | 10 | Per-(job, slot-length) STEADY verdict via median + IQR; D-33 default |
| TEST-07a multiplier-floor-16 companion | 5 | Qualitative finding-replicates-or-inverts; 5 seeds for sign-coherence |

## Caveats

- The runner's `manifest.json` writes `started-at-utc` and `completed-at-utc` for
  each (job, seed) at the same moment after the run completes (they differ only
  in the trailing nanosecond). Per-seed wall-clock is therefore derived from
  suite-start → seed-completion, which over-counts by the binary's cold-start
  overhead. The 89 s minimum observed seed wall-clock is the right floor estimate
  for "one seed in isolation"; the 99.5 s maximum is the right ceiling.
- All 5 seeds were under-subscribed at `-P 8` (5 < 8), so the measured
  wall-clock is approximately the single-seed compute cost, not the parallel
  amortised cost. Wave 2 cells at N = 20 run in 3 parallel waves of 8/8/4 seeds,
  giving a more efficient amortisation (predicted ~3 × 94 s ≈ 282 s wall-clock
  per N=20 cell — close to the 235 s estimate above modulo cold-start).
- The 150-pool topology (TEST-05's 150-pool arm) will run somewhat slower than
  100-pool: 1.5× nodes, network gossip is approximately `O(N log N)` per slot.
  Empirical sizing absorbed into TEST-05's compute budget by capping N at 5.
- N=20 seeds × 4 paired-bootstrap cells (TEST-03 + 4 cells) + 4 baseline controls
  + N=20 seeds × 5 canonical cells (TEST-04) gives roughly 200 (job, seed) runs
  in the paired-bootstrap suites. Total predicted wall-clock at -P 8:
  ~40 min. TEST-05's 33 jobs × 5 demand profiles × 2 pool counts × 5 seeds =
  1650 runs dominate Phase-3 compute and are the load-bearing budget item.
- Phase 3 suite designation: this suite is NOT goldens-pinned per D-25. Phase 3
  outputs land in `.planning/realism-tests/`, not in
  `parameters/phase-2-sweep/suites/.goldens/`. The M5 suite goldens are
  unaffected by this scoping run or any Wave 2 suites.
- Seed=1 `pricing_event_stream.sha256` for cross-reference into the Wave 3
  hash-diversity gate: `5533f2d2a2f37d4d94ebf747352e5e780fabaa63e24ed9b57eb348d5f01d7372`.

## Abbreviations on first use

- **BCa** — Bias-corrected and accelerated (bootstrap confidence-interval method)
- **IQR** — Inter-Quartile Range
- **CIP** — Cardano Improvement Proposal
- **RB** — Ranking Block (Cardano's standard block; EBs are Endorser Blocks)
- **EIP-1559** — Ethereum Improvement Proposal 1559 (the controller pattern used
  for single-lane and per-lane chain-derived fee adjustment)
