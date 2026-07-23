# Discussion of seeded experiment, comparing reserved vs unreserved priority allocation against EIP-1559 and control #

### TLDR ###

> **Cross-lane max-quote correction (2026-07-13).** Under rb-only settlement, an urgent transaction can pay the urgent quote in an RB or the standard quote in an EB. Wallet choice, its posted fee cap, admission, revalidation, and producer selection now use the larger of those two quotes; settlement still uses the inclusion-point quote and rejects underfunding rather than silently undercharging. A matched ten-seed launch-day smoke experiment found no statistically detectable shift in headline welfare metrics: versus the saved pre-correction denominator-8 anchor, overall retained value changed by -0.234 percentage points (95% paired CI [-1.225, +0.756]), service by -0.016 pp [-0.930, +0.898], and mean latency by +0.045 blocks [-0.139, +0.229]. A follow-up at denominator 16 and a further integrated run of the complete canonical D16/K10 configuration were each exactly unchanged across all 550 paired scalar outputs (55 per seed). This is bounded evidence, not an equivalence result or a regeneration of the tables in this report; only the denominator-8 legacy traces directly establish inversion exposure. Except for the dedicated 3×/16× multiplier-floor experiment, those tables used no cross-lane floor (`multiplierFloor: null`) and predate the max-of-two correction. Treat them as evidence about the independently controlled, no-floor mechanism, not as post-correction welfare estimates during quote inversion.

Experiment results show that, across ten seeded runs, the best aggregate rows reduce urgent mean latency from 2.91 to 2.39 blocks (~18%, or 0.52 blocks) and improve urgent retained value from 44.32% to 51.65% (+7.33 percentage points, ~16.5% relative) by providing network participants with a priority lane to which they can opt to submit transactions, for a premium fee. A slight compromise (~2% and 0.1%) on both urgent latency and urgent retention gives us ledger enforceability with the reserved variant, preventing bribery. However, under low load, plain reservation backfires: it falls _below_ the flat-fee baseline (56.77% vs 58.79% urgent retained value, 1.95 vs 1.85 blocks), because every scrap of standard overflow triggers an endorser block whose certificate then consumes ranking-block space. Gating EB announcement on a byte threshold - an EB may only be announced when its payload reaches half the RB byte cap, so every certificate is worth the block that carries it - repairs this while keeping RBs urgent-only at all times: it restores statistical parity with flat fee at low load (+1.01 ± 1.46 percentage points), behaves identically to plain reservation under sustained congestion, and clearly beats flat fee at every contended load (+3.04 to +7.38 percentage points, ten of ten seeds). As a result, we recommend both-dynamic-strict-threshold: reserved RBs with an EB threshold of half the RB byte cap, 5-sample window, calibrated at target utilisation 0.5 with max-change denominator 16 (welfare-equivalent to the denominator-8 anchor used in the comparison tables - all paired CIs span zero - while eliminating price shocks at three of four loads). A parameter stress test (target utilisation × max-change denominator, ten seeds, four loads) confirms the recommendation is not parameter-fragile: inside the envelope of target utilisation 0.5-0.75 and denominator 8-16 it never falls below the flat-fee baseline (the advantage holds at every load at target 0.5, and at every load except EB-saturating traffic at 0.75), fails informatively outside that envelope (target utilisation 0.25 falls below flat fee under launch-day load), fixes the threshold specification at max((1 - targetUtilisation) × |RB|, |RB| / 2), and rejects the cross-lane multiplier floor carried by the mechanism-design doc and the prototype (-9.25 percentage points at low load at 3×; at 16× the severe-congestion advantage is erased entirely). Two further stress tests close the programme: a demand-elasticity sweep confirms the advantage over flat fee at every mix tested and shows it growing with the share of high-value demand (up to +8,624G over a matched flat-fee control at launch-day), while scoping denominator 16's welfare-equivalence to baseline elasticity; and a trickle sweep validates the EB announcement age escape (K = 10 ranking-block intervals), which repairs total standard-lane starvation at 0.1 tx/slot (+83 percentage points standard retained value, ten of ten seeds) at no urgent-lane cost while remaining bit-identical to the pure threshold at ordinary loads. The unenforceable open variant retains a small measurable lead where capacity is slack (~1-1.6 percentage points at low and mid load); we accept this as the price of preventing bribery. Under a launch-day profile (sustained saturation with the urgency mix skewed upward, calibrated to the January 2022 SundaeSwap launch), the recommendation still clearly beats flat fee (+5.83 ± 4.22 percentage points, eight of ten seeds), though through admission rather than latency, while reservation over a statically-priced standard lane delivers nothing: unpriced standard traffic squats in the shared mempool and starves the reserved lane, so ledger enforceability requires the both-dynamic family.


### Question ###

Can reserving space for priority transactions compete with an open, priority-first regime?

We ask this because a reserved mechanism allows for greater control over the way the mechanism must be used. The reason this is desirable is so that we can sculpt the incentive structure in a way that is fair and ecosystem-friendly. For example, if we don't reserve space, we can't enforce the mechanism on the ledger, and thus, if our reward structure for dynamic fees is anything other than giving the node all of the excess, then nodes will have an incentive to accept and even encourage bribery for positioning.

---

### Method ###

Each experiment config is run under an identical seeded load. We do this so that differences in outcomes are attributable to the mechanism rather than to differences in demand between runs. Transaction submissions arrive according to a Poisson process whose rate varies over time; for a fixed seed and workload profile, there is a fixed transaction submission schedule, supporting reproducibility, with the mean load determined by the burst criteria. For these experiments, we operate at a mean load of 40 tx/slot between slots 0-249 and slots 1750-1999, and at a mean load of 160 tx/slot between slots 250-1749.

<details>
<summary>Show experiment config</summary>

Tx load:
```
severeCongestionLoad :: ArrivalProcess
severeCongestionLoad =
  BurstLoad
    [ Burst
        { baseRate = 40.0
        , burstRate = 160.0
        , burstStart = SlotNo 250
        , burstEnd = SlotNo 1_750
        , burstEffect = BurstEffect 1 1
        }
    ]
```

Sweep harness config:
```
{
  "description": "Phase-2 mechanism set: controls, live dynamic-pricing candidates, and windowed-priority companions under severe congestion",
  "seeds": 10,
  "slots": 2000,
  "out": "sweep-results/mechanisms",
  "variants": [
    { "name": "flat-fee", "config": "config/variants/flat-fee.json" },
    { "name": "single-lane-eip1559", "config": "config/variants/single-lane-eip1559.json" },
    { "name": "priority-only-reserved", "config": "config/variants/priority-only-reserved.json" },
    { "name": "priority-only-open", "config": "config/variants/priority-only-open.json" },
    { "name": "both-dynamic-reserved", "config": "config/default-sim-config.json" },
    { "name": "both-dynamic-open", "config": "config/variants/no-reservation.json" },
    { "name": "priority-only-reserved-window3", "config": "config/variants/priority-only-reserved-window3.json" },
    { "name": "priority-only-open-window3", "config": "config/variants/priority-only-open-window3.json" },
    { "name": "both-dynamic-reserved-window3", "config": "config/variants/both-dynamic-reserved-window3.json" },
    { "name": "both-dynamic-open-window3", "config": "config/variants/both-dynamic-open-window3.json" },
    { "name": "priority-only-reserved-windowed", "config": "config/variants/priority-only-reserved-windowed.json" },
    { "name": "priority-only-strict-threshold-rb2-windowed", "config": "config/variants/priority-only-strict-threshold-rb2-windowed.json" },
    { "name": "both-dynamic-strict-threshold-rb2-windowed", "config": "config/variants/both-dynamic-strict-threshold-rb2-windowed.json" },
    { "name": "priority-only-open-windowed", "config": "config/variants/priority-only-open-windowed.json" },
    { "name": "both-dynamic-reserved-windowed", "config": "config/variants/both-dynamic-reserved-windowed.json" },
    { "name": "both-dynamic-open-windowed", "config": "config/variants/both-dynamic-open-windowed.json" },
    { "name": "priority-only-reserved-window10", "config": "config/variants/priority-only-reserved-window10.json" },
    { "name": "priority-only-open-window10", "config": "config/variants/priority-only-open-window10.json" },
    { "name": "both-dynamic-reserved-window10", "config": "config/variants/both-dynamic-reserved-window10.json" },
    { "name": "both-dynamic-open-window10", "config": "config/variants/both-dynamic-open-window10.json" },
    { "name": "priority-only-reserved-window20", "config": "config/variants/priority-only-reserved-window20.json" },
    { "name": "priority-only-open-window20", "config": "config/variants/priority-only-open-window20.json" },
    { "name": "both-dynamic-reserved-window20", "config": "config/variants/both-dynamic-reserved-window20.json" },
    { "name": "both-dynamic-open-window20", "config": "config/variants/both-dynamic-open-window20.json" }
  ]
}
```

</details>

The offered-demand table below translates these arrival rates into the resources they place on the endorser block - bytes from each transaction's body size (`txBody._txSize`, mean ~1,234 B) and ex-units from its script (mean ~615k). Measured from the flat-fee runs, averaged over the ten seeds:

| Slot range | Mean arrivals / slot | Offered bytes/slot (% of EB byte cap) | Offered ex-units/slot (% of EB ex-unit cap) |
|---|---:|---:|---:|
| 0–249 | ~40 | 8% | 5% |
| 250–1749 | ~160 | 33% | 21% |
| 1750–1999 | ~40 | 8% | 5% |

Even at the burst, this load offers only ~33% of EB byte capacity, so the endorser block has ample headroom and the ranking block is the relevant constraint. (The `eb-capacity-stress` profile, discussed later, pushes peak byte demand to ~88% of EB capacity by contrast.)

---

**Metrics.** For each run we record eight families of outcome, some of which are sliced by urgency class by lane:

- **Inclusion** - The percentage of transactions (distinct demand units; retries do not add to the count) that were included in any block
- **Value** - The sum of transaction value (in Lovelace) captured, lost and unresolved
- **Latency** - The delay (in blocks) between first submission of transactions and their inclusion in a block
- **Shock count** - Number of single-step relative price moves exceeding the shock threshold
- **Settled coefficient range** - Residual peak-to-peak coefficient movement after convergence
- **Price oscillation** - Significant repeated direction reversals (moves larger than the 5% convergence-band deadband) in the coefficient
- **Revenue** - The sum of fees
- **Throughput** - The number of transactions per slot
---

**Fee posting and admission.** Every dynamic-fee variant runs under the same stale-fee node policy: a transaction is admitted to the mempool only if its posted max fee covers the quote one worst-case controller step ahead (quote × (1 + 1/D)), and a producer selects only transactions that remain valid through one further step, so a certified EB cannot fail fee validation. Unserviceable transactions are rejected at admission rather than left to go stale against the mempool cap. On the demand side, actors post a 2× fee buffer over the quote at submission, escalating on retries. These settings (`admissionHeadroomUpdates: 1`, `feeBuffer: 2`) sit under every result below, in the same way the demand calibration does.

<details>
<summary>Show variant index: every experiment variant, one line each</summary>

<br>

The configs are the source of truth; each entry links to the exact file the sweep harness runs. Window counts are the urgent-lane signal window in samples; "instant" means no window.

**Mechanism families** ([`sweeps/mechanisms.json`](../../abstract-sim-hs/config/sweeps/mechanisms.json); [`sweeps/strict-threshold.json`](../../abstract-sim-hs/config/sweeps/strict-threshold.json) and [`sweeps/launch-day.json`](../../abstract-sim-hs/config/sweeps/launch-day.json) re-run subsets of these under other load profiles):

| Variant | What it is | Config |
|---|---|---|
| `flat-fee` | Control: today's static minimum fee, single lane | [config](../../abstract-sim-hs/config/variants/flat-fee.json) |
| `single-lane-eip1559` | Control: single lane priced by one EIP-1559 controller, no urgency signal | [config](../../abstract-sim-hs/config/variants/single-lane-eip1559.json) |
| `priority-only-reserved` | RB reserved for urgent transactions; urgent lane dynamic, standard lane static | [instant](../../abstract-sim-hs/config/variants/priority-only-reserved.json) [w3](../../abstract-sim-hs/config/variants/priority-only-reserved-window3.json) [w5](../../abstract-sim-hs/config/variants/priority-only-reserved-windowed.json) [w10](../../abstract-sim-hs/config/variants/priority-only-reserved-window10.json) [w20](../../abstract-sim-hs/config/variants/priority-only-reserved-window20.json) |
| `priority-only-open` | Urgent lane dynamic, standard static; no reservation, priority delivered by producer-side priority-first ordering | [instant](../../abstract-sim-hs/config/variants/priority-only-open.json) [w3](../../abstract-sim-hs/config/variants/priority-only-open-window3.json) [w5](../../abstract-sim-hs/config/variants/priority-only-open-windowed.json) [w10](../../abstract-sim-hs/config/variants/priority-only-open-window10.json) [w20](../../abstract-sim-hs/config/variants/priority-only-open-window20.json) |
| `both-dynamic-reserved` | RB reserved; both lanes dynamic | [instant](../../abstract-sim-hs/config/default-sim-config.json) [w3](../../abstract-sim-hs/config/variants/both-dynamic-reserved-window3.json) [w5](../../abstract-sim-hs/config/variants/both-dynamic-reserved-windowed.json) [w10](../../abstract-sim-hs/config/variants/both-dynamic-reserved-window10.json) [w20](../../abstract-sim-hs/config/variants/both-dynamic-reserved-window20.json) |
| `both-dynamic-open` | Both lanes dynamic; no reservation, priority-first ordering | [instant](../../abstract-sim-hs/config/variants/no-reservation.json) [w3](../../abstract-sim-hs/config/variants/both-dynamic-open-window3.json) [w5](../../abstract-sim-hs/config/variants/both-dynamic-open-windowed.json) [w10](../../abstract-sim-hs/config/variants/both-dynamic-open-window10.json) [w20](../../abstract-sim-hs/config/variants/both-dynamic-open-window20.json) |
| `priority-only-strict-threshold-rb2` | `priority-only-reserved` plus the EB announcement threshold (payload at least half the RB byte cap); window 5 | [w5](../../abstract-sim-hs/config/variants/priority-only-strict-threshold-rb2-windowed.json) |
| `both-dynamic-strict-threshold-rb2` | `both-dynamic-reserved` plus the EB announcement threshold; window 5; the recommended mechanism | [w5](../../abstract-sim-hs/config/variants/both-dynamic-strict-threshold-rb2-windowed.json) |

**Parameter stress test** ([`sweeps/param-robustness.json`](../../abstract-sim-hs/config/sweeps/param-robustness.json)):

| Variant | What it is | Config |
|---|---|---|
| `bdst-tuXX-dY` (9-point grid) | The recommended mechanism at target utilisation 0.XX and max-change denominator Y, EB threshold set by the headroom expression; `bdst-tu50-d8` reuses the main config above | [directory](../../abstract-sim-hs/config/variants/param-robustness/) |
| `bdst-tu25-d8-fixed-thr`, `bdst-tu75-d8-fixed-thr` | As the matching grid cell, but the threshold pinned at half the RB byte cap (45,056 B) instead of the headroom value, isolating the threshold expression | [tu25](../../abstract-sim-hs/config/variants/param-robustness/bdst-tu25-d8-fixed-thr.json) [tu75](../../abstract-sim-hs/config/variants/param-robustness/bdst-tu75-d8-fixed-thr.json) |

