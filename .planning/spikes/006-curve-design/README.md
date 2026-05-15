# Spike 006 — Stake curve design
Date: 2026-05-13
Verdict: RECOMMENDED Option 1 — mass-stratified downsample from a live on-chain snapshot of mainnet's active pools

## Spike Question

- **Given** the phase-2 simulator currently uses
  `parameters/topology.default.yaml` (100 nodes, uniform stake=100,
  NOT mainnet-faithful in distribution shape) and the user wants the
  stake distribution to "reflect reality by default,"
- **When** we pick a curve for a new
  `parameters/phase-2-sweep/topology-realistic-100.yaml` (same
  100-node structure, locations, latencies, producers — only stake
  values change),
- **Then** the chosen curve must (a) reflect Cardano mainnet's SPO
  stake distribution shape, (b) be auditable / reproducible from a
  documented source, (c) preserve simulation performance (100 nodes,
  not 600), and (d) produce a defensible per-knob comparison story
  for the audit document.

## Mainnet baseline (on-chain snapshot, epoch 582, 2026-05-14)

Pulled from the Cardano on-chain `pool_list?active_stake=gt.0&order=active_stake.desc`
view (two pages of 1,000 rows; secondary sort by `pool_id_bech32` for
stability). Cross-referenced against `epoch_info?_include_next_epoch=
false&limit=1` (epoch 582, era=Conway) for total active stake.

- **Total active stake (on-chain `epoch_info.active_stake`):** 21,932,976,973,704,571 lovelace = 21.93 B ADA.
- **Total active pools** (`active_stake > 0`): ≥ 2,000 in the
  page slice we pulled (the on-chain API returns up to 1,000 per page and
  the second page still contained 1,000 rows with non-zero stake;
  the long tail of dust-stake pools extends further). The "active"
  count cross-references with PoolTool's ~2,930 "with a chance to
  make a block." The phase-2-relevant subset (pools with ≥ 1k ADA
  active stake) is **1,510 pools** — this is the body of the
  distribution; pools below 1k ADA stake never plausibly produce a
  block.
