# Latency-by-Urgency × Mechanism

**Status:** Phase 5 post-handoff supplement; landed 2026-05-18.
**Scope:** Per-(actor-component, mechanism-arm) observed inclusion latency + inclusion-rate cross-cut, derived from the Phase 3 `phase-3-canonical-variance` Number of seeds (N) = 20 run. Complements the welfare-delta findings in [`../test-results/multi-seed-variance/results.md`](../test-results/multi-seed-variance/results.md) with the operational user-experience axis the four non-welfare columns of [`coverage-check.md`](coverage-check.md) do not surface.
**Reading guide:** Section §"Headline finding" gives the table + two-paragraph interpretation; §"Per-tier reading" walks each urgency tier; §"Anomaly to flag" surfaces one non-monotone row that needs team attention before the Cardano Improvement Proposal (CIP) cites it; §"Methodology" + §"Reproducibility" let a reviewer rebuild the table from the same raw inputs.

**Abbreviations on first use** (per `CLAUDE.md` §"Conventions / gotchas"): Cardano Improvement Proposal (CIP), Ranking Block (RB), Ethereum Improvement Proposal 1559 (EIP-1559), Bias-corrected and accelerated (BCa) bootstrap, Inter-Quartile Range (IQR), Exponentially-Weighted Moving Average (EMA), decentralised finance (DeFi), Realism Risk identifier (RSK), Claim identifier (CLM).

## Headline finding

| Urgency tier | Component | single-lane EIP-1559 | priority-only RB-reserved | priority-only un-reserved | both-dynamic partitioned | both-dynamic un-partitioned |
|---|---|---|---|---|---|---|
| **very-high** | c9: DEX arbitrage (half-life 1 min) | 12.9 / 22.4% | 15.2 / 36.1% | 14.8 / 35.5% | 15.2 / 36.1% | 14.4 / 41.4% |
| **very-high** | c10: whale swap (half-life 2 min) | 12.9 / 24.8% | 15.5 / 35.6% | 15.0 / 35.1% | 15.5 / 35.6% | 14.6 / 37.8% |
| **high** | c7: eager adopter (half-life 5 min) | 12.6 / 19.7% | 15.5 / 28.9% | 15.2 / 28.9% | 15.5 / 28.9% | 16.3 / 31.3% |
| **high** | c8: FOMO buyer (half-life 10 min) | 13.8 / 12.2% | 13.3 / 6.1% | 46.4 / 16.1% | 13.3 / 6.1% | 50.4 / 15.4% |
| **medium** | c4: larger DeFi op (half-life 15 min) | 8.0 / 57.6% | 6.3 / 28.6% | 8.3 / 34.0% | 6.3 / 28.6% | 9.5 / 33.0% |
| **medium** | c3: routine swap (half-life 30 min) | 7.5 / 56.2% | 5.3 / 10.6% | 22.1 / 27.1% | 5.3 / 10.6% | 22.5 / 26.8% |
| **medium** | c6: casual swapper (half-life 30 min) | 13.5 / 15.4% | 15.3 / 1.8% | 58.5 / 15.0% | 15.3 / 1.8% | 60.5 / 14.4% |
| **low** | c5: small yield farm (half-life 1 h) | 7.3 / 59.5% | 5.3 / 1.8% | 30.5 / 25.8% | 5.3 / 1.8% | 26.0 / 25.2% |
| **low** | c2: moderate transfer (half-life 6 h) | 8.0 / 56.5% | — | 32.6 / 25.4% | — | 29.1 / 25.0% |
| **low** | c0: simple transfer (half-life 1 d) | 7.4 / 55.9% | — | 34.4 / 24.7% | — | 31.7 / 25.0% |
| **very-low** | c1: staking / governance (half-life 2 d) | 6.8 / 60.1% | — | 33.3 / 24.9% | — | 29.6 / 25.0% |