**Demand elasticity** ([`sweeps/elasticity.json`](../../abstract-sim-hs/config/sweeps/elasticity.json)); `mech-*` is the recommended mechanism at denominator 16, each paired with a matched `flat-*` control under the same mix:

| Variant | What it is | Config |
|---|---|---|
| `mech-base` / `flat-base` | The unscaled actor calibration; reuses `bdst-tu50-d16` and `flat-fee` | - |
| `mech-all10x` / `flat-all10x` | Every actor's transaction values scaled 10× | [mech](../../abstract-sim-hs/config/variants/elasticity/mech-all10x.json) [flat](../../abstract-sim-hs/config/variants/elasticity/flat-all10x.json) |
| `mech-hv10` / `flat-hv10` | 10% of arrivals at 100× values | [mech](../../abstract-sim-hs/config/variants/elasticity/mech-hv10.json) [flat](../../abstract-sim-hs/config/variants/elasticity/flat-hv10.json) |
| `mech-hv25` / `flat-hv25` | 25% of arrivals at 100× values | [mech](../../abstract-sim-hs/config/variants/elasticity/mech-hv25.json) [flat](../../abstract-sim-hs/config/variants/elasticity/flat-hv25.json) |
| `mech-hv25-d8` | The hv25 mix at max-change denominator 8, testing the envelope's fast edge | [config](../../abstract-sim-hs/config/variants/elasticity/mech-hv25-d8.json) |

**Cross-lane multiplier floor** ([`sweeps/multiplier-floor.json`](../../abstract-sim-hs/config/sweeps/multiplier-floor.json)):

| Variant | What it is | Config |
|---|---|---|
| `bdst-floor-off` | The recommended mechanism with no floor; reuses the main config | - |
| `bdst-floor-3` / `bdst-floor-16` | Urgent quote held at or above 3× / 16× the standard quote | [3×](../../abstract-sim-hs/config/variants/multiplier-floor/bdst-floor3.json) [16×](../../abstract-sim-hs/config/variants/multiplier-floor/bdst-floor16.json) |

**Trickle aging** ([`sweeps/trickle-aging.json`](../../abstract-sim-hs/config/sweeps/trickle-aging.json)):

| Variant | What it is | Config |
|---|---|---|
| `thr-noescape` | The recommended mechanism with no age escape; reuses `bdst-tu50-d16` | - |
| `thr-k5` / `thr-k10` / `thr-k20` | The recommended mechanism with the announcement age escape at K = 5 / 10 / 20 ranking-block intervals | [k5](../../abstract-sim-hs/config/variants/trickle-aging/thr-k5.json) [k10](../../abstract-sim-hs/config/variants/trickle-aging/thr-k10.json) [k20](../../abstract-sim-hs/config/variants/trickle-aging/thr-k20.json) |
| `plain-reserved-ref` | Plain reservation bracket: announcement threshold of 1 B, the K → 0 limit | [config](../../abstract-sim-hs/config/variants/trickle-aging/plain-reserved-ref.json) |
| `flat-fee` | Flat-fee control, as above | - |

Load profiles are in [`config/loads/`](../../abstract-sim-hs/config/loads/).

</details>

### Mechanisms ###

In this experiment, we compare six designs under active consideration:

|                   | Open (no reservation) | Reserved RB             | Reserved RB + EB threshold           |
| ----------------- | --------------------- | ----------------------- | ------------------------------------ |
| **Both dynamic**  | both-dynamic-open     | both-dynamic-reserved   | both-dynamic-strict-threshold        |
| **Priority only** | priority-only-open    | priority-only-reserved  | priority-only-strict-threshold       |

Note: Each priority-lane config comes with a set of pricing signal variations, which are not enumerated for readability reasons, for example:

```
        "signal": {
          "type": "priority-reservation-window",
          "window": 5
        }
```

This 5-sample window is a way to smooth the signal and decrease oscillation, but it can come with a tradeoff. A window of N uses the previous N priority-signal samples to dampen price changes. This will be discussed later.

Note: the EB-threshold variants were run as a companion sweep (`config/sweeps/strict-threshold.json`) under the same four load profiles, ten seeds, and 2,000 slots as the main sweep.

---

Unreserved space, two lanes, both dynamic:

No capacity is reserved for priority traffic. Priority transactions are taken from the mempool first (`selection: priority-first`). Both lanes are dynamically priced; a standard controller tracking a capacity-weighted utilisation window and a priority controller tracking priority-reservation utilisation. The priority fee applies regardless of whether a priority transaction ends up being selected for an EB or for an RB (`priorityPremiumScope: everywhere`); note that this currently doesn't affect the decision-making for transaction submission.

<details>
<summary>Show config</summary>

```
{
  "design": {
    "laneStructure": "two",
    "reservationPolicy": {
      "type": "no-reservation"
    },
    "selection": "priority-first",
    "feeSemantics": "eip1559",
    "priorityPremiumScope": "everywhere",
    "controllers": {
      "standardController": {
        "targetUtilisation": 0.5,
        "maxChangeDenominator": 8,
        "initialCoefficient": 1.0,
        "signal": {
          "type": "capacity-weighted-window",
          "window": 20
        }
      },
      "priorityController": {
        "targetUtilisation": 0.5,
        "maxChangeDenominator": 8,
        "initialCoefficient": 2.0,
        "signal": "priority-reservation-util"
      },
      "multiplierFloor": null,
      "absoluteCoeffFloor": 1.0
    }
  },
  "curves": "default",
  "f": 0.05,
  "D": 13,
  "load": "severe-congestion",
  "actors": [
    {
      "count": 2,
      "type": "honest",
      "feeBuffer": 2,
      "minValueFeeMultiple": 1.0,
      "valueMultiplier": 1.0,
      "urgencyMultiplier": 1.0
    }
  ],
  "rbTxBytesCap": 90112,
  "rbExUnitsCap": 96991334,
  "ebTxBytesCap": 12000000,
  "ebStructureBytesCap": 512000,
  "ebExUnitsCap": 9499133448,
  "mempoolBytesCap": 24000000,
  "admissionHeadroomUpdates": 1,
  "retryPolicy": {
    "feeTooLow": {
      "type": "resubmit-after",
      "delaySlots": 2,
      "jitterSlots": 6
    },
    "mempoolFull": {
      "type": "resubmit-after",
      "delaySlots": 20,
      "jitterSlots": 20
    },
    "evicted": {
      "type": "resubmit-after",
      "delaySlots": 10,
      "jitterSlots": 30
    },
    "maxAttempts": 5,
    "escalationFactor": 1.2
  },
  "laneLatencyEstimate": {
    "expectedStandardLatency": 50,
    "expectedPriorityLatency": 25
  },
  "priceConvergenceBandPct": 0.05
}
```

</details>

---

Two lanes, reserved space in the priority lane, both dynamic:

The entire ranking block is reserved for priority traffic (`priority-reservation-rb`), with transactions taken FIFO. Both lanes are dynamically priced (standard + priority controllers, same settings as above), and the priority premium is confined to the reserved ranking block: a priority tx that ends up in an endorser block is refunded the difference between the posted fee and the standard fee (`priorityPremiumScope: rb-only`).

<details>
<summary>Show config</summary>

```
{
  "design": {
    "laneStructure": "two",
    "reservationPolicy": {
      "type": "priority-reservation-rb",
      "bytes": 90112
    },
    "selection": "fifo",
    "feeSemantics": "eip1559",
    "priorityPremiumScope": "rb-only",
    "controllers": {
      "standardController": {
        "targetUtilisation": 0.5,
        "maxChangeDenominator": 8,
        "initialCoefficient": 1.0,
        "signal": {
          "type": "capacity-weighted-window",
          "window": 20
        }
      },
      "priorityController": {
        "targetUtilisation": 0.5,
        "maxChangeDenominator": 8,
        "initialCoefficient": 2.0,
        "signal": "priority-reservation-util"
      },
      "multiplierFloor": null,
      "absoluteCoeffFloor": 1.0
    }
  },
  "curves": "default",
  "f": 0.05,
  "D": 13,
  "load": "severe-congestion",
  "actors": [
    {
      "count": 2,
      "type": "honest",
      "feeBuffer": 2,
      "minValueFeeMultiple": 1.0,
      "valueMultiplier": 1.0,
      "urgencyMultiplier": 1.0
    }
  ],
  "rbTxBytesCap": 90112,
  "rbExUnitsCap": 96991334,
  "ebTxBytesCap": 12000000,
  "ebStructureBytesCap": 512000,
  "ebExUnitsCap": 9499133448,
  "mempoolBytesCap": 24000000,
  "admissionHeadroomUpdates": 1,
  "retryPolicy": {
    "feeTooLow": {
      "type": "resubmit-after",
      "delaySlots": 2,
      "jitterSlots": 6
    },
    "mempoolFull": {
      "type": "resubmit-after",
      "delaySlots": 20,
      "jitterSlots": 20
    },
    "evicted": {
      "type": "resubmit-after",
      "delaySlots": 10,
      "jitterSlots": 30
    },
    "maxAttempts": 5,
    "escalationFactor": 1.2
  },
  "laneLatencyEstimate": {
    "expectedStandardLatency": 50,
    "expectedPriorityLatency": 25
  },
  "priceConvergenceBandPct": 0.05
}
```

</details>

---

Unreserved space, two lanes, priority only:

The same as the first design, titled here as: "Unreserved space, two lanes, both dynamic", except only the priority lane is dynamically priced, while the standard lane is fixed-fee.

<details>
<summary>Show config</summary>

```
{
  "design": {
    "laneStructure": "two",
    "reservationPolicy": {
      "type": "no-reservation"
    },
    "selection": "priority-first",
    "feeSemantics": "eip1559",
    "priorityPremiumScope": "everywhere",
    "controllers": {
      "priorityController": {
        "targetUtilisation": 0.5,
        "maxChangeDenominator": 8,
        "initialCoefficient": 2.0,
        "signal": "priority-reservation-util"
      },
      "multiplierFloor": null,
      "absoluteCoeffFloor": 1.0
    }
  },
  "curves": "default",
  "f": 0.05,
  "D": 13,
  "load": "severe-congestion",
  "actors": [
    {
      "count": 2,
      "type": "honest",
      "feeBuffer": 2,
      "minValueFeeMultiple": 1.0,
      "valueMultiplier": 1.0,
      "urgencyMultiplier": 1.0
    }
  ],
  "rbTxBytesCap": 90112,
  "rbExUnitsCap": 96991334,
  "ebTxBytesCap": 12000000,
  "ebStructureBytesCap": 512000,
  "ebExUnitsCap": 9499133448,
  "mempoolBytesCap": 24000000,
  "admissionHeadroomUpdates": 1,
  "retryPolicy": {
    "feeTooLow": {
      "type": "resubmit-after",
      "delaySlots": 2,
      "jitterSlots": 6
    },
    "mempoolFull": {
      "type": "resubmit-after",
      "delaySlots": 20,
      "jitterSlots": 20
    },
    "evicted": {
      "type": "resubmit-after",
      "delaySlots": 10,
      "jitterSlots": 30
    },
    "maxAttempts": 5,
    "escalationFactor": 1.2
  },
  "laneLatencyEstimate": {
    "expectedStandardLatency": 50,
    "expectedPriorityLatency": 25
  },
  "priceConvergenceBandPct": 0.05
}
```

</details>

---

Reserved space, two lanes, priority only:

The entire ranking block is reserved for priority transactions, with FIFO selection, but only the priority lane is dynamically priced. The priority fee is paid if the transaction makes it into the ranking block (`priorityPremiumScope: rb-only`), otherwise the difference between the posted fee and the standard fee is refunded.


<details>
<summary>Show config</summary>

```
{
  "design": {
    "laneStructure": "two",
    "reservationPolicy": {
      "type": "priority-reservation-rb",
      "bytes": 90112
    },
    "selection": "fifo",
    "feeSemantics": "eip1559",
    "priorityPremiumScope": "rb-only",
    "controllers": {
      "priorityController": {
        "targetUtilisation": 0.5,
        "maxChangeDenominator": 8,
        "initialCoefficient": 2.0,
        "signal": "priority-reservation-util"
      },
      "multiplierFloor": null,
      "absoluteCoeffFloor": 1.0
    }
  },
  "curves": "default",
  "f": 0.05,
  "D": 13,
  "load": "severe-congestion",
  "actors": [
    {
      "count": 2,
      "type": "honest",
      "feeBuffer": 2,
      "minValueFeeMultiple": 1.0,
      "valueMultiplier": 1.0,
      "urgencyMultiplier": 1.0
    }
  ],
  "rbTxBytesCap": 90112,
  "rbExUnitsCap": 96991334,
  "ebTxBytesCap": 12000000,
  "ebStructureBytesCap": 512000,
  "ebExUnitsCap": 9499133448,
  "mempoolBytesCap": 24000000,
  "admissionHeadroomUpdates": 1,
  "retryPolicy": {
    "feeTooLow": {
      "type": "resubmit-after",
      "delaySlots": 2,
      "jitterSlots": 6
    },
    "mempoolFull": {
      "type": "resubmit-after",
      "delaySlots": 20,
      "jitterSlots": 20
    },
    "evicted": {
      "type": "resubmit-after",
      "delaySlots": 10,
      "jitterSlots": 30
    },
    "maxAttempts": 5,
    "escalationFactor": 1.2
  },
  "laneLatencyEstimate": {
    "expectedStandardLatency": 50,
    "expectedPriorityLatency": 25
  },
  "priceConvergenceBandPct": 0.05
}
```
</details>

---

Reserved RB with an EB announcement threshold, two lanes (priority-only and both-dynamic forms):

The same as the reserved designs above - the ranking block only ever carries priority transactions - with one extra rule on the endorser block: an EB may only be announced when its payload reaches a byte threshold, here half the RB byte cap (45,056 bytes). Every certificate is therefore worth at least half a ranking block of payload; thin EBs, whose certificates would consume more RB space than the capacity they deliver, are never produced, and standard transactions queue for the next worthwhile batch instead. With the threshold at one byte this rule coincides exactly with the plain reserved design (verified bit-identically at the event level). The configs match the corresponding reserved configs apart from the reservation policy stanza:

```
    "reservationPolicy": {
      "type": "priority-reservation-rb-eb-threshold",
      "ebThresholdBytes": 45056,
      "bytes": 90112
    }
```

---

These six designs are compared against a control, flat fee:

A single lane charging a fixed fee, with no dynamic controller (`feeSemantics: fixed-fee`, `laneStructure: one`) and no priority tier. The mempool is FIFO. This is the static-fee control.


<details>
<summary>Show config</summary>

```
{
  "design": {
    "laneStructure": "one",
    "reservationPolicy": {
      "type": "no-reservation"
    },
    "selection": "fifo",
    "feeSemantics": "fixed-fee",
    "priorityPremiumScope": "everywhere",
    "controllers": {
      "multiplierFloor": null,
      "absoluteCoeffFloor": 1.0
    }
  },
  "curves": "default",
  "f": 0.05,
  "D": 13,
  "load": "severe-congestion",
  "actors": [
    {
      "count": 2,
      "type": "honest",
      "feeBuffer": 1,
      "minValueFeeMultiple": 1.0,
      "valueMultiplier": 1.0,
      "urgencyMultiplier": 1.0
    }
  ],
  "rbTxBytesCap": 90112,
  "rbExUnitsCap": 96991334,
  "ebTxBytesCap": 12000000,
  "ebStructureBytesCap": 512000,
  "ebExUnitsCap": 9499133448,
  "mempoolBytesCap": 24000000,
  "admissionHeadroomUpdates": 1,
  "retryPolicy": {
    "feeTooLow": {
      "type": "resubmit-after",
      "delaySlots": 2,
      "jitterSlots": 6
    },
    "mempoolFull": {
      "type": "resubmit-after",
      "delaySlots": 20,
      "jitterSlots": 20
    },
    "evicted": {
      "type": "resubmit-after",
      "delaySlots": 10,
      "jitterSlots": 30
    },
    "maxAttempts": 5,
    "escalationFactor": 1.2
  },
  "laneLatencyEstimate": {
    "expectedStandardLatency": 50,
    "expectedPriorityLatency": 25
  },
  "priceConvergenceBandPct": 0.05
}
```


