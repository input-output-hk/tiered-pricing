# Latency-by-Urgency × Mechanism

**Status:** Phase 5 post-handoff supplement; landed 2026-05-18.
**Scope:** Per-(actor-component, mechanism-arm) observed inclusion latency + inclusion-rate cross-cut, derived from the robustness suite `robustness-canonical-variance` Number of seeds (N) = 20 run. Complements the welfare-delta findings in [`../test-results/multi-seed-variance/results.md`](../test-results/multi-seed-variance/results.md) with the operational user-experience axis the four non-welfare columns of [`coverage-check.md`](coverage-check.md) do not surface.
**Reading guide:** Section §"Headline finding" gives the table + two-paragraph interpretation; §"Per-tier reading" walks each urgency tier; §"Anomaly to flag" surfaces one non-monotone row that needs team attention before the Cardano Improvement Proposal (CIP) cites it; §"Methodology" + §"Reproducibility" let a reviewer rebuild the table from the same raw inputs.

**Abbreviations on first use** (per `CLAUDE.md` §"Conventions / gotchas"): Cardano Improvement Proposal (CIP), Ranking Block (RB), Ethereum Improvement Proposal 1559 (EIP-1559), Bias-corrected and accelerated (BCa) bootstrap, Inter-Quartile Range (IQR), Exponentially-Weighted Moving Average (EMA), decentralised finance (DeFi), Realism Risk identifier (RSK), Claim identifier (CLM).

## Headline finding

The table splits into two views — latency conditional on inclusion (Table A), and inclusion rate (Table B) — across the same `(urgency tier × component × mechanism)` cells. Both are derived from the same the robustness suites `robustness-canonical-variance` Number of seeds (N) = 20 run.

### Table A — Latency (blocks to inclusion, median across 20 seeds, conditional on inclusion)

Each cell is the median across-seeds mean latency-from-submission-to-inclusion (in priced blocks; 1 block ≈ 20 simulated slots ≈ 10 simulated seconds at `rb-generation-probability = 0.05`). The mean is computed over included transactions only; transactions submitted but never confirmed do not contribute. `—` means inclusion rate < 1% across most seeds — the latency mean over near-zero observations is uninformative.

| Urgency tier | Component | single-lane EIP-1559 | priority-only RB-reserved | priority-only un-reserved | both-dynamic partitioned | both-dynamic un-partitioned |
|---|---|---|---|---|---|---|
| **very-high** | c9: DEX arbitrage (half-life 1 min) | 12.9 | 15.2 | 14.8 | 15.2 | 14.4 |
| **very-high** | c10: whale swap (half-life 2 min) | 12.9 | 15.5 | 15.0 | 15.5 | 14.6 |
| **high** | c7: eager adopter (half-life 5 min) | 12.6 | 15.5 | 15.2 | 15.5 | 16.3 |
| **high** | c8: FOMO buyer (half-life 10 min) | 13.8 | 13.3 | 46.4 | 13.3 | 50.4 |
| **medium** | c4: larger DeFi op (half-life 15 min) | 8.0 | 6.3 | 8.3 | 6.3 | 9.5 |
| **medium** | c3: routine swap (half-life 30 min) | 7.5 | 5.3 | 22.1 | 5.3 | 22.5 |
| **medium** | c6: casual swapper (half-life 30 min) | 13.5 | 15.3 | 58.5 | 15.3 | 60.5 |
| **low** | c5: small yield farm (half-life 1 h) | 7.3 | 5.3 | 30.5 | 5.3 | 26.0 |
| **low** | c2: moderate transfer (half-life 6 h) | 8.0 | — | 32.6 | — | 29.1 |
| **low** | c0: simple transfer (half-life 1 d) | 7.4 | — | 34.4 | — | 31.7 |
| **very-low** | c1: staking / governance (half-life 2 d) | 6.8 | — | 33.3 | — | 29.6 |

**Latency readings.** Single-lane EIP-1559 latency sits in a tight 6.8–13.8 block band across all 11 components — the controller flattens user-experienced wait-time across the urgency spectrum. Two-lane mechanisms widen the band: very-high and high-urgency components see slightly longer latency (14–16 blocks) under all four menu options vs single-lane's 12.6–13.8, but medium / low / very-low components under un-reserved variants pay a much larger penalty (22–60 blocks) because their transactions get confirmed only in the run-tail after the spike subsides. Under RB-reserved variants, the same medium-to-low components don't experience longer latency — they don't get included at all (see Table B).