**Cell format.** `latency_blocks / inclusion_rate%`. `latency_blocks` is the median across-seeds mean latency-from-submission-to-inclusion conditional on inclusion (1 block ≈ 20 simulated slots ≈ 10 simulated seconds at `rb-generation-probability = 0.05`). `inclusion_rate%` is the median across-seeds fraction of submitted transactions that were included. `—` means median inclusion rate < 1% across seeds — the latency mean over near-zero observations is uninformative, so we report exclusion rather than a noisy number.

**Two-paragraph interpretation.** Single-lane EIP-1559 looks competitive on latency (6.8–13.8 blocks across all tiers) but its inclusion rate distribution is the inverse of what an urgency-pricing mechanism is supposed to deliver: high-urgency arbitrage and DEX-launch traffic (c7–c10) get 12–25% inclusion while low-urgency background transfers (c0–c2) get 55–60%. This is the *first-price-auction-under-congestion* failure mode — the controller cannot price-discriminate fast enough during demand spikes, so the spike traffic loses out to the steady-state background load. The two-lane mechanisms reverse this: very-high-urgency users get 36–41% inclusion (up from 22–25% under single-lane), at the cost of slightly higher latency (14–16 blocks vs 13).

The flip side is what happens to low-urgency users. Ranking-block-reserved (RB-reserved) variants (`priority-only RB-reserved` and `both-dynamic partitioned`) effectively exclude users with half-life ≥ 1 hour — no inclusion at all for `c0` / `c1` / `c2`, ~2% for `c5`. Un-reserved variants (`priority-only un-reserved` and `both-dynamic un-partitioned`) serve everyone but charge low-urgency users in latency: 25–35 blocks (half the run length) for 25% inclusion. **The "menu" the CIP presents is genuine on this axis** — RB-reserved trades low-urgency-exclusion for tighter priority service; un-reserved trades latency for inclusion-rate parity. Neither is a free win.

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

1. **One cell of the (demand × multiplier_floor) matrix.** This table covers `sundaeswap_moderate × multiplier_floor = 4` only. The Phase 3 TEST-07a finding (see [`../test-results/multiplier-floor-16-companion/results.md`](../test-results/multiplier-floor-16-companion/results.md) and `RSK-multiplier-floor-4-suite-coverage` in the [`realism-risks-register.md`](realism-risks-register.md)) is that `multiplier_floor = 16` shifts the picture substantially (priority captures more of the supply, total welfare collapses 93–98%); the latency-by-urgency cross-cut at `floor = 16` is not built here and would require either re-using existing TEST-07a outputs (3 seeds; ordering-level only) or a fresh run. Other demand profiles (`paper_like_congested`, `paper_like_uniform`, etc.) are likewise unrepresented in this table.

2. **Median across seeds, not Bias-corrected and accelerated (BCa) Confidence Intervals (CIs).** We report the median of per-seed `latency_blocks_mean` and `inclusion_rate` across the 20 seeds. This is sufficient for ordering claims and for the qualitative story above. If any specific cell becomes load-bearing for a CIP claim, that cell should be re-run with BCa CIs on the paired latency delta (per the Phase 3 N=20 BCa stack documented in [`methodology-overview.md`](methodology-overview.md) §"(5) Confidence intervals"). The COV-05 hash-diversity gate (per [`../test-results/hash-diversity-gate/results.md`](../test-results/hash-diversity-gate/results.md)) already establishes that all 20 seeds in the canonical-variance run produce distinct pricing-event-stream Secure Hash Algorithm 256-bit hashes — so the latency observations are not artefacts of seed collapse.

3. **`target_inclusion_blocks` defaults seed the actor's lane choice for the first ~50 simulated slots** ([`realism-risks-register.md`](realism-risks-register.md) `RSK-target-inclusion-blocks-default`). The observed-latency Exponentially-Weighted Moving Average (EMA) overwrites the seed once inclusion events arrive, so the seeded defaults (priority = 1 block, standard = 4 blocks) influence early-run dynamics only. The latency means in this table are averages over the full 2000-slot run, so they integrate over both the seeded-defaults regime and the observed-latency regime.