</details>

---

And a baseline, EIP-1559:

A single lane with an EIP-1559 dynamic base fee (one standard controller tracking a capacity-weighted utilisation window) but no priority tier and no reservation. This is the EIP-1559 baseline: dynamic pricing without tiering.

<details>
<summary>Show config</summary>

```
{
  "design": {
    "laneStructure": "one",
    "reservationPolicy": {
      "type": "no-reservation"
    },
    "selection": "fifo",
    "feeSemantics": "eip1559",
    "priorityPremiumScope": "everywhere",
    "controllers": {
      "standardController": {
        "targetUtilisation": 0.5,
        "maxChangeDenominator": 8,
        "initialCoefficient": 1.0,
        "signal": {
          "type": "capacity-weighted-window",
          "window": 20
        }
      },
      "multiplierFloor": null,
      "absoluteCoeffFloor": 1.0
    }
  },
  "curves": "default",
  "f": 0.05,
  "D": 13,
  "load": "severe-congestion",
  "actors": [
    {
      "count": 2,
      "type": "honest",
      "feeBuffer": 2,
      "minValueFeeMultiple": 1.0,
      "valueMultiplier": 1.0,
      "urgencyMultiplier": 1.0
    }
  ],
  "rbTxBytesCap": 90112,
  "rbExUnitsCap": 96991334,
  "ebTxBytesCap": 12000000,
  "ebStructureBytesCap": 512000,
  "ebExUnitsCap": 9499133448,
  "mempoolBytesCap": 24000000,
  "admissionHeadroomUpdates": 1,
  "retryPolicy": {
    "feeTooLow": {
      "type": "resubmit-after",
      "delaySlots": 2,
      "jitterSlots": 6
    },
    "mempoolFull": {
      "type": "resubmit-after",
      "delaySlots": 20,
      "jitterSlots": 20
    },
    "evicted": {
      "type": "resubmit-after",
      "delaySlots": 10,
      "jitterSlots": 30
    },
    "maxAttempts": 5,
    "escalationFactor": 1.2
  },
  "laneLatencyEstimate": {
    "expectedStandardLatency": 50,
    "expectedPriorityLatency": 25
  },
  "priceConvergenceBandPct": 0.05
}
```
</details>

---

### What this is not ###

Adversarial actors, workload profile sweep, dependency chain simulation, etc

---

### Results ###

Across ten seeded runs, all mechanisms preserve high overall inclusion rates, but the differences show up in retained value, urgent latency, and the reserved-vs-open tradeoff. Reservation is competitive with open priority-first selection, but it does not dominate it on every metric.

Please note that when we say "urgent", we're referencing the lowest half-life urgency bucket: a half-life of 2 blocks.

| Family | Priority signal | Inclusion | Urgent retained | Urgent latency (blk) | Priority latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp | Settled coeff. range |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| flat-fee | n/a | 98.98% | 44.32% | 2.91 | n/a | 127.4 | 0.0 | 0.0 | 0.000 | 0.000 |
| single-lane-eip1559 | n/a | 98.97% | 43.60% | 3.02 | n/a | 119.0 | 5.9 | 1.1 | 1.743 | 1.327 |
| priority-only-reserved | instant | 98.94% | 50.45% | 2.52 | 2.10 | 127.3 | 55.2 | 14.8 | 1.446 | 1.738 |
| priority-only-reserved | 3-sample window | 98.72% | 49.21% | 2.63 | 2.16 | 127.1 | 20.4 | 6.8 | 1.585 | 0.902 |
| priority-only-reserved | 5-sample window | 98.92% | 50.74% | 2.51 | 2.15 | 127.3 | 12.4 | 4.5 | 1.676 | 1.312 |
| priority-only-reserved | 10-sample window | 98.91% | 49.54% | 2.56 | 2.33 | 127.3 | 12.4 | 3.0 | 2.097 | 1.467 |
| priority-only-reserved | 20-sample window | 99.16% | 48.07% | 2.67 | 2.59 | 127.7 | 11.6 | 2.0 | 3.491 | 2.963 |
| priority-only-open | instant | 99.09% | 50.63% | 2.53 | 2.15 | 127.5 | 55.4 | 14.9 | 1.421 | 1.861 |
| priority-only-open | 3-sample window | 98.97% | 50.44% | 2.52 | 2.10 | 127.4 | 17.5 | 6.2 | 1.501 | 1.259 |
| priority-only-open | 5-sample window | 99.03% | 50.26% | 2.53 | 2.16 | 127.4 | 12.1 | 4.8 | 1.605 | 1.008 |
| priority-only-open | 10-sample window | 99.17% | 50.00% | 2.53 | 2.25 | 127.6 | 11.7 | 2.9 | 2.085 | 1.522 |
| priority-only-open | 20-sample window | 99.01% | 47.88% | 2.70 | 2.57 | 127.4 | 10.0 | 2.0 | 3.055 | 1.663 |
| priority-only-strict-threshold-rb2 | 5-sample window | 98.92% | 50.74% | 2.51 | 2.15 | 127.3 | 12.4 | 4.5 | 1.676 | 1.312 |
| both-dynamic-reserved | instant | 99.08% | 51.01% | 2.50 | 2.19 | 120.9 | 63.4 | 14.9 | 2.464 | 3.479 |
| both-dynamic-reserved | 3-sample window | 99.01% | 51.46% | 2.44 | 2.20 | 121.6 | 22.7 | 6.6 | 2.181 | 2.093 |
| both-dynamic-reserved | 5-sample window | 99.09% | 51.55% | 2.44 | 2.23 | 122.7 | 15.4 | 5.4 | 2.042 | 2.414 |
| both-dynamic-reserved | 10-sample window | 98.76% | 50.70% | 2.47 | 2.45 | 123.2 | 13.1 | 3.4 | 2.438 | 2.272 |
| both-dynamic-reserved | 20-sample window | 99.10% | 48.86% | 2.64 | 2.79 | 124.1 | 13.2 | 2.9 | 3.330 | 1.460 |
| both-dynamic-open | instant | 98.96% | 51.65% | 2.42 | 2.14 | 120.4 | 62.6 | 15.8 | 2.492 | 2.899 |
| both-dynamic-open | 3-sample window | 98.78% | 51.64% | 2.39 | 2.37 | 120.8 | 26.0 | 7.7 | 2.733 | 1.158 |
| both-dynamic-open | 5-sample window | 99.06% | 50.92% | 2.50 | 2.54 | 121.2 | 20.3 | 6.0 | 2.893 | 2.764 |
| both-dynamic-open | 10-sample window | 98.99% | 50.96% | 2.45 | 2.24 | 122.0 | 13.5 | 4.3 | 2.716 | 2.839 |
| both-dynamic-open | 20-sample window | 98.96% | 49.27% | 2.58 | 2.60 | 124.2 | 13.0 | 3.4 | 3.156 | 2.239 |
| both-dynamic-strict-threshold-rb2 | 5-sample window | 99.09% | 51.55% | 2.44 | 2.23 | 122.7 | 15.4 | 5.4 | 2.042 | 2.414 |

You may notice that the strict-threshold rows duplicate the corresponding reserved rows. This is expected: under sustained backlog the prospective EB payload always clears the threshold, so the gate never intervenes and the mechanism reduces to plain reservation by construction - all ten seeds produce bit-identical event streams.

Inclusion reports the mean share of submitted demand eventually included. Latency columns report mean latency as actual produced ranking blocks, from first submission to inclusion; priority latency is n/a for the single-lane controls. Shock count and oscillation cycles are mean counts per run; oscillation cycles count completed significant direction-reversal cycles after the convergence-band deadband. Oscillation max amplitude is the largest local coefficient peak-to-trough range.

The uncertainty checks below use paired seed deltas over the same ten seeds. Deltas are left variant minus right variant; positive is better for urgent retained value and tx/slot, while negative is better for urgent latency, shock count, and oscillation cycles. Confidence intervals are 95% t-intervals over the ten paired seed deltas. "Seeds better" counts strict improvements in the preferred direction. Comparisons labelled with a load use that load's runs (see the corresponding tables in the sections below); unlabelled comparisons use the severe-congestion runs above.

| Comparison | Metric | Mean paired delta ± 95% CI | Seeds better |
|---|---|---:|---:|
| priority-only-reserved 5-sample window vs flat-fee | Urgent retained (pp) | +6.42 ± 1.47 | 10/10 |
| priority-only-reserved 5-sample window vs flat-fee | Urgent latency (blk) | -0.41 ± 0.14 | 10/10 |
| priority-only-reserved 5-sample window vs flat-fee | Tx/slot | -0.1 ± 0.7 | 6/10 |
| both-dynamic-reserved 5-sample window vs flat-fee | Urgent retained (pp) | +7.23 ± 1.64 | 10/10 |
| both-dynamic-reserved 5-sample window vs flat-fee | Urgent latency (blk) | -0.48 ± 0.17 | 10/10 |
| both-dynamic-reserved 5-sample window vs flat-fee | Tx/slot | -4.7 ± 2.2 | 0/10 |
| priority-only-reserved 5-sample window vs priority-only-open 5-sample window | Urgent retained (pp) | +0.47 ± 1.58 | 4/10 |
| priority-only-reserved 5-sample window vs priority-only-open 5-sample window | Urgent latency (blk) | -0.02 ± 0.13 | 5/10 |
| priority-only-reserved 5-sample window vs priority-only-open 5-sample window | Tx/slot | -0.1 ± 0.4 | 5/10 |
| priority-only-reserved 5-sample window vs priority-only-open 5-sample window | Shock count | +0.3 ± 2.2 | 5/10 |
| priority-only-reserved 5-sample window vs priority-only-open 5-sample window | Osc. cycles | -0.3 ± 0.6 | 3/10 |
| both-dynamic-reserved 5-sample window vs both-dynamic-open 5-sample window | Urgent retained (pp) | +0.63 ± 1.43 | 5/10 |
| both-dynamic-reserved 5-sample window vs both-dynamic-open 5-sample window | Urgent latency (blk) | -0.07 ± 0.13 | 6/10 |
| both-dynamic-reserved 5-sample window vs both-dynamic-open 5-sample window | Tx/slot | +1.5 ± 1.6 | 6/10 |
| both-dynamic-reserved 5-sample window vs both-dynamic-open 5-sample window | Shock count | -4.9 ± 7.2 | 7/10 |
| both-dynamic-reserved 5-sample window vs both-dynamic-open 5-sample window | Osc. cycles | -0.6 ± 0.8 | 6/10 |
| both-dynamic-strict-threshold-rb2 vs both-dynamic-reserved | Urgent retained (pp) | +0.00 ± 0.00 | 0/10 |
| both-dynamic-strict-threshold-rb2 vs both-dynamic-reserved (low load) | Urgent retained (pp) | +3.03 ± 1.11 | 10/10 |
| both-dynamic-strict-threshold-rb2 vs both-dynamic-reserved (low load) | Urgent latency (blk) | -0.16 ± 0.05 | 10/10 |
| both-dynamic-strict-threshold-rb2 vs both-dynamic-reserved (mid load) | Urgent retained (pp) | +0.90 ± 0.73 | 8/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee (low load) | Urgent retained (pp) | +1.01 ± 1.46 | 6/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee (low load) | Urgent latency (blk) | -0.06 ± 0.08 | 6/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee (mid load) | Urgent retained (pp) | +3.04 ± 1.17 | 10/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee (mid load) | Urgent latency (blk) | -0.20 ± 0.07 | 10/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee | Urgent retained (pp) | +7.23 ± 1.64 | 10/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee | Urgent latency (blk) | -0.48 ± 0.17 | 10/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee (eb-capacity-stress) | Urgent retained (pp) | +7.38 ± 3.73 | 9/10 |
| both-dynamic-strict-threshold-rb2 vs flat-fee (eb-capacity-stress) | Urgent latency (blk) | -0.81 ± 0.41 | 9/10 |
| both-dynamic-strict-threshold-rb2 vs both-dynamic-open 5-sample window (low load) | Urgent retained (pp) | -1.63 ± 1.11 | 2/10 |
| both-dynamic-strict-threshold-rb2 vs both-dynamic-open 5-sample window (mid load) | Urgent retained (pp) | -1.06 ± 0.83 | 2/10 |
| priority-only-strict-threshold-rb2 vs both-dynamic-strict-threshold-rb2 (eb-capacity-stress) | Urgent retained (pp) | -4.58 ± 4.38 | 2/10 |
| priority-only-strict-threshold-rb2 vs both-dynamic-strict-threshold-rb2 (eb-capacity-stress) | Urgent latency (blk) | +0.51 ± 0.24 | 0/10 |

Three things stand out in the strict-threshold rows. First, under sustained congestion the EB threshold costs nothing: the payload always clears it, so the mechanism is plain reservation exactly, tying in ten of ten seeds. Second, at low load the threshold repairs plain reservation's regression - +3.03 ± 1.11 percentage points over reserved, ten of ten seeds - and restores statistical parity with flat fee (+1.01 ± 1.46, confidence interval spanning zero); at mid load it beats flat fee outright (+3.04 ± 1.17, ten of ten seeds). Third, the both-dynamic family is preferred over priority-only because of the EB-stressing load, where the fixed standard fee cannot shed the demand that saturates the endorser block.



Urgent retained value is improved, in the best aggregate row (both-dynamic-open instant vs flat-fee), from 44.32% to 51.65%, a 7.33 percentage point increase, or ~16.5% relative improvement from the baseline value. This is only a narrow lead over the best reserved row, both-dynamic-reserved 5-sample window, at 51.55%. The best aggregate urgent-latency row is both-dynamic-open 3-sample window, at 2.39 blocks, compared with 2.91 blocks under flat fee: a 0.52 block, or ~18%, reduction. Priority-lane latency can be lower still, reaching 2.10 blocks in the priority-only rows, but the table shows that the priority lane is not exclusively occupied by the most urgent transactions; urgent latency is therefore the better end-to-end measure for urgent users.

The aggregate table shows small matching-row gaps between the open and reserved variants: urgent retained value usually differs by less than 1 percentage point, with the largest matching-row gap being 1.23 percentage points in the priority-only 3-sample window case. As such, the reserved variants should be preferred, since they enable ledger enforceability; this is required in order to prevent bribery, as discussed in the introduction.

We must also note that not everything is an improvement over the flat-fee and EIP-1559 variants. Throughput is slightly lower, at ~122 tx/slot (~6 less than baseline) for the both-dynamic variants and ~127 tx/slot (~1 less than baseline) for the priority-dynamic variants.

While all configs use a capacity weighted utilisation window to adjust the standard lane price, only the windowed variants use this mechanism to adjust the priority lane's price. When looking at the aggregate table, we can spot a tradeoff between the windowed variants and the non-windowed variants. As the sample window size increases, shock count and oscillation cycles decrease, but latency rises, oscillation max amplitude increases, and urgent transaction value retention generally decreases. In other words, "fewer shocks" should not be read as uniformly better price behaviour: the longer sample windows move less often, but their larger peak-to-trough swings are visible in the oscillation max amplitude column. Most of the shock-count reduction is already visible at sample window lengths of 3 and 5, so those short windows look like the most plausible compromise points in the aggregate table.

