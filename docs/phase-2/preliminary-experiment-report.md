Discussion of seeded experiment, comparing reserved vs unreserved priority allocation against EIP-1559 and control

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
  "description": "Phase-2 mechanism set: flat-fee control vs the five live dynamic-pricing candidates, severe congestion",
  "seeds": 10,
  "slots": 2000,
  "out": "sweep-results/mechanisms",
  "variants": [
    { "name": "flat-fee", "config": "config/variants/flat-fee.json" },
    { "name": "single-lane-eip1559", "config": "config/variants/single-lane-eip1559.json" },
    { "name": "priority-only-reserved", "config": "config/variants/priority-only-reserved.json" },
    { "name": "priority-only-open", "config": "config/variants/priority-only-open.json" },
    { "name": "both-dynamic-reserved", "config": "config/default-sim-config.json" },
    { "name": "both-dynamic-open", "config": "config/variants/no-reservation.json" }
  ]
}
```

</details>

---

**Metrics.** For each run we record seven families of outcome, some of which are sliced by urgency class and (diagnostically) by lane:

- **Inclusion** - The percentage of transactions (distinct demand units; retries do not add to the count) that were included in any block
- **Value** - The sum of transaction value (in Lovelace) captured, lost and unresolved
- **Latency** - The delay (in blocks) between first submission of transactions and their inclusion in a block
- **Price shock** - Largest single-step relative price move
- **Price stability** - The tendency for the price to remain converged
- **Revenue** - The sum of fees
- **Throughput** - The number of transactions per slot
---

### Mechanisms ###

In this experiment, we compare four designs under active consideration:

|                   | Open (no reservation) | Reserved RB             |
| ----------------- | --------------------- | ----------------------- |
| **Both dynamic**  | both-dynamic-open     | both-dynamic-reserved   |
| **Priority only** | priority-only-open    | priority-only-reserved  |

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

The same as the first design, titled here as: "Unreserved space, two lanes, both dynamic", except the only the priority lane is dynamically priced, while the standard lane is fixed-fee.


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

### Results ###

#### Reading the figures ####

Across ten seeded runs, all mechanisms preserve high overall service rates, but the differences show up in retained value, urgent latency, and the reserved-vs-open tradeoff. Reservation is competitive with open priority-first selection, but it does not dominate it on every metric.

| Variant | Service | Retained value | Urgent retained | Urgent latency | Priority latency | Tx/slot | Price shocks |
|---|---:|---:|---:|---:|---:|---:|---:|
| flat-fee | 99.03% | 92.99% | 43.29% | 59.8 | n/a | 128.7 | 0.0 |
| single-lane-eip1559 | 98.99% | 92.97% | 43.32% | 59.7 | n/a | 128.6 | 3.2 |
| priority-only-reserved | 98.94% | 93.55% | 50.45% | 49.4 | 39.4 | 127.3 | 55.2 |
| priority-only-open | 99.09% | 93.48% | 50.63% | 49.6 | 41.3 | 127.5 | 55.4 |
| both-dynamic-reserved | 98.97% | 93.67% | 51.18% | 46.7 | 42.4 | 121.4 | 61.1 |
| both-dynamic-open | 98.72% | 93.61% | 51.86% | 44.8 | 39.8 | 122.0 | 60.1 |

![both-dynamic-reserved, seed 2](figures/both-dynamic-reserved-seed-2.png)
Here we can see the results of one seed from the both-dynamic-reserved config. Across the top, note the key info cards. Note also the chart elements:

- The "Price coefficient / lane" chart shows, in multiples of the base fee, the price over time of each lane: blue for standard, purple for priority.
- The "RB content over time" element shows when RBs contain transactions vs EB certificates. Orange denotes EB certificate, while green denotes transactions. Darker = denser.
- The "Latency / lane" chart shows the latency of the priority lane vs the standard lane over time.
- Next, we have the "Submission ⇄ inclusion" chart, which shows the lifespans of submitted transactions. Submissions are shown at the top, and inclusions at the bottom. Green lines denote direct RB inclusion, while orange lines denote EB inclusion.
- We also have a simple "Load" chart, which gives an at-a-glance view of the submissions per slot rate.
- The "Latency distribution" shows a box-and-whisker plot of the standard vs priority lane latency in blocks.
- The "Demand fate" element shows how many transactions, by urgency (in blocks per halflife) and lane, were included, abandoned, or unresolved.
- The "Value retained vs lost" chart is similar to the above, except rather than simple inclusion vs exclusion, it shows how much of the sum of value of transactions in each urgency category and lane was retained, lost or unresolved.