4. **Latency-when-not-included is not captured.** A transaction submitted but never included contributes nothing to the `latency_blocks_mean` for its component — only the included transactions' wait times are averaged. The companion metric is the inclusion rate. The `—` markers in the table flag cases where inclusion is so rare that the latency mean is statistically meaningless; the user-facing experience for those cells is "your transaction does not get confirmed in this 2000-slot run."

5. **The `c6` / `c8` non-monotone anomaly** (see §"Anomaly to flag") needs a team confirmation before this table is cited as definitive for the priority-vs-standard boundary. If it turns out to be a calibration bug rather than a deterministic-rounding-boundary effect, the table requires rebuilding.

## Cross-references

- **Realism-risks register entries this table interacts with** (see [`realism-risks-register.md`](realism-risks-register.md)): `RSK-target-inclusion-blocks-default` (seeded defaults for lane-choice expected-utility), `RSK-max-fee-policy-default` (actor `max_fee_policy = {4, 1}` shapes the priority-vs-standard boundary), `RSK-single-seed-precision` (table is at the same N=20 set as the headline welfare findings), `RSK-multiplier-floor-4-suite-coverage` (table is at the calibration-regime-specific floor=4 setting).
- **Coverage-check rows this table refines** (see [`coverage-check.md`](coverage-check.md)): CLM-06 / CLM-07 (priority-only RB-reserved + un-reserved welfare claims), CLM-08 / CLM-09 (both-dynamic partitioned + un-partitioned welfare claims), CLM-05 (single-lane EIP-1559 control). The four non-welfare property columns of `coverage-check.md` (anti-bribery / signal-source-anchoring / standard-user-fee-drift-exposure / implementation-complexity) do not include latency-by-urgency; this document is the supplement.
- **Audit document section this complements** (see [`cardano-realism-audit.md`](cardano-realism-audit.md)): §"Recommended disclosure statements" §"On controller calibration" notes the calibration-regime-dependence; §"On the priority-vs-standard boundary" can cite this table for the operational expression of that calibration choice.
- **Methodology** (see [`methodology-overview.md`](methodology-overview.md) §"(4) ActorComponent + MaxFeePolicy"): describes the `LanePolicy::UtilityMaximising` expected-utility computation that produces the priority / standard split observed here.

## Methodology

**Source data.** `sim-rs/output/phase-3/canonical-variance-20260518-084846/metrics_comparison.txt`. The Phase 3 `phase-3-canonical-variance` suite (see [`../../sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml`](../../sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml)) ran 5 jobs (the 4 menu options + the single-lane EIP-1559 control) × 20 seeds against `sundaeswap_moderate.yaml` demand on the 100-node realistic topology at `multiplier_floor = 4`. The metrics collector emits one `## job=<name> seed=<n>` block per (job, seed), each with an indented `- per-component:` section listing per-component `latency_blocks_mean=<X>` and `inclusion_rate=<Y>` (plus other fields; see [`sim-rs/sim-cli/src/metrics/collector.rs`](../../sim-rs/sim-cli/src/metrics/collector.rs) for the full schema).

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

The table above is generated from `sim-rs/output/phase-3/canonical-variance-20260518-084846/metrics_comparison.txt` by the Python script below. To rebuild against a fresh run, re-execute the suite via `cargo run --release --bin experiment-suite -- run sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml` and re-run the script against the new run-id output directory.

```python
#!/usr/bin/env python3
"""Build the latency-by-urgency × mechanism table from a phase-3-canonical-variance
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
           "sim-rs/output/phase-3/canonical-variance-20260518-084846/metrics_comparison.txt").read()

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

**Commit + tag.** The table reflects `sim-rs/output/phase-3/canonical-variance-20260518-084846/` outputs from the Phase-3 run at the post-`phase-2-cip-evidence-v1` tag. Future re-runs supersede this table; the tag-pinned snapshot is the citable reference.