#### Reading the figures ####

Across the top of each dashboard, note the key info cards. Note also the chart elements:

The annotations below describe seed 2 only. They are useful for reading individual dashboard examples, but they should not be treated as aggregate rankings across the ten seeded runs.

- The "Price coefficient / lane" chart shows, in multiples of the base fee, the price over time of each lane: blue for standard, purple for priority. The circles denote oscillation peaks and troughs.
- The "RB content over time" element shows when RBs contain transactions vs EB certificates. Orange denotes EB certificate, while green denotes transactions. Darker = denser.
- The "Latency / lane" chart shows the latency of the priority lane vs the standard lane over time.
- Next, we have the "Submission ⇄ inclusion" chart, which shows the lifespans of submitted transactions. Submissions are shown at the top, and inclusions at the bottom. Green lines denote direct RB inclusion, while orange lines denote EB inclusion.
- We also have a simple "Load" chart, which gives an at-a-glance view of the submissions per slot rate.
- The "Latency distribution" shows a box-and-whisker plot of the standard vs priority lane latency in blocks.
- The "Demand fate" element shows how many transactions, by urgency (in blocks per halflife) and lane, were included, abandoned, or unresolved.
- The "Value retained vs lost" chart is similar to the above, except rather than simple inclusion vs exclusion, it shows how much of the sum of value of transactions in each urgency category and lane was retained, lost or unresolved.

##### Controls #####

The controls set the seed-2 baseline: flat fee serves 99.5% of demand but retains only 40.09% of highest-urgency value. Single-lane EIP-1559 is lower on served demand but slightly higher on urgent retained value in this seed, at 99.2% served and 40.4% urgent retained value, with a shock count of 9 and 1 oscillation cycle.

<details>
<summary>flat-fee, seed 2</summary>

![flat-fee, seed 2](figures/flat-fee-seed-2.png)

</details>

<details>
<summary>single-lane-eip1559, seed 2</summary>

![single-lane-eip1559, seed 2](figures/single-lane-eip1559-seed-2.png)

</details>

##### Priority-only, reserved #####

Seed-2 urgent retained value rises to 49.6%, compared with 40.09% under flat fee, but the instant priority controller has a shock count of 64 and 18 oscillation cycles.

<details>
<summary>priority-only-reserved, seed 2</summary>

![priority-only-reserved, seed 2](figures/priority-only-reserved-seed-2.png)

</details>

Within the reserved priority-only family, the 3-sample window has the highest seed-2 urgent retained value at 50.1%. It also cuts shocks from 64 to 24 and oscillation cycles from 18 to 8, so the smoother price response is visible in the stability figures as well as in the chart.

<details>
<summary>priority-only-reserved, 3-sample window, seed 2</summary>

![priority-only-reserved, 3-sample window, seed 2](figures/priority-only-reserved-window3-seed-2.png)

</details>

At the 5-sample window, urgent retained value falls to 48.9%, while shocks and cycles fall again to 21 and 6. Most of the stability improvement has already happened by the 3-sample window; the extra urgent-retention loss is 1.2 percentage points in this seed.

<details>
<summary>priority-only-reserved, 5-sample window, seed 2</summary>

![priority-only-reserved, 5-sample window, seed 2](figures/priority-only-reserved-windowed-seed-2.png)

</details>

At the 10-sample window, urgent retained value falls again to 46.8%, and priority-lane inclusion is 98.9%. Price stability improves only modestly from the 5-sample window, moving from 21 shocks and 6 cycles to 19 shocks and 4 cycles.

<details>
<summary>priority-only-reserved, 10-sample window, seed 2</summary>

![priority-only-reserved, 10-sample window, seed 2](figures/priority-only-reserved-window10-seed-2.png)

</details>

The 20-sample window has the fewest oscillation cycles in this family, with 2 cycles, but it also has the lowest seed-2 urgent retained value, at 46.2%, and priority-lane inclusion falls to 94.8%.

<details>
<summary>priority-only-reserved, 20-sample window, seed 2</summary>

![priority-only-reserved, 20-sample window, seed 2](figures/priority-only-reserved-window20-seed-2.png)

</details>

##### Priority-only, open #####

This family shows the same pattern as "priority-only, reserved". In seed 2, the open runs range from 48.3% to 50.1% urgent retained value, compared with 46.2% to 50.1% for the reserved runs; across ten seeds, the aggregate table shows the two families are much closer.

The instant open variant is close to the instant reserved variant: 49.1% urgent retained value vs 49.6%, priority-lane inclusion of 99.5% vs 99.7%, and 66 shocks / 17 cycles vs 64 shocks / 18 cycles. Because the priority lane is open rather than reserved, this variant is useful as a comparison between soft priority ordering (producers selecting priority-lane transactions before standard-lane transactions) and a ledger-enforced RB reservation.

<details>
<summary>priority-only-open, seed 2</summary>

![priority-only-open, seed 2](figures/priority-only-open-seed-2.png)

</details>

The 3-sample window variant keeps urgent retained value at 49.6% and priority-lane inclusion at 98.3%, while reducing price movement to 15 shocks and 6 cycles. That makes it a reasonable open-family point if the goal is to preserve most of the urgent-value benefit without keeping the instant controller's oscillation.

<details>
<summary>priority-only-open, 3-sample window, seed 2</summary>

![priority-only-open, 3-sample window, seed 2](figures/priority-only-open-window3-seed-2.png)

</details>

The 5-sample window open variant has the highest seed-2 urgent retained value in this open family, reaching 50.1%, compared with 48.9% for the reserved 5-sample window. In this run, urgent inclusion is also slightly higher than the reserved 5-sample window (99.27% vs 98.73%) and urgent mean latency is slightly lower (2.64 vs 2.71 blocks); the ten-seed aggregate does not preserve this lead (50.26% open vs 50.74% reserved), so treat it as seed-level variation rather than evidence that the open 5-sample window dominates.

<details>
<summary>priority-only-open, 5-sample window, seed 2</summary>

![priority-only-open, 5-sample window, seed 2](figures/priority-only-open-windowed-seed-2.png)

</details>

At the 10-sample window, urgent retained value falls to 48.3%, with priority-lane inclusion at 96.8%. Price movement is much lower than instant open (18 shocks and 3 cycles vs 66 shocks and 17 cycles), but the urgent-retention number is only 8.2 percentage points above flat fee in this seed.

<details>
<summary>priority-only-open, 10-sample window, seed 2</summary>

![priority-only-open, 10-sample window, seed 2](figures/priority-only-open-window10-seed-2.png)

</details>

The 20-sample window open variant has only 15 shocks and 2 cycles, but priority-lane inclusion is 94.9% and urgent retained value is 48.4%. That is a larger stability gain than service gain relative to the 3- and 5-sample window open variants.

<details>
<summary>priority-only-open, 20-sample window, seed 2</summary>

![priority-only-open, 20-sample window, seed 2](figures/priority-only-open-window20-seed-2.png)

</details>

##### Both dynamic, reserved #####

The both-dynamic reserved family adds a standard-lane controller on top of the reserved priority mechanism. In seed 2, it ranges from 47.5% to 51.3% urgent retained value, all above the 40.09% flat-fee control and 40.4% single-lane EIP-1559 control, but the aggregate table shows lower throughput for the both-dynamic variants and there is now a second moving price.

The instant variant has 51.3% urgent retained value and 2.1-block mean priority latency, but it also has 68 shocks and 17 cycles. This is near the top of the reserved both-dynamic family on seed-2 urgent retention, but not the most stable price picture.

<details>
<summary>both-dynamic-reserved, seed 2</summary>

![both-dynamic-reserved, seed 2](figures/both-dynamic-reserved-seed-2.png)

</details>

The 3-sample window variant remains close on service, with 51.3% urgent retained value and 2.2-block mean priority latency, while shocks fall to 23 and cycles fall to 8. Relative to the instant variant, this is a large stability improvement with almost no seed-2 service cost.

<details>
<summary>both-dynamic-reserved, 3-sample window, seed 2</summary>

![both-dynamic-reserved, 3-sample window, seed 2](figures/both-dynamic-reserved-window3-seed-2.png)

</details>

The 5-sample window variant gives up 0.3 percentage points of urgent retained value relative to the 3-sample window, moving from 51.3% to 51.0%, while reducing shocks and cycles to 15 and 6. Overall demand served remains 99.6%, and priority-lane inclusion is 99.8%.

<details>
<summary>both-dynamic-reserved, 5-sample window, seed 2</summary>

![both-dynamic-reserved, 5-sample window, seed 2](figures/both-dynamic-reserved-windowed-seed-2.png)

</details>

With the 10-sample window, priority-lane inclusion remains high at 99.8%, but urgent retained value falls to 48.8% and mean priority latency rises to 2.5 blocks. Shocks and cycles are 23 and 5, so this is not clearly more stable than the 5-sample window in seed 2.

<details>
<summary>both-dynamic-reserved, 10-sample window, seed 2</summary>

![both-dynamic-reserved, 10-sample window, seed 2](figures/both-dynamic-reserved-window10-seed-2.png)

</details>

The 20-sample window reserved variant has 47.5% urgent retained value and 100% priority-lane inclusion, but standard mean latency is 3.4 blocks and standard p95 latency is 7 blocks. That makes it a cautionary long-window case: priority demand is served, while the standard lane slows.

<details>
<summary>both-dynamic-reserved, 20-sample window, seed 2</summary>

![both-dynamic-reserved, 20-sample window, seed 2](figures/both-dynamic-reserved-window20-seed-2.png)

</details>

##### Both dynamic, open #####

In the ten-seed aggregate table above, the two highest urgent-retention rows are both-dynamic-open instant and 3-sample window, at 51.65% and 51.64%; the lead over both-dynamic-reserved 5-sample window is only 0.10 percentage points. In seed-2, it has the widest spread among the main candidate families: 45.0% to 51.1% urgent retained value and 97.7% to 99.6% demand served.

The instant open variant has 49.1% urgent retained value and 2.3-block mean priority latency, but it also has 74 shocks, 15 cycles, and max oscillation amplitude of 4.327. This is the largest shock count in seed-2.

<details>
<summary>both-dynamic-open, seed 2</summary>

![both-dynamic-open, seed 2](figures/both-dynamic-open-seed-2.png)

</details>

The 3-sample window open variant retains 51.1% of urgent value, serves 99.6% of demand, and has 24 shocks and 10 cycles. Compared with the reserved 3-sample window (51.3% urgent retained value, 99.6% served, 23 shocks, 8 cycles), service is close but instability is not lower, and open still lacks the ledger-enforceability component of the reserved design.

<details>
<summary>both-dynamic-open, 3-sample window, seed 2</summary>

![both-dynamic-open, 3-sample window, seed 2](figures/both-dynamic-open-window3-seed-2.png)

</details>

The 5-sample window open variant has 46.9% urgent retained value and 43 shocks. Its max oscillation amplitude is 7.662, much higher than instant open's 4.327, making it the clearest seed-2 example of the aggregate trend that longer sample windows can reduce shock counts while increasing peak-to-trough movement.

<details>
<summary>both-dynamic-open, 5-sample window, seed 2</summary>

![both-dynamic-open, 5-sample window, seed 2](figures/both-dynamic-open-windowed-seed-2.png)

</details>

The 10-sample window open variant is one of the more stable open seed-2 cases, with 16 shocks, 4 cycles, 50.2% urgent retained value, and 99.5% demand served. It improves stability relative to instant and the 5-sample window, while urgent retention remains below the 3-sample window.

<details>
<summary>both-dynamic-open, 10-sample window, seed 2</summary>

![both-dynamic-open, 10-sample window, seed 2](figures/both-dynamic-open-window10-seed-2.png)

</details>

The 20-sample window open variant is the clearest service failure in seed 2: demand served falls to 97.7%, retry amplification rises to 1.12x, urgent retained value falls to 45.0%, and standard mean / p95 latency reaches 3.7 / 10 blocks. Priority-lane inclusion is 100%, so the cost is mostly pushed onto standard demand.

<details>
<summary>both-dynamic-open, 20-sample window, seed 2</summary>

![both-dynamic-open, 20-sample window, seed 2](figures/both-dynamic-open-window20-seed-2.png)

</details>


### Low load (below RB capacity) ###

An interesting effect occurs under (relatively) low load which hovers between the fill target of the RB and near the RB max. With our plain reserved configurations, we see an _increase_ in urgent transaction latency - the reserved designs fall _below_ the flat-fee baseline in this regime.

This is because, when reserving an RB for urgent transactions, any standard transactions mean the announcement of an EB, however small. Each announced EB must later be certified, and the certificate consumes ranking-block space, depriving urgent transactions of RBs. At this load the EBs are thin: the certificate costs more RB capacity than the payload it delivers.

The solution is to gate the announcement: an EB may only be announced when its payload reaches a byte threshold (we use half the RB byte cap), so every certificate is worth the block space it consumes. RBs remain reserved for urgent transactions at all times - standard transactions simply queue for the next worthwhile batch. We deliberately do not admit standard transactions into underfull RBs: any rule that sells RB access below the urgent quote creates an incentive for producers to accept side payments for inclusion, which cannot be enforced away; we explored and rejected work-conserving variants of that kind for exactly this reason.

The four load profiles span the block-capacity hierarchy (byte fill measured from the flat-fee runs; "reserved" shown where it differs materially):

| Load | RB byte fill | EB byte fill | Binding resource |
|---|---:|---:|---|
| `low` (this section) | ~71% open / ~34% reserved | ~1% | neither saturates; EB idle, RB the only active block |
| `mid-load` (next section) | ~87% open / ~52% reserved | ~1% | RB, just above saturation; small standard overflow trickles to EBs |
| `severe-congestion` (main results) | ~98% | ~56% | RB |
| `eb-capacity-stress` | ~98% | ~93% | RB + EB |

<details>
<summary>Show load profile</summary>

```
lowLoad :: ArrivalProcess
lowLoad = ConstantLoad 3.0
```

A constant 3.0 tx/slot. The RB holds ~73 transactions and is produced at ~f, so its throughput saturates near ~3.5 tx/slot; 3.0 fills the RB to ~80% (offered ~82% of RB byte capacity, ~1% of EB capacity) without saturating it; non-trivial, but uncongested.

</details>

| Family | Priority signal | Inclusion | Urgent retained | Urgent latency (blk) | Priority latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp | Settled coeff. range |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| flat-fee | n/a | 98.24% | 58.79% | 1.85 | n/a | 2.9 | 0.0 | 0.0 | 0.000 | 0.000 |
| single-lane-eip1559 | n/a | 98.24% | 58.82% | 1.85 | n/a | 2.9 | 0.9 | 0.1 | 0.118 | 0.015 |
| priority-only-reserved | 5-sample window | 97.68% | 56.77% | 1.95 | 2.00 | 2.9 | 8.7 | 5.2 | 0.938 | 0.319 |
| priority-only-open | 5-sample window | 98.26% | 61.38% | 1.70 | 1.76 | 2.9 | 8.0 | 4.7 | 0.955 | 0.129 |
| priority-only-strict-threshold-rb2 | 5-sample window | 97.58% | 59.80% | 1.79 | 1.85 | 2.9 | 9.6 | 5.4 | 0.955 | 0.229 |
| both-dynamic-reserved | 5-sample window | 97.68% | 56.77% | 1.95 | 2.00 | 2.9 | 8.7 | 5.2 | 0.938 | 0.319 |
| both-dynamic-open | 5-sample window | 98.26% | 61.43% | 1.70 | 1.75 | 2.9 | 8.3 | 4.7 | 0.931 | 0.132 |
| both-dynamic-strict-threshold-rb2 | 5-sample window | 97.58% | 59.80% | 1.79 | 1.85 | 2.9 | 9.6 | 5.4 | 0.955 | 0.229 |
| priority-only-reserved | instant (worst) | 97.54% | 56.22% | 1.97 | 2.06 | 2.9 | 37.3 | 15.3 | 0.997 | 0.713 |