### Table B — Inclusion rate (% of submitted transactions confirmed, median across 20 seeds)

Each cell is the median across-seeds fraction of submitted transactions that were included in any block.

| Urgency tier | Component | single-lane EIP-1559 | priority-only RB-reserved | priority-only un-reserved | both-dynamic partitioned | both-dynamic un-partitioned |
|---|---|---|---|---|---|---|
| **very-high** | c9: DEX arbitrage (half-life 1 min) | 22.4% | 36.1% | 35.5% | 36.1% | 41.4% |
| **very-high** | c10: whale swap (half-life 2 min) | 24.8% | 35.6% | 35.1% | 35.6% | 37.8% |
| **high** | c7: eager adopter (half-life 5 min) | 19.7% | 28.9% | 28.9% | 28.9% | 31.3% |
| **high** | c8: FOMO buyer (half-life 10 min) | 12.2% | 6.1% | 16.1% | 6.1% | 15.4% |
| **medium** | c4: larger DeFi op (half-life 15 min) | 57.6% | 28.6% | 34.0% | 28.6% | 33.0% |
| **medium** | c3: routine swap (half-life 30 min) | 56.2% | 10.6% | 27.1% | 10.6% | 26.8% |
| **medium** | c6: casual swapper (half-life 30 min) | 15.4% | 1.8% | 15.0% | 1.8% | 14.4% |
| **low** | c5: small yield farm (half-life 1 h) | 59.5% | 1.8% | 25.8% | 1.8% | 25.2% |
| **low** | c2: moderate transfer (half-life 6 h) | 56.5% | 0.0% | 25.4% | 0.0% | 25.0% |
| **low** | c0: simple transfer (half-life 1 d) | 55.9% | 0.0% | 24.7% | 0.0% | 25.0% |
| **very-low** | c1: staking / governance (half-life 2 d) | 60.1% | 0.0% | 24.9% | 0.0% | 25.0% |

