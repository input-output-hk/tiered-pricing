# Discussion of seeded experiment, comparing reserved vs unreserved priority allocation against EIP-1559 and control #

### TLDR ###

Preliminary experiment results show that, across ten seeded runs, the best aggregate rows reduce urgent mean latency from 2.91 to 2.39 blocks (~18%, or 0.52 blocks) and improve urgent retained value from 44.32% to 51.65% (+7.33 percentage points, ~16.5% relative) by providing network participants with a priority lane to which they can opt to submit transactions, for a premium fee. A slight compromise (~2% and 0.1%) on both urgent latency and urgent retention gives us ledger enforceability with the reserved variant, preventing bribery. As a result, we recommend one of the two reserved variants: priority-only-reserved 5-sample window or both-dynamic-reserved 5-sample window.

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

### Mechanisms ###

In this experiment, we compare four designs under active consideration:

|                   | Open (no reservation) | Reserved RB             |
| ----------------- | --------------------- | ----------------------- |
| **Both dynamic**  | both-dynamic-open     | both-dynamic-reserved   |
| **Priority only** | priority-only-open    | priority-only-reserved  |

Note: Each priority-lane config comes with a set of pricing signal variations, which are not enumerated for readability reasons, for example:

```
        "signal": {
          "type": "priority-reservation-window",
          "window": 5
        }
```

This 5-sample window is a way to smooth the signal and decrease oscillation, but it can come with a tradeoff. A window of N uses the previous N priority-signal samples to dampen price changes. This will be discussed later.

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

These four designs are compared against a control, flat fee:

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

Inclusion reports the mean share of submitted demand eventually included. Latency columns report mean latency as actual produced ranking blocks, from first submission to inclusion; priority latency is n/a for the single-lane controls. Shock count and oscillation cycles are mean counts per run; oscillation cycles count completed significant direction-reversal cycles after the convergence-band deadband. Oscillation max amplitude is the largest local coefficient peak-to-trough range.

The uncertainty checks below use paired seed deltas over the same ten seeds. Deltas are left variant minus right variant; positive is better for urgent retained value and tx/slot, while negative is better for urgent latency, shock count, and oscillation cycles. Confidence intervals are 95% t-intervals over the ten paired seed deltas. "Seeds better" counts strict improvements in the preferred direction.

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

<!-- Points to write around:
- Third load profile, `low` = constant 3.0 tx/slot; same variant set, seeds, and caps. Offers ~82% of RB byte capacity and ~1% of EB capacity, so the ranking block is the only active block and the endorser block sits idle. This is the uncongested / below-RB-capacity flank of the load spectrum (see ladder table).
- Because there is no standard-lane congestion, the standard-lane controller never leaves its floor, so both-dynamic degenerates to priority-only here (their rows are identical). The meaningful contrast is reserved vs open.
- Headline reversal: the reserved variants now regress *below* the no-mechanism baselines. flat-fee / single-lane-eip1559 retain ~58.8% of urgent value at 1.85 blk; reserved 5-window retains 56.8% at 1.95 blk; open 5-window retains 61.4% at 1.70 blk.
- Mechanism (the cost of reservation when capacity is abundant): reserving the whole RB for priority idles it. RB byte fill drops from ~91% median under open/flat-fee to ~34% median under reserved, because the RB is priority-only and priority demand is sparse at this load; standard traffic is denied the fast RB and pushed onto the ~20-slot EB cadence, which adds latency and loses urgent value.
- So at low load reservation has a real cost and no offsetting benefit, while open captures the priority-lane upside (61.4% vs 58.8% flat-fee) without paying it. This is the mirror image of the congested loads and is the regime that most exposes reservation's structural downside.
- Columns defined as in the `severe-congestion` table; all rows are means over the same ten seeds.
-->

The three load profiles span the block-capacity hierarchy (byte fill measured from the flat-fee runs; "reserved" shown where it differs materially):

| Load | RB byte fill | EB byte fill | Binding resource |
|---|---:|---:|---|
| `low` (this section) | ~71% open / ~34% reserved | ~1% | neither saturates; EB idle, RB the only active block |
| `severe-congestion` (main results) | ~98% | ~56% | RB |
| `eb-capacity-stress` | ~98% | ~93% | RB + EB |

<details>
<summary>Show load profile</summary>

```
lowLoad :: ArrivalProcess
lowLoad = ConstantLoad 3.0
```

A constant 3.0 tx/slot. The RB holds ~73 transactions and is produced at ~f, so its throughput saturates near ~3.5 tx/slot; 3.0 fills the RB to ~80% (offered ~82% of RB byte capacity, ~1% of EB capacity) without saturating it — non-trivial, but uncongested.

</details>