The strict-threshold rows are identical across the two families: at 3 tx/slot standard traffic never touches the RB, and the EB-dominated utilisation signal keeps the standard controller at its absolute coefficient floor, so the both-dynamic design degenerates to its priority-only counterpart. The headline of this regime is that the EB threshold repairs plain reservation's regression: +3.03 ± 1.11 percentage points urgent retained value over reserved (ten of ten seeds), with urgent latency down from 1.95 to 1.79 blocks, restoring statistical parity with the flat-fee baseline (+1.01 ± 1.46, confidence interval spanning zero). Fewer, fuller EBs mean fewer certificates consuming RB space. The cost falls on the standard lane, whose transactions wait for a worthwhile batch: mean standard latency rises from 3.04 blocks under reserved to 3.15, and the p95 wait rises by about four slots. That is the fairness price of keeping RBs urgent-only at all times.

---

### Mid load (just above RB saturation) ###

This load exists to exercise the EB threshold in the regime where it matters most. At 5 tx/slot the RB is just past its ~3.5 tx/slot throughput, so a small standard overflow continuously trickles toward the EB path: under plain reservation every trickle triggers a thin EB whose certificate consumes ranking-block space, while under the threshold rule those transactions queue until an EB is worth its certificate. In this model a certificate does not share its RB with transactions, so the accounting is direct: over the ten seeds, plain reservation spends 529 of ~1,036 RB opportunities on certificates, while the RB/2 threshold spends 494, freeing 35 additional transaction-carrying RBs (the saving is modest here because all standard traffic must still flow through EBs; the threshold trims only the thin ones).

<details>
<summary>Show load profile</summary>

```
{
  "name": "mid-load",
  "description": "Constant 5 tx/slot: just above RB saturation (~3.5 tx/slot), so a small standard overflow accumulates toward the EB threshold without congesting the EB path",
  "load": {
    "type": "constant",
    "rate": 5.0
  }
}
```

</details>

| Family | Priority signal | Inclusion | Urgent retained | Urgent latency (blk) | Priority latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp | Settled coeff. range |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| flat-fee | n/a | 97.46% | 52.84% | 2.18 | n/a | 4.8 | 0.0 | 0.0 | 0.000 | 0.000 |
| single-lane-eip1559 | n/a | 97.46% | 52.91% | 2.17 | n/a | 4.8 | 2.3 | 0.0 | 0.226 | 0.005 |
| priority-only-reserved | 5-sample window | 97.28% | 54.98% | 2.04 | 2.16 | 4.8 | 14.4 | 6.3 | 0.960 | 0.514 |
| priority-only-open | 5-sample window | 97.50% | 56.94% | 1.92 | 2.07 | 4.8 | 14.3 | 6.4 | 0.962 | 0.515 |
| priority-only-strict-threshold-rb2 | 5-sample window | 97.24% | 55.88% | 1.99 | 2.10 | 4.8 | 16.7 | 6.5 | 0.960 | 0.710 |
| both-dynamic-reserved | 5-sample window | 97.28% | 54.98% | 2.04 | 2.16 | 4.8 | 14.4 | 6.3 | 0.960 | 0.514 |
| both-dynamic-open | 5-sample window | 97.50% | 56.94% | 1.93 | 2.07 | 4.8 | 14.9 | 6.3 | 0.945 | 0.518 |
| both-dynamic-strict-threshold-rb2 | 5-sample window | 97.24% | 55.88% | 1.99 | 2.10 | 4.8 | 16.7 | 6.5 | 0.960 | 0.710 |

The strict-threshold variants improve on plain reservation (+0.90 ± 0.73 percentage points urgent retained value, eight of ten seeds) and clearly beat flat fee (+3.04 ± 1.17, ten of ten seeds, with urgent latency down from 2.18 to 1.99 blocks). As at low load, the two controller families are identical here: standard traffic never touches the RB, so the standard controller stays at its floor. The open variant leads the strict threshold beyond noise in this regime (-1.06 ± 0.83 percentage points, two of ten seeds better) - together with low load, this is where the measured price of ledger enforceability sits, and we accept it, since the open design cannot prevent bribery.

---

### EB-stressing load ###

The `eb-capacity-stress` profile differs from `severe-congestion` only in its arrival schedule (same variant set, seeds, and caps). Rather than one flat burst, it cycles through repeated high-rate peaks separated by troughs, with peak demand (~396 tx/slot) roughly 2.5× the `severe-congestion` burst rate (160), which is what drives demand against the endorser-block byte capacity.

The results of this load broadly follow the `severe-congestion` results, but with two differences worth noting.

First, single-lane EIP-1559 (34.63% urgent retained value, 3.35 blocks) beats both priority-only families here. When the EB is the binding resource, pricing the standard lane matters more than offering a priority lane, since it is standard demand that is saturating the system. This is the strongest evidence in this study for preferring the both-dynamic family over priority-only.

Second, the EB threshold is essentially inert here: the prospective EB payload clears half the RB byte cap at nearly every opportunity, so the strict-threshold variant reproduces plain reservation in nine of ten seeds bit-identically (the tenth diverges briefly in a trough). The open variant leads nominally (38.87% vs 37.50%) but not significantly, so enforceability costs nothing measurable in this regime.

<details>
<summary>Show load profile</summary>

```
ebCapacityStressLoad :: ArrivalProcess
ebCapacityStressLoad =
  BurstLoad
    -- arrivalRateAt sums these bursts; trailing comment is the resulting total rate
    [ Burst { baseRate = 20.0, burstRate =  20.0, burstStart = SlotNo    0, burstEnd = SlotNo 2000, burstEffect = BurstEffect 1 1 }  -- constant 20 baseline
    , Burst { baseRate =  0.0, burstRate =  20.0, burstStart = SlotNo    0, burstEnd = SlotNo  200, burstEffect = BurstEffect 1 1 }  -- shoulder -> 40
    , Burst { baseRate =  0.0, burstRate =  20.0, burstStart = SlotNo 1800, burstEnd = SlotNo 2000, burstEffect = BurstEffect 1 1 }  -- shoulder -> 40
    , Burst { baseRate =  0.0, burstRate = 297.0, burstStart = SlotNo  200, burstEnd = SlotNo  450, burstEffect = BurstEffect 1 1 }  -- peak    -> 317
    , Burst { baseRate =  0.0, burstRate = 376.0, burstStart = SlotNo  650, burstEnd = SlotNo  900, burstEffect = BurstEffect 1 1 }  -- peak    -> 396
    , Burst { baseRate =  0.0, burstRate = 297.0, burstStart = SlotNo 1100, burstEnd = SlotNo 1350, burstEffect = BurstEffect 1 1 }  -- peak    -> 317
    , Burst { baseRate =  0.0, burstRate = 376.0, burstStart = SlotNo 1550, burstEnd = SlotNo 1800, burstEffect = BurstEffect 1 1 }  -- peak    -> 396
    ]
```

</details>

Arrival schedule measured from the runs (fresh first-attempt submissions per slot; taken from the flat-fee runs, whose fixed low fee means almost nothing is declined at generation, so submissions track the raw arrival rate; averaged over the ten seeds). The offered-demand columns translate the arrival count into the resources it places on the endorser block - bytes from each transaction's body size (`txBody._txSize`, mean ~1,233 B) and ex-units from its script (mean ~614k) — expressed as a fraction of the EB capacity per slot (the 12 MB / 9.5 G caps amortised at the ~0.046 EB/slot production rate):

| Slot range | Mean arrivals / slot | Offered bytes/slot (% of EB byte cap) | Offered ex-units/slot (% of EB ex-unit cap) |
|---|---:|---:|---:|
| 0–199 | ~40 | 9% | 6% |
| 200–449 | ~317 | 71% | 45% |
| 450–649 | ~20 | 4% | 3% |
| 650–899 | ~396 | 88% | 56% |
| 900–1099 | ~20 | 4% | 3% |
| 1100–1349 | ~317 | 70% | 45% |
| 1350–1549 | ~20 | 4% | 3% |
| 1550–1799 | ~396 | 88% | 56% |
| 1800–1999 | ~40 | 9% | 6% |

At the peaks, offered byte demand reaches ~88% of EB capacity while ex-unit demand is only ~56% — bytes are ~1.6× tighter, so they saturate first. These are per-slot averages; because EBs are produced only every ~22 slots, demand accumulates into a backlog between them, so the realised per-EB byte fill reaches ~100% at the peaks even though the per-slot offered average is ~88%.

| Family | Priority signal | Inclusion | Urgent retained | Urgent latency (blk) | Priority latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp | Settled coeff. range |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| flat-fee | n/a | 93.42% | 30.12% | 3.88 | n/a | 179.4 | 0.0 | 0.0 | 0.000 | 0.000 |
| single-lane-eip1559 | n/a | 95.05% | 34.63% | 3.35 | n/a | 152.4 | 30.4 | 3.3 | 4.183 | 1.311 |
| priority-only-reserved | 5-sample window | 94.54% | 32.92% | 3.58 | 3.13 | 181.7 | 44.0 | 4.3 | 2.609 | 2.513 |
| priority-only-open | 5-sample window | 93.78% | 33.37% | 3.46 | 2.99 | 180.3 | 35.0 | 4.3 | 2.295 | 2.038 |
| priority-only-strict-threshold-rb2 | 5-sample window | 94.54% | 32.92% | 3.58 | 3.13 | 181.7 | 44.0 | 4.3 | 2.609 | 2.513 |
| both-dynamic-reserved | 5-sample window | 95.67% | 37.57% | 3.05 | 3.78 | 163.8 | 59.3 | 7.3 | 4.253 | 4.857 |
| both-dynamic-open | 5-sample window | 95.76% | 38.87% | 3.03 | 4.42 | 164.5 | 59.9 | 8.5 | 4.420 | 4.173 |
| both-dynamic-strict-threshold-rb2 | 5-sample window | 95.69% | 37.50% | 3.07 | 3.79 | 163.6 | 59.8 | 7.2 | 4.413 | 4.866 |
| priority-only-reserved | 20-sample window (worst) | 91.78% | 30.41% | 3.81 | 4.61 | 176.5 | 14.9 | 1.9 | 3.613 | 2.662 |

---

### Launch-day load (sustained saturation with urgency-skewed demand) ###

**Ten seeds, window-5 signal variants only.**

The `launch-day` profile models the regime a major dApp launch creates: offered demand pinned at the endorser-block byte capacity for longer than the run horizon, with the urgency mix shifted upward. It is the first profile to exercise a burst `urgencyMultiplier` above 1. Its shape and levels follow per-block chain data from the January 2022 SundaeSwap launch ([`launch-day-daily-chain-load.csv`](launch-day-daily-chain-load.csv)): measured byte fullness ran ~31% baseline → ~60% build-up → ~88% pre-launch surge → pinned at ~93% from launch onward, mapped here onto the EB byte capacity (~36% → ~65% → ~96% → ~134% onset → ~100% plateau). The mapping preserves the event's saturation fractions, not its absolute volume: the 2022 chain's ~195k transactions per day would not stress linear-Leios at all, so the profile models a launch event that saturates this chain's capacity the way SundaeSwap saturated the 2022 chain's. The onset overshoot and the urgency multipliers (×2 surge and plateau, ×4 onset) are modelling assumptions: fullness data is capacity-pinned and cannot show either.

<details>
<summary>Show load profile</summary>

```json
{
  "name": "launch-day",
  "load": {
    "type": "burst",
    "bursts": [
      { "baseRate": 0, "burstRate": 160, "burstStart": 0,    "burstEnd": 300,  "burstEffect": { "valueMultiplier": 1, "urgencyMultiplier": 1 } },
      { "baseRate": 0, "burstRate": 290, "burstStart": 300,  "burstEnd": 700,  "burstEffect": { "valueMultiplier": 1, "urgencyMultiplier": 1 } },
      { "baseRate": 0, "burstRate": 430, "burstStart": 700,  "burstEnd": 950,  "burstEffect": { "valueMultiplier": 1, "urgencyMultiplier": 2 } },
      { "baseRate": 0, "burstRate": 600, "burstStart": 950,  "burstEnd": 1150, "burstEffect": { "valueMultiplier": 1, "urgencyMultiplier": 4 } },
      { "baseRate": 0, "burstRate": 450, "burstStart": 1150, "burstEnd": 2000, "burstEffect": { "valueMultiplier": 1, "urgencyMultiplier": 2 } }
    ]
  }
}
```

</details>

Measurement notes: the time-varying multiplier fragments the urgency bands, so the summary's urgency-class metrics are unusable here; urgent columns are recomputed from the event streams by base band (high + critical, ~8% of demand). And because the dynamic-standard variants shed 44–50% of demand before submission, ratio metrics have unequal denominators; retained value is therefore expressed against the offered value of the identical seeded schedule (1.34T lovelace).

| Family | Priority signal | Retained value (% of offered) | Units submitted | Priority admit rate | Priority latency (blk) | Standard latency (blk) | Tx/slot | Shock count |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| flat-fee | n/a | 53.4% | 758k | n/a | n/a | 5.44 | 242.7 | 0.0 |
| single-lane-eip1559 | n/a | 57.2% | 376k | n/a | n/a | 3.22 | 180.5 | 12.2 |
| priority-only-reserved | 5-sample window | 53.6% | 758k | 22.1% | 3.59 | 5.59 | 243.8 | 21.2 |
| priority-only-open | 5-sample window | 55.7% | 758k | 22.8% | 3.11 | 5.29 | 251.4 | 17.8 |
| priority-only-strict-threshold-rb2 | 5-sample window | 53.6% | 758k | 22.1% | 3.59 | 5.59 | 243.8 | 21.2 |
| both-dynamic-reserved | 5-sample window | 59.2% | 425k | 66.2% | 3.65 | 3.29 | 196.1 | 44.5 |
| both-dynamic-open | 5-sample window | 59.7% | 423k | 66.2% | 3.26 | 3.39 | 196.5 | 51.2 |
| both-dynamic-strict-threshold-rb2 | 5-sample window | 59.2% | 425k | 66.2% | 3.65 | 3.29 | 196.1 | 44.5 |

As under `eb-capacity-stress`, the strict-threshold variants reproduce plain reservation bit-identically (all ten seeds): saturated EBs always clear the announcement threshold, so the low-load repair costs nothing here.

**How the mechanism wins changes at saturation.** At every other contended load the priority lane's value shows up as latency. Here the lanes converge (3.65 vs 3.29 blocks reserved; 3.26 vs 3.39 open), because urgency-skewed demand overwhelms the ranking block's share of throughput, yet both-dynamic still clearly beats flat fee (+5.83 ± 4.22 percentage points of offered value, eight of ten seeds), through admission rather than speed:

| Family | Urgent units submitted | Ever included | Mean delay (blk) | Urgent retained (G lovelace) |
|---|---:|---:|---:|---:|
| flat-fee | 52.0k | 50.6% | 4.34 | 22.14 |
| single-lane-eip1559 | 17.8k | 92.6% | 3.26 | 23.43 |
| priority-only-reserved | 52.4k | 50.4% | 4.30 | 23.62 |
| priority-only-open | 52.2k | 53.6% | 3.85 | 26.63 |
| both-dynamic-reserved | 21.6k | 87.1% | 3.22 | 26.67 |
| both-dynamic-open | 21.4k | 87.8% | 3.06 | 27.77 |

Under flat fee, nearly half of urgent demand is never included at all: it queues in a jammed mempool, decays, and abandons. Under both-dynamic, the rising standard quote makes low-surplus demand decline to submit (paying or abstaining at the posted price is itself the urgency signal), and what remains is included with near-certainty (87.1% vs 50.6%), a block sooner, retaining +20% urgent-band value over flat fee (26.67G vs 22.14G). The benefit degrades gracefully from speed into admission.

**Reservation over a static standard lane delivers nothing at this load.** Priority-only-reserved is statistically indistinguishable from the flat-fee baseline (+0.21 ± 3.94 percentage points, five of ten seeds): the enforcement machinery and premium fees buy an outcome no better than no mechanism at all, and unlike the low-load regression the EB threshold cannot help. A statically-priced lane can neither shed demand nor be evicted, so standard traffic squats in the shared mempool and priority transactions bounce at admission (22.1% admitted vs 66.2% for both-dynamic under the same reservation rule), leaving the reserved ranking blocks starved behind a jammed front door. Priority-only-open shares the jam (22.8%) but survives it, because its RBs may backfill with standard transactions: reservation is what turns the jam into starvation. Within the designs tested, a reserved RB partition is only worth having when standard-lane mempool occupancy can be repriced out from under it: ledger enforceability requires the both-dynamic family, which delivers +5.83 ± 4.22 under the same reservation rule. The open variant's lead over reserved is meanwhile within noise (+0.49 ± 3.43), so as under `eb-capacity-stress`, enforceability costs nothing measurable in this regime. (Caveats: the sim models neither transaction validity intervals, whose expiry would give a static lane some mempool turnover, nor certificates sharing RBs with transactions; see the mid-load note.)

The demand-fate and value panels for a representative seed make the admission story visible. In the first figure, note the Priority (Pri) rows: under priority-only-reserved, priority demand itself is heavily abandoned, because it bounces at admission behind the standard-lane jam.

![Demand fate and retained value by urgency class under launch-day load, priority-only-reserved, seed 2: heavy abandonment and lost value across both Standard and Priority classes](figures/launch-day-priority-only-reserved-seed-2.png)

![Demand fate and retained value by urgency class under launch-day load, both-dynamic-strict-threshold, seed 2: most demand included and most value retained](figures/launch-day-both-dynamic-strict-threshold-seed-2.png)

---

### Parameter stress test (controller settings and the threshold rule) ###

**Ten seeds, both-dynamic-strict-threshold window-5 only; low, severe-congestion, launch-day, and eb-capacity-stress loads.**

Everything above stresses the mechanism along the load axis while holding the controller at a single setting: target utilisation 0.5, max-change denominator 8, EB announcement threshold 45,056 B (half the RB byte cap). This section stresses the parameter axis instead. It answers two questions: whether the recommendation depends on a fragile parameter point (our elimination criteria require that it must not), and how the EB threshold should be specified when the controller is retuned - as a constant, or as a function of the controller's headroom.

We sweep a lockstep grid over target utilisation {0.25, 0.5, 0.75} and max-change denominator {4, 8, 16}, applied to both controllers. (The max-change denominator D bounds the controller's per-block price step: one update moves a lane's coefficient by at most ±1/D of its current value, so the swept values allow at most ±25%, ±12.5%, and ±6.25% per block respectively - a lower denominator is a faster, twitchier controller.) The EB threshold is derived at each grid point from the controller headroom, (1 - targetUtilisation) × |RB|: 67,584 B at 0.25, 45,056 B at 0.5, 22,528 B at 0.75. Two isolation variants pin the threshold at 45,056 B while the controller moves off-default (target utilisation 0.25 and 0.75, denominator 8), separating the mechanism's own robustness from the threshold rule's contribution. Sweep manifest: `config/sweeps/param-robustness.json`. The (0.5, 8) point is the anchor: the calibration behind every comparison table in this report. It reproduces the severe-congestion main-table row exactly (51.55% urgent retained value), validating the invocation.

Low load:

| Target util | Denom | EB threshold (B) | Inclusion | Urgent retained | Urgent latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 4 | 67,584 | 97.41% | 58.23% | 1.99 | 2.9 | 63.1 | 7.8 | 3.452 |
| 0.25 | 8 | 67,584 | 97.38% | 59.76% | 1.84 | 2.9 | 42.1 | 6.4 | 1.507 |
| 0.25 | 16 | 67,584 | 97.11% | 60.24% | 1.77 | 2.9 | 3.3 | 2.2 | 0.937 |
| 0.5 | 4 | 45,056 | 97.50% | 59.93% | 1.80 | 2.9 | 27.7 | 7.5 | 1.013 |
| 0.5 | 8 | 45,056 | 97.58% | 59.80% | 1.79 | 2.9 | 9.6 | 5.4 | 0.955 |
| 0.5 | 16 | 45,056 | 97.81% | 59.81% | 1.79 | 2.9 | 0.0 | 0.9 | 0.942 |
| 0.75 | 4 | 22,528 | 98.02% | 59.08% | 1.86 | 2.9 | 3.8 | 3.3 | 0.976 |
| 0.75 | 8 | 22,528 | 98.14% | 58.78% | 1.87 | 2.9 | 5.0 | 0.0 | 0.000 |
| 0.75 | 16 | 22,528 | 98.13% | 58.58% | 1.86 | 2.9 | 0.0 | 0.0 | 0.000 |
| 0.25 | 8 | 45,056 (fixed) | 97.35% | 57.76% | 1.94 | 2.9 | 43.9 | 5.9 | 1.461 |
| 0.75 | 8 | 45,056 (fixed) | 98.26% | 60.64% | 1.77 | 2.9 | 5.0 | 0.0 | 0.000 |

Severe congestion:

| Target util | Denom | EB threshold (B) | Inclusion | Urgent retained | Urgent latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 4 | 67,584 | 94.02% | 46.23% | 2.49 | 82.1 | 133.7 | 8.9 | 64.439 |
| 0.25 | 8 | 67,584 | 96.46% | 48.46% | 2.46 | 79.3 | 77.7 | 6.7 | 31.417 |
| 0.25 | 16 | 67,584 | 98.16% | 52.52% | 2.33 | 87.2 | 12.8 | 2.3 | 8.525 |
| 0.5 | 4 | 45,056 | 98.68% | 48.55% | 2.56 | 123.4 | 82.5 | 7.8 | 5.312 |
| 0.5 | 8 | 45,056 | 99.09% | 51.55% | 2.44 | 122.7 | 15.4 | 5.4 | 2.042 |
| 0.5 | 16 | 45,056 | 99.20% | 50.97% | 2.52 | 123.4 | 0.0 | 0.5 | 1.233 |
| 0.75 | 4 | 22,528 | 98.84% | 49.02% | 2.59 | 127.2 | 14.9 | 5.2 | 1.704 |
| 0.75 | 8 | 22,528 | 98.85% | 48.73% | 2.57 | 126.7 | 3.2 | 0.0 | 0.000 |
| 0.75 | 16 | 22,528 | 99.13% | 49.05% | 2.54 | 127.6 | 0.0 | 0.0 | 0.000 |

The fixed-threshold isolation variants are omitted from the severe-congestion table because they are bit-identical to their formula counterparts: under sustained backlog the prospective EB payload clears every threshold in the swept range (22,528-67,584 B), so the gate never intervenes at any setting. The threshold is inert under sustained congestion regardless of where it sits in this range; everything it does, it does at light and moderate loads.

Launch-day (retained value expressed against the offered value of the identical seeded schedule, 1.34T lovelace, as in the launch-day section; the time-varying urgency multiplier makes urgency-sliced summary metrics unusable here):

| Target util | Denom | EB threshold (B) | Retained (% of offered) | Retained (G lovelace) | Inclusion | Tx/slot | Priority latency (blk) | Standard latency (blk) | Shock count | Osc. max amp |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 4 | 67,584 | 30.3% | 405.71 | 80.27% | 90.1 | 2.58 | 3.13 | 164.9 | 1486.2 |
| 0.25 | 8 | 67,584 | 40.8% | 547.13 | 95.36% | 107.9 | 2.70 | 2.90 | 93.4 | 46.4 |
| 0.25 | 16 | 67,584 | 43.0% | 576.31 | 96.21% | 116.9 | 2.71 | 2.95 | 25.5 | 18.8 |
| 0.5 | 4 | 45,056 | 62.1% | 832.80 | 85.21% | 227.8 | 3.98 | 3.57 | 133.7 | 21.6 |
| 0.5 | 8 | 45,056 | 59.3% | 794.54 | 92.68% | 196.1 | 3.65 | 3.29 | 44.5 | 10.3 |
| 0.5 | 16 | 45,056 | 59.7% | 800.00 | 94.10% | 197.5 | 2.85 | 3.54 | 0.0 | 6.5 |
| 0.75 | 4 | 22,528 | 59.8% | 800.85 | 74.09% | 253.3 | 5.49 | 5.09 | 24.4 | 4.5 |
| 0.75 | 8 | 22,528 | 65.1% | 871.67 | 83.32% | 258.5 | 3.86 | 4.36 | 1.8 | 0.0 |
| 0.75 | 16 | 22,528 | 59.9% | 802.45 | 75.37% | 251.6 | 4.07 | 5.11 | 0.0 | 0.0 |

For reference, flat fee retains 53.4% of offered value (715.6G) at this load with 64.1% inclusion. The tu75-d8 fixed-threshold isolation variant is bit-identical to its formula counterpart here; the tu25-d8 pair differs marginally (544.62G fixed vs 547.13G formula).

EB-capacity stress:

| Target util | Denom | EB threshold (B) | Inclusion | Urgent retained | Urgent latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 0.25 | 4 | 67,584 | 73.45% | 34.24% | 2.69 | 82.0 | 160.5 | 8.6 | 398.684 |
| 0.25 | 8 | 67,584 | 82.99% | 39.47% | 2.63 | 87.2 | 124.9 | 7.1 | 73.799 |
| 0.25 | 16 | 67,584 | 96.53% | 46.22% | 2.59 | 100.6 | 31.9 | 4.6 | 13.053 |
| 0.5 | 4 | 45,056 | 91.47% | 31.87% | 3.40 | 165.8 | 121.5 | 8.4 | 9.483 |
| 0.5 | 8 | 45,056 | 95.69% | 37.50% | 3.07 | 163.6 | 59.8 | 7.2 | 4.413 |
| 0.5 | 16 | 45,056 | 96.78% | 38.50% | 3.02 | 166.6 | 0.0 | 3.5 | 2.225 |
| 0.75 | 4 | 22,528 | 92.27% | 30.78% | 3.64 | 176.6 | 23.0 | 6.3 | 1.975 |
| 0.75 | 8 | 22,528 | 93.84% | 31.48% | 3.50 | 179.0 | 7.1 | 0.0 | 0.000 |
| 0.75 | 16 | 22,528 | 93.64% | 31.39% | 3.61 | 179.8 | 0.0 | 0.0 | 0.000 |

The anchor reproduces the main eb-capacity-stress strict-threshold row exactly (37.50%). The tu75-d8 isolation variant is bit-identical to its formula counterpart; the tu25-d8 pair differs marginally (39.15% fixed vs 39.47% formula).

Paired seed deltas, same conventions as the main results table:

| Comparison | Metric | Mean paired delta ± 95% CI | Seeds better |
|---|---|---:|---:|
| formula threshold vs fixed 45,056 B, target 0.25 (low load) | Urgent retained (pp) | +2.00 ± 0.99 | 9/10 |
| formula threshold vs fixed 45,056 B, target 0.25 (low load) | Urgent latency (blk) | -0.10 ± 0.06 | 10/10 |
| formula threshold vs fixed 45,056 B, target 0.75 (low load) | Urgent retained (pp) | -1.85 ± 0.94 | 0/10 |
| formula threshold vs fixed 45,056 B, target 0.75 (low load) | Urgent latency (blk) | +0.10 ± 0.06 | 0/10 |
| tu 0.25 / d 4 vs anchor (severe) | Urgent retained (pp) | -5.32 ± 2.29 | 1/10 |
| tu 0.25 / d 8 vs anchor (severe) | Urgent retained (pp) | -3.09 ± 3.40 | 3/10 |
| tu 0.25 / d 16 vs anchor (severe) | Urgent retained (pp) | +0.97 ± 1.79 | 8/10 |
| tu 0.5 / d 4 vs anchor (severe) | Urgent retained (pp) | -3.00 ± 0.92 | 0/10 |
| tu 0.5 / d 16 vs anchor (severe) | Urgent retained (pp) | -0.58 ± 1.26 | 4/10 |
| tu 0.75 / d 4 vs anchor (severe) | Urgent retained (pp) | -2.53 ± 1.56 | 0/10 |
| tu 0.75 / d 8 vs anchor (severe) | Urgent retained (pp) | -2.82 ± 2.33 | 0/10 |
| tu 0.75 / d 16 vs anchor (severe) | Urgent retained (pp) | -2.50 ± 1.71 | 1/10 |
| tu 0.25 / d 4 vs flat-fee (launch-day) | Retained (pp of offered) | -23.19 ± 4.49 | 0/10 |
| tu 0.25 / d 8 vs flat-fee (launch-day) | Retained (pp of offered) | -12.64 ± 3.58 | 0/10 |
| tu 0.25 / d 16 vs flat-fee (launch-day) | Retained (pp of offered) | -10.46 ± 3.25 | 0/10 |
| tu 0.5 / d 4 vs flat-fee (launch-day) | Retained (pp of offered) | +8.68 ± 3.87 | 9/10 |
| tu 0.5 / d 8 vs flat-fee (launch-day) | Retained (pp of offered) | +5.83 ± 4.22 | 8/10 |
| tu 0.5 / d 16 vs flat-fee (launch-day) | Retained (pp of offered) | +6.23 ± 3.34 | 9/10 |
| tu 0.75 / d 4 vs flat-fee (launch-day) | Retained (pp of offered) | +6.30 ± 5.72 | 7/10 |
| tu 0.75 / d 8 vs flat-fee (launch-day) | Retained (pp of offered) | +11.58 ± 3.00 | 10/10 |
| tu 0.75 / d 16 vs flat-fee (launch-day) | Retained (pp of offered) | +6.42 ± 4.36 | 9/10 |
| tu 0.25 / d 16 vs anchor (eb-capacity-stress) | Urgent retained (pp) | +8.72 ± 3.91 | 10/10 |
| tu 0.5 / d 4 vs anchor (eb-capacity-stress) | Urgent retained (pp) | -5.63 ± 2.91 | 0/10 |
| tu 0.5 / d 16 vs anchor (eb-capacity-stress) | Urgent retained (pp) | +1.00 ± 3.08 | 4/10 |
| tu 0.75 / d 8 vs anchor (eb-capacity-stress) | Urgent retained (pp) | -6.02 ± 2.97 | 0/10 |

Four findings.