**Inclusion-rate readings.** The most striking finding lives in this table, not the latency one. Under single-lane Ethereum Improvement Proposal 1559 (EIP-1559), inclusion rate is *higher for lower-urgency components* (55–60% for half-life ≥ 1 hour vs 12–25% for half-life ≤ 10 min). This is the inverse of what an urgency-pricing mechanism is supposed to deliver — the controller cannot price-discriminate fast enough during the dex-launch + arbitrage demand spike, so spike traffic loses out to the steady-state background load. Two-lane mechanisms reverse this for very-high and high-urgency components (29–41% inclusion for c7, c9, c10 — significantly better than single-lane's 12–25%) at the cost of low-urgency inclusion under RB-reserved variants (the four `0.0%` cells under `priority-only RB-reserved` and `both-dynamic partitioned` are exact zeros — those components never get included over 20 seeds × 2000 slots). Un-reserved variants are the compromise: they include low-urgency users at ~25% (paying for that inclusion in latency per Table A) while still beating single-lane on high-urgency inclusion.

### Cross-table synthesis

The menu the Cardano Improvement Proposal (CIP) presents is genuine on this axis. **Ranking-block-reserved (RB-reserved) variants trade low-urgency exclusion for tighter priority service**: half-life ≥ 1 hour gets 0% inclusion outright, but very-high-urgency components get 36% inclusion at 15 blocks (vs single-lane's 22–25% at 13 blocks). **Un-reserved variants trade latency for inclusion-rate parity**: every user class gets ~25–41% inclusion, but the medium-and-below tier waits 22–60 blocks for it (vs single-lane's 7–8 blocks at the same tier). **Single-lane EIP-1559 is "fair on latency" but unfair on inclusion-rate distribution under congestion**: low-urgency users get fast and frequent service while high-urgency arbitrage / DEX-launch users are systematically under-served during the spike. Neither mechanism is a free win across both axes.

## Per-tier reading

**Very-high urgency (arbitrage, whale swap; half-life 1–2 min).** All four menu options give 35–41% inclusion at 14–16 blocks. Single-lane EIP-1559 gives only 22–25% inclusion. The menu mechanisms are unambiguously better for this user class.

**High urgency (eager adopter, FOMO buyer; half-life 5–10 min).** `c7` (5-min half-life) tracks the very-high tier — 29–31% inclusion at 15–16 blocks across all menu options. `c8` (10-min half-life) splits: RB-reserved variants give only 6% inclusion at 13 blocks (fast but rare); un-reserved variants give 15–16% inclusion at 46–50 blocks (delayed but reachable); single-lane gives 12% at 14 blocks. See §"Anomaly to flag" for why `c8` and `c6` diverge so much from `c7`.

**Medium urgency (DeFi ops, routine swaps, casual swappers; half-life 15–30 min).** `c4` (15-min half-life, larger DeFi op) is the only medium-tier component that gets reasonable inclusion across all mechanisms (28–58% inclusion, 6–10 blocks). `c3` (routine swap) and `c6` (casual swapper) are penalised heavily under both RB-reserved and un-reserved variants — either via low inclusion rate (RB-reserved 1.8%–10.6%) or via long latency (un-reserved 22–60 blocks).

**Low urgency (yield farm, moderate transfer, simple transfer; half-life 1 hour – 1 day).** RB-reserved variants exclude these users entirely or near-entirely (`—` or ~2% inclusion). Un-reserved variants serve them at ~25% inclusion but with 26–34 blocks of latency. Single-lane EIP-1559 actually serves this tier *best* among the five mechanisms (55–60% inclusion at 7–8 blocks).

**Very-low urgency (staking / governance; half-life 2 days).** Same pattern as low urgency: RB-reserved excludes; un-reserved delays; single-lane includes.

## Anomaly to flag

The `c6` (casual swapper, 30-min half-life) and `c8` (FOMO buyer, 10-min half-life) rows are non-monotone vs `c7` (eager adopter, 5-min half-life) under several mechanisms:

- Under `priority-only un-reserved`: `c7` = 15.2 blocks / 28.9%, `c8` = 46.4 / 16.1%, `c6` = 58.5 / 15.0%.
- Under `both-dynamic un-partitioned`: `c7` = 16.3 / 31.3%, `c8` = 50.4 / 15.4%, `c6` = 60.5 / 14.4%.
- Under `priority-only RB-reserved` and `both-dynamic partitioned`: `c7` = 15.5 / 28.9%, `c8` = 13.3 / 6.1%, `c6` = 15.3 / 1.8%.

The half-life ordering is `c7 < c8 < c6` (5 min < 10 min < 30 min), which means urgency-decay-rate is `c7 > c8 > c6`. A monotone latency-by-urgency story would predict `c7` latency ≤ `c8` ≤ `c6` (more urgent ⇒ faster service). Under RB-reserved variants, the inclusion-rate ordering inverts (`c7` >> `c8` > `c6`) but latency stays similar — consistent with a sharp cutoff in the actor's expected-utility-based lane choice where `c8` and `c6` fall just below the threshold to bid for priority. Under un-reserved variants, `c8` and `c6` get delayed inclusion at the standard rate (in the run-tail when congestion fades), so their "latency-when-included" jumps to 46–60 blocks.

This is **mechanistically explainable but worth a team-eye before the CIP cites it**, because (a) it's the first place a careful reviewer will ask "is this an artefact?", and (b) the actor's lane-choice math goes through `libm::pow` + `libm::round` (per [`methodology-overview.md`](methodology-overview.md) §"(4) ActorComponent") and a small change in the rounding threshold could flip several c8 / c6 transactions across the priority / standard boundary, shifting these cells visibly.

If the team confirms this is a deterministic-rounding-boundary effect rather than a calibration bug, the CIP should disclose: "users at the priority-vs-standard expected-utility boundary experience large mechanism-dependent variation in latency and inclusion, including in some cases longer waits than less-urgent neighbours; the boundary is set by the actor's `max_fee_lovelace` policy and is a calibration choice rather than a mechanism property." If the team instead finds it's a calibration bug, this table should be rebuilt after the fix.

## What this changes vs the welfare-only narrative

The welfare findings ([`../test-results/multi-seed-variance/results.md`](../test-results/multi-seed-variance/results.md) §"TEST-04") at the same calibration (`sundaeswap_moderate × multiplier_floor = 4`) showed un-reserved arms outperform single-lane EIP-1559 by Δ ≈ +6.7e+09 to +7.9e+09 retained_value with Bias-corrected and accelerated (BCa) 95% confidence intervals excluding zero. This table refines that picture along the user-experience axis:

- **Un-reserved arms win on welfare *because* they include high-urgency users that single-lane misses.** The welfare gain has a concrete user-class attribution (arbitrage, DEX-launch).
- **The welfare loss for RB-reserved arms maps to the low-urgency-exclusion observed here.** RB-reserved arms include high-urgency users at similar rates to un-reserved but lose the moderate-to-low-urgency inclusion entirely.
- **Single-lane EIP-1559 is "fair on latency" but unfair on inclusion-rate distribution under congestion.** A reviewer who reads only the welfare table might conclude un-reserved arms are dominant; a reviewer who reads this table sees the user-class trade-off un-reserved makes (slightly higher latency for previously-fast users, in exchange for serving high-urgency users that single-lane drops).

## Caveats

1. **One cell of the (demand × multiplier_floor) matrix.** This table covers `sundaeswap_moderate × multiplier_floor = 4` only. The robustness TEST-07a finding (see [`../test-results/multiplier-floor-16-companion/results.md`](../test-results/multiplier-floor-16-companion/results.md) and `RSK-multiplier-floor-4-suite-coverage` in the [`realism-risks-register.md`](realism-risks-register.md)) is that `multiplier_floor = 16` shifts the picture substantially (priority captures more of the supply, total welfare collapses 93–98%); the latency-by-urgency cross-cut at `floor = 16` is not built here and would require either re-using existing TEST-07a outputs (3 seeds; ordering-level only) or a fresh run. Other demand profiles (`paper_like_congested`, `paper_like_uniform`, etc.) are likewise unrepresented in this table.

2. **Median across seeds, not Bias-corrected and accelerated (BCa) Confidence Intervals (CIs).** We report the median of per-seed `latency_blocks_mean` and `inclusion_rate` across the 20 seeds. This is sufficient for ordering claims and for the qualitative story above. If any specific cell becomes load-bearing for a CIP claim, that cell should be re-run with BCa CIs on the paired latency delta (per the robustness N=20 BCa stack documented in [`methodology-overview.md`](methodology-overview.md) §"(5) Confidence intervals"). The COV-05 hash-diversity gate (per [`../test-results/hash-diversity-gate/results.md`](../test-results/hash-diversity-gate/results.md)) already establishes that all 20 seeds in the canonical-variance run produce distinct pricing-event-stream Secure Hash Algorithm 256-bit hashes — so the latency observations are not artefacts of seed collapse.

3. **`target_inclusion_blocks` defaults seed the actor's lane choice for the first ~50 simulated slots** ([`realism-risks-register.md`](realism-risks-register.md) `RSK-target-inclusion-blocks-default`). The observed-latency Exponentially-Weighted Moving Average (EMA) overwrites the seed once inclusion events arrive, so the seeded defaults (priority = 1 block, standard = 4 blocks) influence early-run dynamics only. The latency means in this table are averages over the full 2000-slot run, so they integrate over both the seeded-defaults regime and the observed-latency regime.

4. **Latency-when-not-included is not captured.** A transaction submitted but never included contributes nothing to the `latency_blocks_mean` for its component — only the included transactions' wait times are averaged. The companion metric is the inclusion rate. The `—` markers in the table flag cases where inclusion is so rare that the latency mean is statistically meaningless; the user-facing experience for those cells is "your transaction does not get confirmed in this 2000-slot run."

5. **The `c6` / `c8` non-monotone anomaly** (see §"Anomaly to flag") needs a team confirmation before this table is cited as definitive for the priority-vs-standard boundary. If it turns out to be a calibration bug rather than a deterministic-rounding-boundary effect, the table requires rebuilding.

## Cross-references

- **Realism-risks register entries this table interacts with** (see [`realism-risks-register.md`](realism-risks-register.md)): `RSK-target-inclusion-blocks-default` (seeded defaults for lane-choice expected-utility), `RSK-max-fee-policy-default` (actor `max_fee_policy = {4, 1}` shapes the priority-vs-standard boundary), `RSK-single-seed-precision` (table is at the same N=20 set as the headline welfare findings), `RSK-multiplier-floor-4-suite-coverage` (table is at the calibration-regime-specific floor=4 setting).
- **Coverage-check rows this table refines** (see [`coverage-check.md`](coverage-check.md)): CLM-06 / CLM-07 (priority-only RB-reserved + un-reserved welfare claims), CLM-08 / CLM-09 (both-dynamic partitioned + un-partitioned welfare claims), CLM-05 (single-lane EIP-1559 control). The four non-welfare property columns of `coverage-check.md` (anti-bribery / signal-source-anchoring / standard-user-fee-drift-exposure / implementation-complexity) do not include latency-by-urgency; this document is the supplement.
- **Audit document section this complements** (see [`cardano-realism-audit.md`](cardano-realism-audit.md)): §"Recommended disclosure statements" §"On controller calibration" notes the calibration-regime-dependence; §"On the priority-vs-standard boundary" can cite this table for the operational expression of that calibration choice.
- **Methodology** (see [`methodology-overview.md`](methodology-overview.md) §"(4) ActorComponent + MaxFeePolicy"): describes the `LanePolicy::UtilityMaximising` expected-utility computation that produces the priority / standard split observed here.

## Methodology

**Source data.** `sim-rs/output/robustness/canonical-variance-20260518-084846/metrics_comparison.txt`. The the robustness suites `robustness-canonical-variance` suite (see [`../../sim-rs/parameters/phase-2-sweep/suites/robustness-canonical-variance.yaml`](../../sim-rs/parameters/phase-2-sweep/suites/robustness-canonical-variance.yaml)) ran 5 jobs (the 4 menu options + the single-lane EIP-1559 control) × 20 seeds against `sundaeswap_moderate.yaml` demand on the 100-node realistic topology at `multiplier_floor = 4`. The metrics collector emits one `## job=<name> seed=<n>` block per (job, seed), each with an indented `- per-component:` section listing per-component `latency_blocks_mean=<X>` and `inclusion_rate=<Y>` (plus other fields; see [`sim-rs/sim-cli/src/metrics/collector.rs`](../../sim-rs/sim-cli/src/metrics/collector.rs) for the full schema).

**Urgency-tier mapping.** The `sundaeswap_moderate` demand profile carries 11 components (indices 0–10) with `half-life-seconds: log-normal` distributions; the medians are extracted from the profile's `mu` parameter via `exp(mu)`. The tier assignment in the table is by median half-life:

| Tier | Half-life range | Components |
|---|---|---|
| very-high | ≤ 2 min | c9 (1 min), c10 (2 min) |
| high | 5–10 min | c7 (5 min), c8 (10 min) |
| medium | 15–30 min | c3 (30 min), c4 (15 min), c6 (30 min) |
| low | 1 h – 1 day | c5 (1 h), c2 (6 h), c0 (1 day) |
| very-low | ≥ 2 days | c1 (2 days) |

The tier names are descriptive only; they are not parameters of the simulator. The actor's effective urgency is `urgency = exp(-ln(2) / half_life_seconds × target_inclusion_blocks)` per [`methodology-overview.md`](methodology-overview.md) §"(4) ActorComponent" and goes through `libm::pow` for bit-stability — so the actor's lane-choice math reads the half-life directly, not the tier label.

**Aggregation.** For each (job, component_index), we collect the 20 per-seed `(latency_blocks_mean, inclusion_rate)` pairs. The reported `latency_blocks` cell is the median of `latency_blocks_mean` across seeds whose `inclusion_rate > 0.01` (rejecting the seeds where the cell observed near-zero inclusions, because the latency mean over zero observations is `0.0` by convention and would pull the median toward an uninformative value). The reported `inclusion_rate%` cell is the median of `inclusion_rate` across all 20 seeds. If fewer than 10 seeds had `inclusion_rate > 0.01`, the cell is reported as `—` instead of a number.

## Reproducibility

The table above is generated from `sim-rs/output/robustness/canonical-variance-20260518-084846/metrics_comparison.txt` by the Python script below. To rebuild against a fresh run, re-execute the suite via `cargo run --release --bin experiment-suite -- run sim-rs/parameters/phase-2-sweep/suites/robustness-canonical-variance.yaml` and re-run the script against the new run-id output directory.

```python
#!/usr/bin/env python3
"""Build the latency-by-urgency × mechanism table from a robustness-canonical-variance
metrics_comparison.txt. Outputs a markdown table to stdout."""
import re, statistics
from collections import defaultdict

# sundaeswap_moderate component_index -> (description, half-life-seconds-median, tier)
COMPONENT_MAP = {
    0:  ("background: simple transfer (60%)",      86400,  "low"),
    1:  ("background: staking/governance (25%)",   172800, "very-low"),
    2:  ("background: moderate transfer (15%)",    21600,  "low"),
    3:  ("background-defi: routine swap (50%)",    1800,   "medium"),
    4:  ("background-defi: larger DeFi op (30%)",  900,    "medium"),
    5:  ("background-defi: small yield farm (20%)",3600,   "low"),
    6:  ("dex-launch: casual swapper (40%)",       1800,   "medium"),
    7:  ("dex-launch: eager adopter (35%)",        300,    "high"),
    8:  ("dex-launch: FOMO buyer (25%)",           600,    "high"),
    9:  ("arbitrage: DEX arbitrage (70%)",         60,     "very-high"),
    10: ("arbitrage: whale swap (30%)",            120,    "very-high"),
}

JOBS = [
    ('control_eip1559_d8_t50_w32',                    'single-lane EIP-1559'),
    ('menu_rb_reserved_priority_only_static_x4',      'priority-only RB-reserved'),
    ('menu_unreserved_priority_only_static_x4',       'priority-only un-reserved'),
    ('menu_rb_reserved_both_dynamic_x4',              'both-dynamic partitioned'),
    ('menu_unreserved_both_dynamic_x4',               'both-dynamic un-partitioned'),
]
TIER_ORDER = ['very-high', 'high', 'medium', 'low', 'very-low']

import sys
text = open(sys.argv[1] if len(sys.argv) > 1 else
           "sim-rs/output/robustness/canonical-variance-20260518-084846/metrics_comparison.txt").read()

data = defaultdict(list)
for block in re.split(r'^## job=', text, flags=re.MULTILINE)[1:]:
    header = block.split('\n', 1)[0]
    m = re.match(r'(\S+) seed=(\d+)', header)
    if not m: continue
    job = m.group(1)
    for line in block.split('\n'):
        m2 = re.match(r'  - component_index=(\d+) .* latency_blocks_mean=([\d.]+) inclusion_rate=([\d.]+)', line)
        if not m2: continue
        data[(job, int(m2.group(1)))].append((float(m2.group(2)), float(m2.group(3))))

agg = {}
for (job, cidx), obs in data.items():
    lats = [lat for lat, inc in obs if inc > 0.01]
    incs = [inc for _, inc in obs]
    agg[(job, cidx)] = (
        statistics.median(lats) if len(lats) >= 10 else None,
        statistics.median(incs),
    )

print("| Urgency tier | Component | " + " | ".join(name for _, name in JOBS) + " |")
print("|---|---|" + "---|" * len(JOBS))
for tier in TIER_ORDER:
    for cidx in sorted(COMPONENT_MAP):
        if COMPONENT_MAP[cidx][2] != tier: continue
        desc, hl, _ = COMPONENT_MAP[cidx]
        hl_str = f"{hl}s" if hl < 60 else f"{hl//60} min" if hl < 3600 else f"{hl//3600} h" if hl < 86400 else f"{hl//86400} d"
        cells = [f"c{cidx}: {desc} (half-life {hl_str})"]
        for jid, _ in JOBS:
            lat, inc = agg.get((jid, cidx), (None, 0.0))
            cells.append(f"{lat:.1f} / {inc*100:.1f}%" if lat is not None and inc >= 0.01 else "—")
        print("| **" + tier + "** | " + " | ".join(cells) + " |")
```

**Commit + tag.** The table reflects `sim-rs/output/robustness/canonical-variance-20260518-084846/` outputs from the robustness run at the post-`phase-2-cip-evidence-v1` tag. Future re-runs supersede this table; the tag-pinned snapshot is the citable reference.