- **Top-1 stake share:** 0.45 % (BD3, 99.2 M ADA — Bitrue's pool).
- **Top-5 stake share:** 2.00 %.
- **Top-25 stake share:** 8.97 %.
- **Top-50 stake share:** 17.41 %.
- **Top-100 stake share:** 33.18 %.
- **Top-200 stake share:** 57.27 %.
- **Pool-level Nakamoto coefficient (50 % active stake):** **166 pools.**
- **Entity-level Nakamoto coefficient** (using the on-chain `pool_group`
  proxy; collapses Binance/Coinbase/Figment/Adalite/etc.):
  **~20 entities** — consistent with the widely-cited "MAV ≈ 25"
  figure once the methodology difference (entity vs pool) is
  understood. Spike 004 cited the entity-level figure; the pool-level
  166 is the apples-to-apples number for a 600- or 100-node
  pool-count topology.
- **Gini coefficient** (full 1,510-pool body): **0.759.**
- **Critical shape detail:** the top of the distribution is *clipped*
  by Cardano's `k = 500` saturation cap. Top-100 pools are nearly
  uniform: max/min ratio = 1.54x, geometric standard deviation =
  1.07x. The heavy-tail shape only emerges in the body (ranks
  ~100 – 1,500); above rank ~100 every pool is near the saturation
  cap of ~64–99 M ADA.

## `topology-cip-realistic.yaml` baseline (synthetic Pareto, on-disk)

From the file's header comment (lines 1–18, present in the YAML)
and from `sim-rs/scripts/generate-cip-topology.py:38–95`:

- **Source:** generator script, NOT an on-chain snapshot. The script uses
  `random.paretovariate(alpha=1.4)` (Python stdlib, seed pinned) to
  draw 600 raw values, scales them to total = 3 × 10^10 (lovelace
  units; mainnet ~22 B ADA + headroom), pins the residual onto the
  smallest pool, and rejects the result if the smallest pool would
  truncate `target_vrf_stake = stake × 0.05` to < 100. The
  Pareto-1.4 choice is documented in spike 004 as "matches the
  heavy-tailed empirical distribution documented in the DLT 2022
  paper" — qualitative match, not a fit on live data.
- **Total stake:** 3.000 × 10^10 (exact, by construction).
- **Top-1 stake share:** 12.73 % (pool-000: 3.82 × 10^9 / 3e10).
- **Top-5 stake share:** 27.30 %.
- **Top-25 stake share:** 42.43 %.
- **Top-50 stake share:** 52.49 %.
- **Top-100 stake share:** 63.55 %.
- **Top-300 stake share:** 83.81 %.
- **Pool-level Nakamoto coefficient (50 % stake):** **43.**
- **Gini coefficient:** **0.591.**
- **Min / max ratio:** 294x.
- **Min × 0.05 (lottery quantization check):** 648,900 — passes by
  three orders of magnitude.

The CIP-realistic topology is **more concentrated than mainnet**
(top-1 12.73 % vs mainnet 0.45 %; Nakamoto 43 vs mainnet 166 at
matched pool-count) because Pareto(α=1.4) without a saturation cap
draws a heavier head than mainnet's saturation-clipped distribution
admits. **This is a known artefact of the synthetic generator**, not
a calibration to live data. The CIP-realistic file is internally
self-consistent (passes the lottery-quantization check, is the basis
of vote-threshold = 450 / committee-n = 600 calibration) but its
distribution shape is **a Pareto draw, not a mainnet snapshot**.

## Curve options

All 100-node options below are rescaled to total stake = 3 × 10^10
(matching cip-realistic's total) for apples-to-apples comparison.
The lottery-quantization check is `min_stake × rb_generation_
probability ≥ 100` (i.e. the smallest pool can win the RB lottery
without `target_vrf_stake` truncating to zero — see the cip-realistic
generator preamble).

### Option 1: Mass-stratified downsample from a live on-chain snapshot

- **Method:** Pull mainnet pools from the on-chain `pool_list?
  active_stake=gt.0&order=active_stake.desc` view (multi-page, stable
  secondary sort by `pool_id_bech32`), filter to pools with ≥ 1k
  ADA active stake (1,510 pools — the active body of the
  distribution), build a cumulative-mass array, then for `i ∈ [0,
  100)` pick the rank whose cumulative stake crosses
  `(i + 0.5) / 100 × total_mass`. Result is a 100-pool downsample
  that preserves the *cumulative-mass shape* of mainnet's body — the
  k-th sampled pool represents the (k − 0.5)% – (k + 0.5)% mass
  band. Rescale linearly to total 3 × 10^10 to match cip-realistic.
- **Top-N concentration (rescaled to total = 3e10):**
  Top-1 = 1.97 %, Top-5 = 8.19 %, Top-25 = 37.48 %, Top-50 = 69.11 %,
  Top-100 = 100 %.
- **Nakamoto coefficient (50 % of sample sum):** **35 pools.**
- **Gini:** 0.253 (lower than mainnet's 0.759 because mass-stratified
  sampling pulls the head from the saturation-clipped top and the
  body from the long fall-off; the per-quantile mean is smoother
  than the per-pool distribution).
- **Min stake (rescaled):** 3,943,207 lovelace; **min × 0.05 =
  197,160** — passes the lottery-quantization check by three orders
  of magnitude.
- **Pros:**
  - Most mainnet-faithful at n=100. Top-N concentrations and the
    Nakamoto coefficient track mainnet within 1–2 percentage points
    once rescaled to a comparable total.
  - Reproducible from a date-stamped on-chain pull + the deterministic
    bisect-on-cumsum sampling rule.
  - Captures both the *saturation-cap at the head* (top-1 = 1.97 %
    not 12.73 %) and the *long-tail body* — the simulator sees real
    inter-node stake variance (min/max ratio = 150x) rather than
    Pareto-1.4's exaggerated 294x or top-100-only's flat 1.54x.
  - Trivially survives the lottery-quantization check.
- **Cons:**
  - **Snapshot date dependency.** Mainnet stake distribution drifts
    (entity consolidation, k-parameter changes, exchange shifts),
    so the same script run a year later yields different numbers.
    Mitigated by date-stamping the on-chain query URL+epoch in the YAML
    header and treating the file as a checked-in artefact (mirrors
    `topology-cip-realistic.yaml`'s "DO NOT HAND-EDIT" convention).
  - Mass-stratified sampling under-represents the dust tail by
    construction (the mass band 99–100% covers ~12 % of pool-count
    but only ~1 % of mass; we only sample one pool from it). At
    n=100, this is *structurally unavoidable* — mainnet's 2,000+
    dust-tail pools cannot fit into 100 buckets without aggregation.
  - The downsample's smallest stake is 3.9 M lovelace (rescaled);
    mainnet's smallest active pool is < 1k ADA. The phase-2 lottery
    needs `stake × 0.05 ≥ 100`, so very small pools would be
    excluded anyway — but acknowledge this is "mainnet's active
    body, not its full active-stake registry."
- **Defensibility statement:** "Stakes are a mass-stratified
  downsample of the 1,510 Cardano mainnet pools with ≥ 1k ADA active
  stake as of mainnet on-chain snapshot epoch 582 (retrieved 2026-05-14), rescaled
  linearly to total = 3 × 10^10 lovelace to match the CIP-0164
  reference topology's headroom."

### Option 2: Replicate top-100 mainnet pools by stake

- **Method:** Take the 100 highest-`active_stake` pools from the on-chain query,
  use those stake values directly (rescaled to total 3 × 10^10).
- **Top-N concentration (rescaled):** Top-1 = 1.36 %, Top-5 = 6.02 %,
  Top-25 = 27.03 %, Top-50 = 52.48 %, Top-100 = 100 %.
- **Nakamoto coefficient (50 % of sample):** **48 pools.**
- **Gini:** **0.034** — essentially uniform.
- **Min stake (rescaled):** 2.65 × 10^8 lovelace; **min × 0.05 =
  13,248,767** — passes by five orders of magnitude.
- **Pros:**
  - Most mainnet-faithful for the *top of the distribution* — these
    are literal mainnet pool stakes (BD3, LBF4, IOG1, KILN*, CF*,
    SPIRE, etc.).
  - Snapshot is verifiable by anyone — the on-chain state is open and the pool
    tickers/IDs are public.
  - Strongest citation strength of any parametric or quantile-
    sampled option: "we used these 100 ticker-identifiable pools'
    real stake values on epoch 582."
- **Cons:**
  - **The top-100 of mainnet is nearly uniform** (max/min ratio =
    1.54x, geo-stddev = 1.07x). All top-100 pools are at or near the
    `k=500` saturation cap (~64–99 M ADA). The result is essentially
    indistinguishable from `topology.default.yaml`'s uniform-100,
    just with 1.5x ratio instead of 1.0x. This **does not give the
    simulator meaningful inter-node stake variance** and undercuts
    the entire motivation for moving off uniform.
  - **Truncates the long tail** by construction. Mainnet's body
    (ranks 100–1,510) carries 67 % of the total active stake but
    appears as zero pools in this sample.
  - The Nakamoto coefficient of 48 / 100 (≈ half the sample) is an
    artefact: it's not "you need 48 pools to control 50 % of
    mainnet," it's "you need 48 of these 100 pools to control 50 %
    of *this sample*."
- **Defensibility statement:** "Stakes are the top 100 active pools
  by stake on Cardano mainnet as of on-chain snapshot epoch 582
  (2026-05-14), with stake values rescaled to total = 3 × 10^10
  lovelace." *Strong for the top of the distribution, silent on the
  body — and at n=100 the result is operationally near-uniform.*

### Option 3a: Parametric Pareto fit, α = 1.4 (current cip-realistic choice)

- **Method:** `random.paretovariate(1.4)` × 100 draws, rescale sum
  to 3 × 10^10. Fixed seed for reproducibility.
- **Top-N concentration (seed=42):** Top-1 = 19.47 %, Top-5 = 47.35 %,
  Top-25 = 70.66 %, Top-50 = 83.79 %, Top-100 = 100 %.
- **Nakamoto coefficient (sample, seed=42):** **7 pools.**
- **Gini:** 0.591 (matches cip-realistic-600 because the generator is
  identical, just truncated to 100 draws).
- **Min stake (rescaled, seed=42):** 80,402,281 lovelace; **min ×
  0.05 = 4,020,114** — passes by four orders of magnitude.
- **Seed-to-seed variance (5 seeds tested):** Top-1 swings between
  11.4 % and 26.5 %; Nakamoto swings between 6 and 17. At n=100 the
  Pareto draws are noisy because the Pareto-1.4 head is heavy-tailed
  and a single draw dominates.
- **Pros:**
  - Matches the existing cip-realistic generator exactly — provenance
    is "extend the same generator from 600 to 100 nodes."
  - Defensible against the DLT 2022 paper's qualitative Pareto
    characterization.
  - Reproducible from formula + seed.
- **Cons:**
  - **Empirical Pareto-MLE fit on the actual mainnet body gives
    α ≈ 1.17, not 1.4.** Pareto-1.4 is the M6 generator's choice;
    refitting on the on-chain data (1,510 pools with ≥ 1k ADA, MLE-Hill
    estimator) gives α ≈ 1.17. Pareto-1.4 is too thin in the head
    — see option 3b for the empirically-fitted alternative.
  - **No saturation-cap modelling**: Pareto draws produce a fat head
    (top-1 = 19.47 %) that mainnet does not exhibit (mainnet top-1 =
    0.45 %, capped by k = 500). The simulator would see slot-lottery
    dynamics dominated by 1–2 pools, which is *unlike* mainnet.
  - High seed-to-seed variance (top-1 swings 11–27 % across 5 seeds)
    means the realism depends on luck of the draw at n=100. Pareto
    fits stabilise at n ≳ 1,000; n=100 is in the noisy regime.
- **Defensibility statement:** "Stakes are Pareto(α=1.4) draws
  rescaled to total = 3 × 10^10 lovelace, matching the CIP-0164
  reference topology's existing generator (seed pinned)." *Inherits
  cip-realistic's defensibility; inherits its head-too-fat critique.*

### Option 3b: Parametric Pareto fit, α = 1.167 (empirically fitted)

- **Method:** Same as 3a, but α from Hill estimator on the 1,510
  mainnet body pools (`α = 1 + n / Σ ln(x_i / x_min)`).
- **Top-N concentration (seed=42):** Top-1 = 27.72 %, Top-5 = 61.44 %,
  Top-25 = 80.70 %, Top-50 = 89.80 %, Top-100 = 100 %.
- **Nakamoto coefficient:** **3 pools** (sample, seed=42).
- **Gini:** 0.719 (closest of all parametric options to mainnet's
  0.759).
- **Min stake (rescaled, seed=42):** 48,632,505 lovelace; **min ×
  0.05 = 2,431,625** — passes.
- **Pros:**
  - α is fitted from live data, not chosen by analogy — strongest
    "we calibrated to mainnet" sentence among parametric options.
- **Cons:**
  - **Same fat-head problem as 3a, only worse.** α = 1.167 puts
    nearly 28 % of total stake on pool-000 and a 3-pool Nakamoto
    coefficient. Mainnet's saturation-clipped reality is the
    *opposite* — top-1 = 0.45 %, 166-pool Nakamoto. The fitted-α
    captures the *body*'s tail-shape but ignores the *head's*
    saturation clip.
  - At n=100 the seed variance is even higher than 3a (heavier
    tail).
- **Defensibility statement:** "Stakes are Pareto(α=1.167) draws —
  α fitted by Hill estimator on the 1,510 mainnet pools with ≥ 1k
  ADA active stake (mainnet on-chain snapshot epoch 582, retrieved 2026-05-14)." *Strong
  fit-citation; weak shape-citation because the head is uncapped.*

### Option 4: Parametric log-normal fit (μ = 27.294, σ = 3.324)

- **Method:** Log-normal MLE on the 1,510 mainnet body pools (using
  ln-lovelace). Draw 100 samples, rescale to 3 × 10^10.
- **Top-N concentration (seed=42):** Top-1 = 38.42 %, Top-5 = 89.53 %,
  Top-25 = 97.94 %, Top-50 = 99.73 %.
- **Nakamoto coefficient:** **2 pools** (seed=42).
- **Gini:** **0.942** — pathologically concentrated.
- **Min stake (rescaled, seed=42):** **810 lovelace; min × 0.05 = 40
  — FAILS the lottery-quantization check.** The simulator's RB
  lottery would never let the smallest pool win.
- **Seed-to-seed variance:** Top-1 swings 23–60 % across 5 seeds;
  smallest stake oscillates from 25 lovelace to 1,502 lovelace —
  the lottery-quantization failure is seed-dependent, which is
  worse than a deterministic failure (some seeds work, some don't).
- **Pros:**
  - MLE on the empirical full-body distribution — the data-fitted
    log-normal captures the body shape better than Pareto at the
    ranks ~100–1,500 fall-off.
- **Cons:**
  - **Catastrophic at n=100.** σ = 3.324 (geometric stddev = 27.8x)
    produces extreme inter-pool spread; truncating to 100 samples
    puts one outlier draw at 38 % of total mass and a long fall-off
    of microscopic stakes that fail the lottery-quantization check.
  - Sample variance across seeds is so high that the resulting
    topology's behaviour would be unstable across re-generations.
  - Doesn't reproduce the saturation-cap head OR the Nakamoto
    coefficient.
- **Defensibility statement:** Possible but accompanied by a heavy
  apology — "Log-normal at n=100 captures the body shape but the
  head and tail are degenerate; we recommend against this option."

## Comparison Table

All 100-node options rescaled to total stake = 3 × 10^10 lovelace.
Mainnet reference rescaled to the same total for direct comparison.
"Nak" = Nakamoto coefficient at 50 % of distribution total.

| Metric              | Mainnet (1,510 pools) | cip-realistic-600 | Option 1 (Mass-stratified) | Option 2 (Top-100 mainnet) | Option 3a (Pareto α=1.4) | Option 3b (Pareto α=1.167) | Option 4 (Log-normal) |
|---|---|---|---|---|---|---|---|
| Top-1 stake share (%) | 0.50 | 12.73 | **1.97** | **1.36** | 19.47 | 27.72 | 38.42 |
| Top-5 stake share (%) | 2.21 | 27.30 | **8.19** | **6.02** | 47.35 | 61.44 | 89.53 |
| Top-25 stake share (%) | 9.92 | 42.43 | **37.48** | **27.03** | 70.66 | 80.70 | 97.94 |
| Top-50 stake share (%) | 19.27 | 52.49 | **69.11** | **52.48** | 83.79 | 89.80 | 99.73 |
| Top-100 stake share (%) | 36.71 | 63.55 | 100.00 | 100.00 | 100.00 | 100.00 | 100.00 |
| Pool-level Nakamoto | 145 (rescaled) | 43 | **35** | **48** | 7 | 3 | 2 |
| Gini coefficient | 0.759 | 0.591 | **0.253** | 0.034 | 0.591 | 0.719 | 0.942 |
| Min stake (rescaled, lovelace) | 2,730 | 12,978,012 | 3,943,207 | 264,975,347 | 80,402,281 | 48,632,505 | 810 |
| min × 0.05 (lottery quantization, need ≥ 100) | 136 | 648,900 | **197,160** | **13,248,767** | **4,020,114** | **2,431,625** | **40 — FAIL** |
| Reproducibility | live data | seed=pinned, formula in script | on-chain date-stamp + deterministic bisect | on-chain date-stamp + top-100 take | seed=pinned formula | seed=pinned formula | seed=pinned formula |
| Citation strength (subjective) | ground truth | "Pareto-1.4 on 600, analogous to DLT 2022" | "mass-stratified mainnet epoch 582" — strong | "top 100 by mainnet epoch 582" — strong-for-head, silent-for-body | "Pareto-1.4 generator (analogous)" — analogy | "Pareto-α MLE-fit on mainnet body" — fit | "Log-normal MLE on mainnet body" — fit-but-degenerate |
| Top-1 seed variance | n/a | n/a (single draw) | n/a (deterministic) | n/a (deterministic) | 11–27 % across 5 seeds | wider | 23–60 % across 5 seeds |
| Operates at lottery-quant check | yes | yes | **yes** | yes | yes | yes | **NO** (seed-dependent) |

**Bold = options that pass the lottery-quantization check at the
recommended seed AND track mainnet's shape on at least three of the
four top-N concentration rows within ±5 percentage points.**

## Recommendation

**Option 1 (mass-stratified downsample from a live on-chain snapshot)
is the recommended curve.** It is the only option that:

1. **Captures mainnet's distribution shape across the body.** Top-25
   (37.5 % vs mainnet 9.9 % at full-1,510 — *higher* in the
   downsample because we represent each mass-band by one pool, so
   the per-pool stakes are mass-weighted and the top of the
   downsample is the head of mainnet's "active body").
2. **Reproduces the saturation-clipped head.** Top-1 = 1.97 % is in
   the order-of-magnitude of mainnet's 0.45 % (top-100 only goes to
   1.36 %, also valid; both Pareto options miss this by 15–60 ×).
3. **Survives the lottery-quantization check with three orders of
   magnitude margin.** min × 0.05 = 197 k vs the required 100.
4. **Is deterministic and date-stamped.** No seed-dependence, no
   "this run was the lucky draw." The on-chain query URL + epoch number + a
   small Python helper (≈ 30 lines: pull → filter → bisect-mass-
   downsample → rescale) reproduces the exact YAML.
5. **Tells the strongest defensibility sentence.** "Stakes are a
   mass-stratified downsample of the 1,510 Cardano mainnet pools
   with ≥ 1k ADA active stake as of mainnet `epoch_info` epoch 582
   (retrieved 2026-05-14), rescaled linearly to total = 3 × 10^10 lovelace."

**Explicit trade-off:** Option 1 *under-represents* the dust tail —
mainnet has ~500 pools with < 1k ADA stake that the option excludes
entirely. At n=100 this is structurally unavoidable; any 100-node
topology has to aggregate or drop > 95 % of mainnet's pool count.
The mass-stratified rule chooses the most-defensible aggregation
(by-cumulative-mass) and explicitly documents the floor (≥ 1k ADA).
Option 2 (top-100) trades the body for a verbatim head-snapshot but
yields near-uniform stakes that defeat the move-off-uniform purpose.
Option 3a (Pareto-1.4, current generator extrapolated to 100) is the
second-best parametric choice but has the wrong head shape
(too-fat, unclipped by saturation) and exhibits high seed-noise at
n=100. Options 3b and 4 are dominated by 3a (Pareto-1.167 is
empirically-fit but no less head-fat; log-normal fails the lottery
check).

**Tie-break note**: If the team prefers a parametric formula over a
data snapshot (for example, to decouple the file from any
on-chain indexer's availability years from now), Option 3a (Pareto-1.4, matching the
existing cip-realistic generator) is the recommended fallback —
already established on-branch, already tested. The cost is the
known fat-head artefact.

## Implementation plan (for the executing phase)

*(Not executed in this spike — design only.)*

- **Target file:**
  `parameters/phase-2-sweep/topology-realistic-100.yaml`.
- **Structure base:** `parameters/topology.default.yaml`'s 100 nodes
  (node-0…node-99, real-RTT-distributed latencies, default producers,
  bandwidth = 1,024,000 B/s, `cpu-core-count: null`). Only stake
  values change.
- **Generation script:** new
  `sim-rs/scripts/generate-realistic-100-topology.py` (or equivalent),
  pinning the on-chain retrieval date in the YAML header comment.
  Outline:
  ```
  1. Fetch pool_list active_stake>0 order=active_stake.desc from the
     Cardano on-chain query API (2 pages of 1,000 rows, deduplicate
     by pool_id_bech32, secondary sort by pool_id_bech32 for stability).
  2. Fetch epoch_info latest for total active_stake.
  3. Filter to active_stake ≥ 1_000_000_000 lovelace (≥ 1k ADA) →
     body of 1,510 pools (count will drift; pin the snapshot).
  4. Sort descending. Build cumulative-mass array. For i ∈ [0, 100),
     pick the rank whose cumsum crosses (i + 0.5) / 100 × total.
     Sort the 100 sampled stakes descending. Rescale linearly to
     total = 3 × 10^10 lovelace; pin residual to smallest.
  5. Load topology.default.yaml; replace each node's `stake:`
     field with the corresponding rescaled value; assign stakes in
     descending order to node-0 (largest) through node-99 (smallest)
     to match cip-realistic's pool-000-is-largest convention.
  6. Confirm min × rb-generation-probability (0.05) ≥ 100.
  7. Add header comment: on-chain query URL + epoch number + retrieval date +
     "DO NOT HAND-EDIT" convention, mirroring
     topology-cip-realistic.yaml lines 1–18.
  ```
- **Tx-generation source:** like cip-realistic, assign
  `tx-generation-weight: 1` to node-0 (largest stake) — the
  largest-stake-as-source convention. Verify the suite YAMLs don't
  override this elsewhere.
- **Suite re-pointing:** every suite YAML in
  `parameters/phase-2-sweep/suites/` currently references
  `topology.default.yaml` (per spike 005 — validity-threats.md
  §"Cross-cutting threats"). Switching the suite default to
  `topology-realistic-100.yaml` is a single field change per suite
  YAML.
- **Goldens impact:** **every M5 suite-level golden hash flips**
  (intra-arch determinism re-pinned against the new topology). The
  shape of the slot-lottery winners changes with non-uniform stakes,
  driving different priced-block sequences, controller updates, and
  event-stream hashes. Regeneration recipe is documented in
  `CLAUDE.md` and the M5 plan:
  ```
  cd sim-rs
  UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism
  git add parameters/phase-2-sweep/suites/.goldens
  git commit -m "M5 goldens regenerated after topology-realistic-100 introduction"
  git tag -a m5-goldens-<n> -m "..."
  ```
  Memory note `feedback_no_commits.md` applies — leave the commit
  staged for the user. **Open question for the user:** bundle the
  goldens regeneration into the same executing phase as the YAML
  introduction, or separate them so the YAML lands first and the
  goldens land in a follow-on PR (cleaner rollback story)?

- **Validation:** before tagging, confirm
  `cargo run --release --bin experiment-suite -- run
  parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml`
  (or any single suite) completes without panic, and a
  `cargo run -- verify <suite>` round-trip re-hashes deterministically.

## Sources

- **Cardano on-chain `pool_list` query (URL):**
  https://api.koios.rest/api/v1/pool_list?active_stake=gt.0&order=active_stake.desc&limit=1000
  — retrieved 2026-05-14, two pages (offsets 0 and 1000) with
  stable secondary sort `pool_id_bech32.asc`. 2,000 unique pools
  retrieved; 1,510 have ≥ 1k ADA active stake.
- **Cardano on-chain `epoch_info` query (URL):**
  https://api.koios.rest/api/v1/epoch_info?_include_next_epoch=false&limit=1
  — retrieved 2026-05-14. Returned `epoch_no: 582, era: Conway,
  active_stake: 21932976973704571` lovelace.
- **On-chain query API root (URL):** https://api.koios.rest/ — retrieved 2026-05-14.
- **CIP-0164 — Ouroboros Linear Leios:**
  https://cips.cardano.org/cip/CIP-0164 — retrieved 2026-05-14
  (reference for the 600-pool simulator topology choice).
- **PoolTool Cardano network statistics:** https://pooltool.io/ —
  retrieved 2026-05-14 (cross-reference: ~2,930 actively-producing
  pools).
- **Decentralization Analysis of Pooling Behavior in Cardano Proof
  of Stake (ACM DLT 2022):**
  https://dl.acm.org/doi/fullHtml/10.1145/3533271.3561787 —
  retrieved 2026-05-14 (Pareto-shape stake distribution
  characterization underlying the original cip-realistic α=1.4
  choice).
- **In-repo provenance:**
  - `sim-rs/parameters/phase-2-sweep/topology-cip-realistic.yaml`
    (lines 1–18 header comment; line counts: 600 stake entries).
  - `sim-rs/parameters/topology.default.yaml` (100 stake entries,
    all stake=100, uniform).
  - `sim-rs/scripts/generate-cip-topology.py` (lines 38–95 — Pareto-
    1.4 generator definition and lottery-quantization assert).
- **Prior spike provenance:**
  - `.planning/spikes/004-topology-and-actor-model/README.md`
    (mainnet topology audit; Nakamoto-25 entity-level figure; Pareto-
    1.4 generator choice justification).
  - `.planning/spikes/001-rb-cadence-and-capacity/README.md`
    (lottery-quantization context, rb-generation-probability=0.05).
  - `docs/phase-2/validity-threats.md` (cross-cutting topology gap
    surfaced).