| Family | Priority signal | Inclusion | Urgent retained | Urgent latency (blk) | Priority latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp | Settled coeff. range |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| flat-fee | n/a | 98.24% | 58.79% | 1.85 | n/a | 2.9 | 0.0 | 0.0 | 0.000 | 0.000 |
| single-lane-eip1559 | n/a | 98.24% | 58.82% | 1.85 | n/a | 2.9 | 0.9 | 0.1 | 0.118 | 0.015 |
| priority-only-reserved | 5-sample window | 97.68% | 56.77% | 1.95 | 2.00 | 2.9 | 8.7 | 5.2 | 0.938 | 0.319 |
| priority-only-open | 5-sample window | 98.26% | 61.38% | 1.70 | 1.76 | 2.9 | 8.0 | 4.7 | 0.955 | 0.129 |
| priority-only-conditional | 5-sample window | 97.85% | 64.07% | 1.56 | 1.64 | 2.9 | 11.5 | 5.6 | 0.967 | 0.414 |
| both-dynamic-reserved | 5-sample window | 97.68% | 56.77% | 1.95 | 2.00 | 2.9 | 8.7 | 5.2 | 0.938 | 0.319 |
| both-dynamic-open | 5-sample window | 98.26% | 61.43% | 1.70 | 1.75 | 2.9 | 8.3 | 4.7 | 0.931 | 0.132 |
| priority-only-reserved | instant (worst) | 97.54% | 56.22% | 1.97 | 2.06 | 2.9 | 37.3 | 15.3 | 0.997 | 0.713 |

---

### EB-stressing load ###

<!-- Points to write around:
- Second load profile, `eb-capacity-stress`; same variant set, seeds, and caps as the `severe-congestion` table above. Only the load profile differs.
- Binding constraint is the EB byte capacity (12 MB), not the RB: across the burst (slots 250–1749, ten seeds) EBs run at/near 100% of the byte cap for the majority of blocks under the static and priority-only variants; EB ex-unit usage stays near half (peak ~67%).
- Oversubscription is moderate, not extreme: new demand exceeds realised inclusion by ~1.1× during the burst.
- Row selection: two single-lane baselines + the 5-sample-window operating point for each family + the single worst windowed row (by urgent retained value).
- 5-sample window is shown for cross-load comparability, NOT as the per-family best — under this load it is no longer the family maximum in every family (e.g. both-dynamic-open instant 40.32% > 5-window 38.87%; both-dynamic-reserved 10-window 38.05% > 5-window 37.57%).
- Columns defined as in the `severe-congestion` table; all rows are means over the same ten seeds.
-->

The `eb-capacity-stress` profile differs from `severe-congestion` only in its arrival schedule (same variant set, seeds, and caps). Rather than one flat burst, it cycles through repeated high-rate peaks separated by troughs, with peak demand (~396 tx/slot) roughly 2.5× the `severe-congestion` burst rate (160), which is what drives demand against the endorser-block byte capacity.

<details>
<summary>Show load profile</summary>

> Note: the `eb-capacity-stress` preset is not present in the committed source (it was a local edit on the machine that produced the sweep). The block below is a behaviourally-equivalent reconstruction: `arrivalRateAt` sums the bursts, and this sum reproduces the per-slot arrival schedule measured from the runs (table below) exactly. The arrival counts constrain the rates only; the `BurstEffect` value/urgency multipliers are assumed `1 1`, as in `severeCongestionLoad`.

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

Arrival schedule measured from the runs (fresh first-attempt submissions per slot; taken from the flat-fee runs, whose fixed low fee means almost nothing is declined at generation, so submissions track the raw arrival rate; averaged over the ten seeds). The offered-demand columns translate the arrival count into the resources it places on the endorser block — bytes from each transaction's body size (`txBody._txSize`, mean ~1,233 B) and ex-units from its script (mean ~614k) — expressed as a fraction of the EB capacity per slot (the 12 MB / 9.5 G caps amortised at the ~0.046 EB/slot production rate):

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
| both-dynamic-reserved | 5-sample window | 95.67% | 37.57% | 3.05 | 3.78 | 163.8 | 59.3 | 7.3 | 4.253 | 4.857 |
| both-dynamic-open | 5-sample window | 95.76% | 38.87% | 3.03 | 4.42 | 164.5 | 59.9 | 8.5 | 4.420 | 4.173 |
| priority-only-reserved | 20-sample window (worst) | 91.78% | 30.41% | 3.81 | 4.61 | 176.5 | 14.9 | 1.9 | 3.613 | 2.662 |

---

### Summary ###

Since the open variants don't outperform (beyond noise) the reserved variants, it's quite clear that the reserved variants would be a better choice, since they allow us to ensure that nodes don't try to circumvent the intended mechanism by offering bribes.