**The recommendation is not parameter-fragile within target utilisation 0.5-0.75 and denominator 8-16.** Every point in that sub-grid beats flat fee at both contended loads (severe congestion: 48.7-51.6% urgent retained vs 44.32%; launch-day: +5.83 to +11.58 pp of offered, all confidence intervals excluding zero) and holds low-load urgent retention within about a point of the flat-fee aggregate (58.6-60.6% vs 58.79%; unpaired comparison, see caveats). Within the envelope, target 0.75 trades roughly 2.5-2.8 pp of urgent retention under severe congestion for the strongest launch-day result (+11.58 ± 3.00, ten of ten seeds) at a visibly lower inclusion rate (83.3% vs 92.7%); the default 0.5 remains the recommended point. One qualifier: under eb-capacity-stress, target 0.75 loses ~6 pp against the anchor (-6.02 ± 2.97, zero of ten seeds), landing at ~31.4% urgent retained against flat fee's 30.12% - approximate parity, not advantage. The envelope criterion is therefore worth stating precisely: inside the envelope the mechanism never falls below the flat-fee baseline at any load. At target 0.5 the advantage holds at every load; at 0.75 it holds at every load except EB-stressing traffic, where it narrows to parity - the same worst case the design itself accepts at low load.

**Outside the envelope the mechanism fails informatively, not gracefully.** Target utilisation 0.25 falls below the flat-fee baseline at launch-day at every denominator (-10.46 to -23.19 pp of offered, zero of ten seeds better): aiming for quarter-full blocks makes the controller shed demand the chain could have served, and at (0.25, 4) the price coefficient melts down outright (oscillation max amplitude ~1,486). Under severe congestion the same settings collapse throughput to 79-87 tx/slot against ~123 elsewhere. Denominator 4 fails on stability at every load (shock counts 5-9× the anchor's) and costs retention under severe congestion (-3.00 ± 0.92 pp, zero of ten). These corners are excluded, and the exclusion matters for governance: a parameter update that wanders here does not degrade the mechanism, it inverts it.

**The threshold rule needs a floor: specify max((1 - targetUtilisation) × |RB|, |RB| / 2).** The isolation pairs test the headroom formula's two halves separately. At target 0.25, the formula's larger threshold (67,584 B) beats the fixed 45,056 B in nine of ten seeds (+2.00 ± 0.99 pp urgent retained, -0.10 ± 0.06 blocks urgent latency). At target 0.75, the formula's smaller threshold (22,528 B) loses to the fixed 45,056 B in ten of ten seeds (-1.85 ± 0.94 pp, +0.10 ± 0.06 blocks). So the certificate-worth property tracks headroom in one direction only: the threshold must rise when the controller leaves more headroom, and must not follow shrinking headroom below the half-RB floor.

The event streams show the mechanism directly, in the same accounting style as the mid-load section. Mean urgent payload per transaction-carrying RB tracks the target almost exactly (27.7% fill at target 0.25, 46.0% at 0.5, 62.0% at 0.75), so at low targets the urgent lane needs roughly 2.2× as many RB opportunities to move the same bytes, and each certificate displaces a proportionally larger share of its block supply. Raising the threshold at target 0.25 converts certificates into transaction-RBs (-7.3 ± 1.8 certificate-RBs per run, 58.8 vs 51.5 transaction-RBs); lowering it at target 0.75 does the reverse (+7.4 ± 2.6, 60.2 vs 67.6). Those ±7 RB opportunities out of ~96 per run are what the ±2 pp retention deltas are made of:

| Config | EB threshold (B) | Cert-RBs | Tx-RBs | Cert share of RBs | Urgent fill per tx-RB | EBs announced | Mean EB payload (kB) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 0.25 / 8, formula | 67,584 | 37.5 | 58.8 | 38.9% | 27.7% | 72.7 | 152.8 |
| 0.25 / 8, fixed | 45,056 | 44.8 | 51.5 | 46.5% | 27.0% | 86.6 | 135.0 |
| 0.5 / 8 (anchor) | 45,056 | 37.7 | 58.6 | 39.1% | 46.0% | 74.2 | 129.5 |
| 0.75 / 8, formula | 22,528 | 36.1 | 60.2 | 37.5% | 62.0% | 69.6 | 112.9 |
| 0.75 / 8, fixed | 45,056 | 28.7 | 67.6 | 29.8% | 60.9% | 57.6 | 126.3 |

One refinement: the certificate rate responds to the threshold sublinearly. Scaling as 1/threshold would predict formula-to-fixed certificate ratios of 0.67 (target 0.25) and 2.00 (target 0.75); the measured ratios are 0.84 and 1.26. Mean announced payloads (113-153 kB) sit far above every threshold tested because announcement is limited by EB production opportunities as well as by the threshold: standard traffic often accumulates well past the threshold before the next opportunity arrives, so moving the threshold only changes behaviour in the intervals where accumulation lands between the two values. The threshold is a guard rail, not a proportional dial - which is itself an argument for the conservative max() rule over any finely-tuned per-parameter optimum.

**Denominator 16 replaces 8 as the recommended calibration.** Its retention is statistically indistinguishable from the anchor's at every swept load (severe: -0.58 ± 1.26 pp; eb-capacity-stress: +1.00 ± 3.08 pp), with zero shocks at three of the four loads and a fraction of the oscillation amplitude. The eb-capacity-stress run was the test it was most likely to fail - fast transients punish a slow controller - and it passed with zero shocks and a quarter of the anchor's amplitude. Since the stability gain costs nothing in welfare at baseline demand elasticity, we adopt denominator 16 in the recommendation; the comparison tables throughout this report remain on the denominator-8 anchor, which is welfare-equivalent at that elasticity. (The mid-load profile was not part of this sweep, so there the equivalence is inferred rather than measured; and the demand-elasticity stress test below shows the equivalence is elasticity-scoped - under an extreme high-value mix, denominator 8 retains ~2 pp more at a large stability cost. The rationale for keeping 16 is given there.) (The same run shows (0.25, 16) beating the anchor by +8.72 ± 3.91, ten of ten seeds. This is the EB-stressing section's mechanism at work, dialled up: when the endorser block is the binding resource, it is the standard-lane price that sheds the saturating demand, and a lower standard target sheds more of it. It does not rescue target 0.25, which launch-day still disqualifies outright.)

**Cross-lane multiplier floor: tested and rejected.** The mechanism-design document carries a cross-lane multiplier floor default (urgent coefficient ≥ 16 × standard coefficient) and the prototype enforces one at 3×, but every experiment above runs with the floor off (`multiplierFloor: null`) - the component had no evidence behind it in either direction. (This is distinct from the absolute coefficient floor, which bounds each lane's own quote below, and from the half-RB floor in the threshold expression.) A three-point test (off / 3× / 16×) on the recommended configuration rejects it decisively (sweep: `config/sweeps/multiplier-floor.json`). At low load the standard coefficient rests at its minimum, so the floor pins the urgent price at a multiple of the minimum exactly when it should approach parity: -9.25 ± 1.81 pp urgent retained at 3× and -15.30 ± 2.32 pp at 16×, zero of ten seeds better in either case. Under severe congestion, 3× costs -2.80 ± 1.39 pp, and 16× drives urgent retention to 44.28%: statistically indistinguishable from the flat-fee control (44.32%), erasing the mechanism's entire advantage. At launch-day the differences are within noise, with the urgent lane priced out of use altogether under 16×. The floor does not enter the specification.

The max-of-two correction is not a 1× multiplier floor. It constrains only the maximum fee an rb-only urgent transaction must cover; it does not alter either controller coefficient. A separate forced-inversion regression test covers fee validity and settlement.

To check the implementation correction against the historical results, we ran one focused matched experiment. The saved pre-correction launch-day denominator-8 anchor and both corrected candidates use the same ten seeds and 2,000-slot horizon. Three saved legacy traces directly demonstrate quote inversion on this workload; the other seven legacy seeds retain summary metrics only. One candidate leaves the controllers independent and applies max(standard quote, urgent quote) only to wallet choice and fee-cap validity; the other additionally clamps the urgent coefficient to at least 1× the standard coefficient. The corrected sweep ran summary-only, retaining no JSONL traces. Cells are paired mean changes with two-sided 95% paired-t confidence intervals.

| Corrected rule vs pre-correction | Overall retained value (pp) | Priority retained value (pp) | Unit service rate (pp) | Priority service rate (pp) | Mean latency (blocks) | Throughput (tx/slot) |
|---|---:|---:|---:|---:|---:|---:|
| max-of-two, no floor | -0.234 [-1.225, +0.756] | +2.181 [-0.664, +5.025] | -0.016 [-0.930, +0.898] | +2.494 [-0.528, +5.515] | +0.045 [-0.139, +0.229] | -1.546 [-6.573, +3.480] |
| 1× controller floor | +0.552 [-1.205, +2.309] | +0.697 [-1.449, +2.844] | +1.219 [-1.106, +3.545] | +1.072 [-3.073, +5.216] | -0.040 [-0.246, +0.166] | +3.382 [-3.096, +9.861] |

No corrected-versus-pre-correction interval in the table excludes zero, so this smoke found no statistically detectable difference in the displayed metrics. The appropriate conclusion is not equivalence: the max-of-two priority-retention interval, for example, still permits an increase as large as 5.025 percentage points. The 1× floor also changes submission behaviour relative to max-of-two/no-floor, reducing mean within-seed priority submissions by 15.67% [6.11%, 25.24%]. We select max-of-two on the semantic ground that it fixes fee-cap validity while keeping the controllers independent, not because this smoke establishes that the alternatives are welfare-equivalent. The tables above remain explicitly pre-correction and do not need to be represented as post-correction results.

We repeated the independent-controller max-of-two candidate at the recommended controller denominator 16 against its archived pre-correction launch-day baseline, using the same seeds 0-9 and 2,000-slot horizon. Its effective configuration was byte-identical to the baseline's, and every one of the 55 reported scalars in every seed was exactly unchanged: 0 differences among 550 paired comparisons. This is a useful confirmation for that controller calibration, not a universal equivalence claim. Unlike the denominator-8 baseline, no D16 event traces remain to demonstrate quote-inversion exposure, and the exact legacy simulator revision was not recorded, so the denominator-8 traces provide the direct-exposure evidence. The D16 check omits the K = 10 announcement age escape because it was absent from the archived baseline and adding it would confound the fee-cap comparison. The [D16 evidence record](experiment-results/cross-lane-inversion-d16-baseline.json) preserves the decision-facing per-seed scalars, result, and checksums.

Finally, we ran the complete canonical D16/K10 configuration as a post-correction launch-day integration check on the same seeds and horizon. Its effective configuration differs from the corrected D16 reference only by `ebAgeEscapeRbIntervals: 10`. All 55 reported scalars in all ten seeds were exactly unchanged against both the corrected D16/no-K10 and pre-correction D16 references: 0 differences among 550 paired comparisons in either case. This verifies that the assembled recommendation executes and introduces no observed outcome change here; summary-only output does not directly show whether the K = 10 condition was evaluated, so the trickle sweep remains the binding-case evidence. The [canonical integration evidence](experiment-results/canonical-final-smoke.json) preserves its per-seed metrics, result, and provenance.

The exact denominator-8 experiment is `config/sweeps/cross-lane-inversion-smoke.json`; from `abstract-sim-hs`, run `./scripts/smoke_cross_lane_inversion.sh --out sweep-results/cross-lane-inversion-smoke-launch-day-rerun`. The D16 check is `config/sweeps/cross-lane-inversion-smoke-d16.json`; run `./scripts/smoke_cross_lane_inversion_d16.sh --out sweep-results/cross-lane-inversion-smoke-d16-launch-day-rerun`. The integrated check is `config/sweeps/canonical-final-smoke.json`; run `./scripts/smoke_canonical_final.sh --out sweep-results/canonical-final-d16-k10-launch-day-rerun`. They execute only the corrected candidates against preserved references and write paired comparisons without event traces. Every completed local output occupied under 100 KiB.

Caveats. The low-load flat-fee comparison is against the aggregate in the low-load section rather than paired per-seed, because the earlier sweep outputs were not retained; the margins involved are within about a point either way, so we describe low-load behaviour as parity, not improvement. Finally, certificate counts compared across different targets at the same threshold (44.8 at target 0.25 vs 28.7 at 0.75, both at 45,056 B) confound two channels: the threshold mechanics above, and lane migration - a higher target makes the urgent lane cheaper, drawing demand out of the standard lane and shrinking EB traffic on its own. The isolation pairs are clean because they hold the target fixed; cross-target comparisons are not.

---

### Demand elasticity stress test ###

**Ten seeds; severe-congestion and launch-day loads; recommended calibration (target 0.5, denominator 16, window 5), each demand mix paired with its own flat-fee control.**

The stability results above were all produced at one demand elasticity: the default actor calibration. Stability is not a property of the controller alone - the loop gain that decides whether prices converge is the product of the controller's step size and the demand curve's steepness - so elasticity is an environment parameter sitting under every stability claim. This sweep varies it (`config/sweeps/elasticity.json`). The mixes: `base` (the standard calibration), `all10x` (every actor's values scaled 10×, so demand sheds roughly ten times later), and `hv10`/`hv25` (10% and 25% of arrivals at 100× values - a bounded-inelastic tranche with very high but finite willingness to pay). Each mix carries its own flat-fee control because the mix changes offered value: paired deltas must compare like against like. A final variant runs the harshest mix at denominator 8 to test the envelope's fast edge under steep demand. The base pair reproduces its known numbers exactly (50.97% / 44.32% urgent retained under severe congestion), validating the invocation.

Severe congestion (mechanism variants; the paired table below carries the flat-fee comparisons):

| Mix | Denom | Urgent retained | Urgent latency (blk) | Inclusion | Shock count | Osc. max amp | Settled coeff. range |
|---|---:|---:|---:|---:|---:|---:|---:|
| base | 16 | 50.97% | 2.52 | 99.20% | 0.0 | 1.233 | 0.374 |
| all10x | 16 | 49.90% | 2.59 | 99.21% | 0.0 | 8.669 | 5.134 |
| hv10 | 16 | 50.80% | 2.90 | 99.01% | 0.0 | 19.981 | 12.934 |
| hv25 | 16 | 50.27% | 2.77 | 99.10% | 0.0 | 20.614 | 26.499 |
| hv25 | 8 | 52.13% | 2.74 | 98.76% | 29.4 | 68.208 | 56.588 |

Launch-day (retained value in G lovelace; each mix's offered value differs, so only within-mix comparisons are meaningful):

| Mix | Denom | Retained (G) | Matched flat-fee (G) | Inclusion | Shock count | Osc. max amp |
|---|---:|---:|---:|---:|---:|---:|
| base | 16 | 800.00 | 716 | 94.10% | 0.0 | 6.54 |
| all10x | 16 | 7,766.98 | 6,445 | 74.25% | 0.0 | 59.63 |
| hv10 | 16 | 11,777.60 | 7,531 | 92.75% | 0.0 | 14.78 |
| hv25 | 16 | 26,818.68 | 18,195 | 88.19% | 0.0 | 0.00 |
| hv25 | 8 | 28,804.47 | 18,195 | 95.34% | 53.3 | 272.47 |

| Comparison | Metric | Mean paired delta ± 95% CI | Seeds better |
|---|---|---:|---:|
| mech vs matched flat, base (severe) | Urgent retained (pp) | +6.65 ± 2.40 | 9/10 |
| mech vs matched flat, all10x (severe) | Urgent retained (pp) | +6.43 ± 2.03 | 10/10 |
| mech vs matched flat, hv10 (severe) | Urgent retained (pp) | +7.28 ± 2.54 | 10/10 |
| mech vs matched flat, hv25 (severe) | Urgent retained (pp) | +6.13 ± 2.02 | 10/10 |
| mech vs matched flat, base (launch-day) | Retained (G) | +83.53 ± 44.75 | 9/10 |
| mech vs matched flat, all10x (launch-day) | Retained (G) | +1,321.75 ± 444.53 | 10/10 |
| mech vs matched flat, hv10 (launch-day) | Retained (G) | +4,247.09 ± 557.24 | 10/10 |
| mech vs matched flat, hv25 (launch-day) | Retained (G) | +8,623.68 ± 1,156.76 | 10/10 |
| denominator 8 vs 16 at hv25 (severe) | Urgent retained (pp) | +1.85 ± 1.08 | 9/10 |
| denominator 8 vs 16 at hv25 (launch-day) | Retained (G) | +1,985.79 ± 1,363.52 | 9/10 |

Three findings.

**The advantage over flat fee holds at every elasticity tested, and grows with the share of high-value demand.** Against matched controls the mechanism wins at every mix under both loads, and the launch-day progression is steep: +83G at base, +1,322G at all10x, +4,247G at hv10, +8,624G at hv25, ten of ten seeds at every non-base mix. The mechanism delivers the most value exactly when high-willingness-to-pay demand exists, which is the demand an urgency lane is for.

**At denominator 16 the price is shock-free at every elasticity, but its excursions grow with demand steepness.** Shock counts are exactly zero at every mix - the ±6.25% step cap cannot produce a shock - while oscillation amplitude rises from 1.2 (base) to ~20 (hv mixes) and the settled coefficient range from 0.37 to 26.5 under severe congestion: pricing a 100× tranche requires the coefficient to travel a long way, and it does so smoothly but far. Stability claims should therefore be scoped: block-to-block predictability holds at all tested elasticities; a narrow settled band does not. No mix produces runaway pricing - the bounded tranche is always eventually priced.

**Denominators 8 and 16 are welfare-equivalent only at baseline elasticity.** Under the hv25 mix, denominator 8 retains +1.85 ± 1.08 pp more at severe congestion and +1,986 ± 1,364 G more at launch-day (nine of ten seeds each): the faster controller tracks the fast-moving equilibrium better. It pays in stability everywhere: 29-53 shocks and amplitude up to 272 under the same mixes, and 15-60 shocks at baseline mixes where it buys nothing. We keep denominator 16 as the recommendation on the asymmetry of the error modes: choosing 16 in an hv25-shaped world costs a slice of an advantage that remains overwhelming (+6.13 pp / +8,624G over flat fee), while choosing 8 in a baseline-shaped world imposes chronic price shocks at every load for no welfare return; the hv25 mix is a constructed stress bracket rather than a demand forecast (the empirically calibrated launch-day profile at baseline elasticity shows the two statistically tied); and the denominator is an updatable protocol parameter inside a validated envelope, so a persistent steep-demand regime can be met by moving toward 8 without a mechanism change.

The price traces at the hv25 mix show the trade directly (seed 0, shared y-domain; markers are significant direction reversals):

![Per-lane price coefficient under severe congestion at the hv25 mix, max-change denominator 16, seed 0: smooth traces with zero shocks and zero reversal markers despite a wide excursion](figures/hv25-d16-price-seed-0.png)

![Per-lane price coefficient under severe congestion at the hv25 mix, max-change denominator 8, seed 0: reversal markers on both lanes and a late collapse; this run records 33 price moves exceeding 10%](figures/hv25-d8-price-seed-0.png)

Caveats. Low load was not swept: prices rest at the floor there and elasticity barely engages. Mid-load was not swept. The hv mixes are synthetic brackets, not calibrated demand. And the sim's actors respond to price shocks only by re-quoting and resubmitting, so retained-value metrics understate what instability costs real users in fee predictability; welfare ties between calibrations should be read alongside the stability columns, not instead of them.

---

### Trickle loads and the announcement age escape ###

**Ten seeds; trickle-0.1 and trickle-0.5 tx/slot plus the low profile; recommended calibration with age-escape variants K ∈ {5, 10, 20}; plain reservation (threshold 1 B, the K → 0 limit) and flat fee as brackets.**

The pure announcement threshold can starve a trickle: standard traffic that never accumulates to the byte bar pools forever, and anything depending on its outputs waits with it (first observed live in the engineering prototype). The specified repair is a time-gated escape - an EB may be announced below the threshold once at least K ranking blocks have been produced since the last EB announcement, a condition on chain history only. This sweep (`config/sweeps/trickle-aging.json`, using the simulator's new `ebAgeEscapeRbIntervals` policy field) calibrates K and checks that the escape stays inert when the threshold crosses naturally.

| Load | Variant | Inclusion | Unresolved units | Standard latency (blk) | Urgent retained | Standard retained |
|---|---|---:|---:|---:|---:|---:|
| 0.1 tx/slot | no escape | 90.55% | 19.1 | n/a | 66.67% | 0.00% |
| 0.1 tx/slot | K = 5 | 99.22% | 1.6 | 8.41 | 66.06% | 89.67% |
| 0.1 tx/slot | K = 10 | 99.22% | 1.6 | 14.72 | 65.55% | 83.39% |
| 0.1 tx/slot | K = 20 | 99.22% | 1.6 | 22.50 | 65.55% | 74.91% |
| 0.1 tx/slot | plain reservation | 99.22% | 1.6 | 2.94 | 65.52% | 95.81% |
| 0.1 tx/slot | flat fee | 99.26% | 1.5 | 1.00 | 75.54% | 97.54% |
| 0.5 tx/slot | no escape | 97.15% | 28.5 | 4.31 | 73.42% | 93.27% |
| 0.5 tx/slot | K = 10 | 98.94% | 10.7 | 6.60 | 73.42% | 91.21% |
| 0.5 tx/slot | plain reservation | 98.94% | 10.7 | 3.02 | 72.26% | 95.50% |
| 0.5 tx/slot | flat fee | 98.94% | 10.7 | 1.00 | 73.92% | 97.48% |
| low (3 tx/slot) | no escape | 97.81% | 131.4 | 3.15 | 59.81% | 96.19% |
| low (3 tx/slot) | K = 5 | 97.81% | 131.6 | 3.13 | 59.63% | 96.20% |
| low (3 tx/slot) | K = 10 | 97.81% | 131.4 | 3.15 | 59.81% | 96.19% |
| low (3 tx/slot) | K = 20 | 97.81% | 131.4 | 3.15 | 59.81% | 96.19% |

The headline rows are the first and second: with no escape at a 0.1 tx/slot trickle, the standard lane retains 0.00% of its value - total starvation, every pooled transaction unresolved or abandoned - and the escape repairs it: +83.39 ± 8.59 pp standard retained value (K = 10 vs no escape, ten of ten seeds) at no measurable urgent-lane cost (-1.12 ± 1.74 pp, confidence interval spanning zero; certificates are nearly free when ranking blocks run this empty). The K rows form a clean monotone tradeoff (standard wait roughly 1.5 × K slots' worth of blocks against certificate frequency), with no urgent cost at any K.

The starvation and its repair are close to binary in the fate panels (seed 2 shown; identical crop and scale):

![Demand fate and retained value at the 0.1 tx/slot trickle with no age escape, seed 2: every Standard class is entirely unresolved and 0% of standard value is retained](figures/trickle-0p1-thr-noescape-seed-2.png)

![Demand fate and retained value at the 0.1 tx/slot trickle with the age escape at K = 10, seed 2: all Standard units included and most standard value retained](figures/trickle-0p1-thr-k10-seed-2.png)

The inertness check is the strongest available result: at the low profile, K = 10 and K = 20 are **bit-identical to the no-escape runs in every seed** - when standard traffic crosses the threshold naturally, the escape never fires, so it imposes no steady tax outside the regime it exists for. K = 5 fires occasionally at low load (harmlessly: -0.18 pp urgent retained, within noise). K = 10 is therefore adopted as the default: fully inert at ordinary loads, with a ~15-block worst-case standard wait at the deepest trickle tested.

Two honest notes. At 0.1 tx/slot the urgent-retention comparison against flat fee is noise-dominated (-9.99 ± 19.85 pp, roughly twenty urgent-class transactions per run); at 0.5 tx/slot it is tight parity (-0.50 ± 0.80). We report the trickle regime as statistical parity with flat fee, on the same criterion used at low load. And the sim does not model dependency chains, so the deadlock variant of starvation observed in the prototype (dependents of pooled transactions wedging) is repaired here only by implication: the escape unblocks the parents; the chained consequence is untested.

---

### D16/K10 headline rerun ###

The purpose of this rerun was to ensure that same-seed flat-fee and D16/K10 runs faced the same fresh demand and ranking-block opportunities. At heavier loads, different retry counts had advanced the shared random stream differently, so within-seed outcome differences mixed the mechanism effect with different exogenous simulation draws.

After separating the simulator's fresh-demand, ranking-block-production, and retry-jitter random streams, we reran flat fee against the exact recommended D16/K10 configuration over paired seeds 0–9 for 2,000 slots at each headline load. Intervals are two-sided 95% paired-t confidence intervals.

| Load | Retained-value metric | Flat | D16/K10 | D16/K10 − flat (95% CI) | Seeds better |
|---|---|---:|---:|---:|---:|
| Low | Urgent | 59.40% | 59.87% | +0.469 [-1.155, +2.093] pp | 6/10 |
| Mid load | Urgent | 52.32% | 56.01% | +3.687 [+2.630, +4.744] pp | 10/10 |
| Severe congestion | Urgent | 43.56% | 50.85% | +7.288 [+6.000, +8.576] pp | 10/10 |
| EB-capacity stress | Urgent | 29.40% | 37.97% | +8.573 [+6.335, +10.811] pp | 10/10 |
| Launch day | Overall | 51.72% | 59.87% | +8.151 [+6.108, +10.194] pp | 10/10 |

| Load | Urgent latency, flat | Urgent latency, D16/K10 | D16/K10 − flat (95% CI) | Seeds faster |
|---|---:|---:|---:|---:|
| Low | 1.791 | 1.757 | -0.033 [-0.103, +0.036] blocks | 6/10 |
| Mid load | 2.233 | 1.992 | -0.241 [-0.309, -0.173] blocks | 10/10 |
| Severe congestion | 2.983 | 2.502 | -0.481 [-0.596, -0.366] blocks | 10/10 |
| EB-capacity stress | 3.810 | 3.001 | -0.809 [-1.010, -0.608] blocks | 10/10 |

The rerun was successful: low load remained at parity, while retained value and urgent latency improved at every contended load in all ten paired seeds. Launch-day overall retained value also improved in all ten seeds. These results support the recommendation without changing it.

For launch day, both retained-value numerators use each seed's flat-fee retained + lost + unresolved value as the denominator. Summary output does not record fresh samples that decline before first submission, so this is a flat-fee proxy for offered demand. The simulator announces eligible EBs eagerly; producer withholding is not modelled.

From `abstract-sim-hs`, rerun with `./scripts/run_canonical_headlines.sh --out sweep-results/canonical-headlines-rerun`.

---

### Summary ###

We recommend both-dynamic-strict-threshold with a 5-sample window: ranking blocks reserved for urgent transactions at all times, and an endorser block announced only when its payload reaches half the RB byte cap or when K = 10 ranking blocks have passed since the last announcement (the age escape, which repairs standard-lane starvation at trickle loads and is bit-identical to the pure threshold whenever traffic crosses the threshold naturally). The full recommended construction:

| Component | Setting |
|---|---|
| Lanes | Two: standard and urgent |
| Ranking blocks | Urgent-only at all loads (ledger-enforced), FIFO selection |
| EB announcement threshold | Payload ≥ max((1 - targetUtilisation) × RB byte cap, RB byte cap / 2); 45,056 B at the default target |
| EB announcement age escape | Announce below threshold once K = 10 ranking blocks have passed since the last EB announcement; inert at ordinary loads, repairs trickle starvation (+83 pp standard retained at 0.1 tx/slot) |
| Fee semantics | Per-lane EIP-1559 coefficient applied to the ordinary min fee |
| Premium scope | rb-only: an urgent transaction included via an EB is refunded down to the standard quote |
| Mempool admission and producer selection (node policy) | For rb-only urgent transactions the fee-cap quote is max(standard quote, urgent quote). Admission requires the posted max fee to cover it one worst-case controller step ahead; producers select only transactions covering one further step, so certified EBs cannot fail fee validation |
| Standard controller | Target utilisation 0.5, max-change denominator 16, capacity-weighted 20-block signal window |
| Urgent controller | Target utilisation 0.5, max-change denominator 16, priority-reservation signal, 5-sample window |
| Floors | Absolute coefficient floor 1.0 (no quote below the ordinary min fee); no cross-lane multiplier floor |
| Validated envelope | Target utilisation 0.5-0.75 and max-change denominator 8-16, from a swept grid of {0.25, 0.5, 0.75} × {4, 8, 16}: target 0.25 and denominator 4 tested and excluded, and 0.75 narrows to flat-fee parity under EB-saturating load |

The canonical machine-readable simulator configuration is [`config/variants/trickle-aging/thr-k10.json`](../../abstract-sim-hs/config/variants/trickle-aging/thr-k10.json). Its embedded load is a replaceable experiment default, not part of the recommended mechanism; the max-of-two fee-cap rule is supplied by the simulator's rb-only fee semantics. The post-correction integrated launch-day check above ran this exact configuration and was scalar-identical to both D16 references in all ten seeds.

The calibration names denominator 16 rather than the denominator-8 anchor used in the comparison tables: the parameter stress test shows the two are welfare-equivalent at every swept load (all paired CIs span zero) while 16 eliminates price shocks at three of four loads. The threshold makes every certificate worth the block space it consumes, which repairs plain reservation's low-load regression (+3.03 ± 1.11 percentage points, ten of ten seeds) and restores statistical parity with the flat-fee baseline in the one regime where reservation used to lose to it; at every contended load the design clearly beats flat fee (+3.04 to +7.38 percentage points). Because the payload rule references only on-chain data, the whole design is ledger-enforceable, and because ranking-block access is never sold below the urgent quote, there is no discount for a producer to trade against. We explored work-conserving variants that admitted standard transactions into underfull RBs at the standard rate; they retained more value at light loads but create an unavoidable bribery incentive, and were rejected.

We prefer the both-dynamic family over priority-only because of the EB-stressing results (37.50% vs 32.92% urgent retained value, with half a block of urgent latency), where it is the standard-lane price that sheds the demand saturating the endorser block, and because of the launch-day results, where reservation over a statically-priced standard lane delivers nothing at all (unpriced standard traffic squats in the shared mempool and starves the reserved lane at admission) while both-dynamic under the same reservation rule beats flat fee by +5.83 ± 4.22 percentage points; at the remaining loads the two families are identical. The priority-only variant remains a quantified fallback if implementation complexity demands it, though the launch-day result bounds what that fallback delivers under sustained saturation. Two further notes for the specification: the parameter stress test fixes the threshold expression at max((1 - targetUtilisation) × |RB|, |RB| / 2) - tracking the controller headroom when headroom is large, never falling below the half-RB floor where certificates stop paying for themselves - so that retuning the controller cannot silently break the certificates-pay-for-themselves property; and the same property gives linear-Leios a clean latency story against Praos, since an EB is only ever produced when it adds more capacity than its certificate consumes.

Answering the question we set out with: the open variants do retain a measurable edge where capacity is slack (-1.63 ± 1.11 percentage points at low load, -1.06 ± 0.83 at mid), and nothing beyond noise elsewhere. That gap is the measured price of ledger enforceability, and we accept it: an open design cannot prevent nodes from circumventing the intended mechanism by accepting bribes for positioning.
