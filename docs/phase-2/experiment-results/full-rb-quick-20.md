# Quick full-RB threshold check

Full-RB threshold minus three-quarter-RB threshold; paired seeds 0–19 (`n = 20`), 2,000 slots per seed, under independent random streams. The arms differ only in `design.reservationPolicy.ebThresholdBytes` (90,112 B versus 67,584 B). Cells are paired mean differences with two-sided 95% paired-t confidence intervals.

Simulator SHA-256: `3843646696ac5d1190d00565ecc7a28b40d2eb009225ad5a67cb14ec49174c85`. The rerun's three-quarter-RB arm exactly reproduces the first 20 seeds of the production threshold ablation at both loads.

| Load | Urgent retained ratio (pp) | Urgent retained value (M lovelace) | Urgent latency (blocks) | Standard latency (blocks) | Standard retained ratio (pp) | Overall retained ratio (pp) |
|---|---:|---:|---:|---:|---:|---:|
| Low | +1.229 [+0.808, +1.650] | +2.074 [+1.421, +2.726] | -0.063 [-0.083, -0.043] | +0.279 [+0.182, +0.376] | -0.260 [-0.339, -0.180] | +0.059 [+0.019, +0.100] |
| Mid load | +0.999 [+0.440, +1.559] | +2.594 [+1.158, +4.029] | -0.051 [-0.081, -0.022] | +0.061 [+0.022, +0.100] | -0.061 [-0.101, -0.021] | +0.081 [+0.034, +0.128] |

The absolute overall-retained-value differences are imprecise: +0.814 M lovelace [-12.537 M, +14.164 M] at low load and +14.316 M [-10.668 M, +39.299 M] at mid load. The ratio and absolute estimands weight seeds differently.

The monotone urgent/standard trade-off therefore continues through one RB: the full-RB arm improves urgent retention and latency while worsening standard retention and latency. It is the best tested arm for urgent outcomes, not an unconditional welfare optimum. This is a directional exploration, not a replacement for the 100-seed production sweep. K = 10 remains active, so the tested policy is a one-RB byte threshold with the age escape; no execution-unit threshold was tested.
