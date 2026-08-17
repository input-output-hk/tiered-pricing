---
CIP: ?
Title: Transaction Urgency Signalling On Linear-Leios
Category: Consensus
Status: Proposed
Authors:
  - Will Gould <will.gould@iohk.io>
  - Polina Vinogradova <polina.vinogradova@iohk.io>
  - Nicolas Henin <nicolas.henin@iohk.io>
  - Giorgos Panagiotakos <giorgos.panagiotakos@iohk.io>
Implementors: []
Discussions: []
Solution-To:
  - CPS-0031
Created: 2026-06-24
License: CC-BY-4.0
---

<details>
<summary><strong>Table of contents</strong></summary>

- [1. Abstract](#abstract)
- [2. Motivation: why is this CIP necessary?](#motivation-why-is-this-cip-necessary)
- [3. Specification](#specification)
  - [3.1 The recommended construction](#the-recommended-construction)
  - [3.2 Controller updates and signals](#controller-updates-and-signals)
    - [3.2.1 Shared update rule](#shared-update-rule)
    - [3.2.2 When block activity enters the signals](#when-block-activity-enters-the-signals)
    - [3.2.3 Urgent controller: 5-sample reservation-utilisation window](#urgent-controller-5-sample-reservation-utilisation-window)
    - [3.2.4 Standard controller: 20-block capacity-weighted window](#standard-controller-20-block-capacity-weighted-window)
  - [3.3 Mempool](#mempool)
    - [3.3.1 Praos and Leios Mempool Specifications](#praos-and-leios-mempool-specifications)
    - [3.3.2 Priority Signaling Mempool Specifications](#priority-signaling-mempool-specifications)
    - [3.3.3 Safe Reordering of Priority Transactions](#safe-reordering-of-priority-transactions)
      - [3.3.3.1 Alternative: a lazy inclusion buffer for non-commuting transactions](#alternative-a-lazy-inclusion-buffer-for-non-commuting-transactions)
    - [3.3.4 Queue structure](#queue-structure)
    - [3.3.5 Revalidation and stale fees](#revalidation-and-stale-fees)
    - [3.3.6 Dependencies and conflicts](#dependencies-and-conflicts)
    - [3.3.7 Capacity, eviction, and DoS](#capacity-eviction-and-dos)
  - [3.4 Ledger](#ledger)
    - [3.4.1 Transaction representation](#transaction-representation)
    - [3.4.2 Ledger Rule Changes](#ledger-rule-changes)
    - [3.4.3 Block validity](#block-validity)
    - [3.4.4 Additional post-transaction-application validation](#additional-post-transaction-application-validation)
    - [3.4.5 New protocol parameter](#new-protocol-parameter)
  - [3.5 Block production and node policy](#block-production-and-node-policy)
  - [3.6 Endorser Block announcement threshold](#endorser-block-announcement-threshold)
    - [3.6.1 Validation evidence](#validation-evidence)
      - [3.6.1.1 Experiment 1: low-load threshold](#experiment-1-low-load-threshold)
      - [3.6.1.2 Experiment 2: announcement age escape](#experiment-2-announcement-age-escape)
      - [3.6.1.3 Experiment 3: parameter stress](#experiment-3-parameter-stress)
      - [3.6.1.4 Experiment 4: default-point byte-threshold sensitivity](#experiment-4-default-point-byte-threshold-sensitivity)
  - [3.7 Incentives](#incentives)
    - [3.7.1 Giorgos](#giorgos)
    - [3.7.2 Nicolas](#nicolas)
- [4. Rationale: how does this CIP achieve its goals?](#rationale-how-does-this-cip-achieve-its-goals)
  - [4.1 How this CIP addresses CPS-0031](#how-this-cip-addresses-cps-0031)
    - [4.1.1 Goal 1: reduce value destroyed by avoidable delay](#goal-1-reduce-value-destroyed-by-avoidable-delay)
    - [4.1.2 Goal 2: permissionless access](#goal-2-permissionless-access)
    - [4.1.3 Goal 3: predictability over raw speed](#goal-3-predictability-over-raw-speed)
    - [4.1.4 Constraints](#constraints)
    - [4.1.5 Open questions](#open-questions)
  - [4.2 Experimental evidence](#experimental-evidence)
    - [4.2.1 D16/K10 headline rerun](#d16k10-headline-rerun)
    - [4.2.2 Thousand-seed replication at low and severe-congestion load](#thousand-seed-replication-at-low-and-severe-congestion-load)
    - [4.2.3 Independent standard-lane target screen and confirmation](#independent-standard-lane-target-screen-and-confirmation)
    - [4.2.4 Window-removal experiment](#window-removal-experiment)
  - [4.3 Prototype](#prototype)
  - [4.4 Why not full tiered pricing?](#why-not-full-tiered-pricing)
  - [4.5 Optional extensions](#optional-extensions)
    - [4.5.1 Tipping](#tipping)
- [5. Path to Active](#path-to-active)
  - [5.1 Acceptance Criteria](#acceptance-criteria)
  - [5.2 Implementation Plan](#implementation-plan)
- [6. Versioning](#versioning)
  - [6.1 Relationship to CIP-183: conflict-based fee priority](#relationship-to-cip-183-conflict-based-fee-priority)
  - [6.2 If the linear-Leios mechanism changes](#if-the-linear-leios-mechanism-changes)
- [7. Copyright](#copyright)

</details>

## Abstract

We propose two lanes by which a user can submit a transaction to a node: urgent and standard. Only urgent transactions can enter Ranking Blocks. Both urgent and standard transactions can enter Endorser Blocks. Nodes produce Ranking Blocks more frequently than Endorser Blocks, and a Ranking Block enters the chain immediately. When capacity and queue order permit, an urgent transaction can therefore enter an earlier Ranking Block instead of the later Endorser Block path. This creates an earlier inclusion opportunity, not a guarantee of earlier inclusion.

The ledger enforces the urgency signalling rule: every transaction in a valid Ranking Block must carry a fee that covers the urgent quote for that block. In simulation under severe congestion, the mechanism preserves more urgent-class transaction value than linear-Leios with today's flat fee. Retained value means the modelled gross transaction value that remains at inclusion, before fees. Urgent-class retained value improved across most simulated loads. At light load, the mechanism slightly reduces overall retained value, because transactions on the standard path wait longer while Endorser Blocks fill. The Rationale gives exact figures.

## Motivation: why is this CIP necessary?

Some transactions lose value when delayed, but users currently have no protocol-level way to signal that urgency.

Linear-Leios introduces a new block type: the Endorser Block. Vanilla linear-Leios uses this additional path only when traffic exceeds Ranking Block capacity. This proposal instead routes standard transactions through Endorser Blocks at every load. Endorser Blocks are slightly slower than Ranking Blocks, so latency variability increases. An urgency signal offsets this cost: it lets nodes allocate block space to serve users' intents.

From CPS-0031:

> During periods of congestion, high-urgency transactions lose value when they cannot obtain timely inclusion. A protocol-recognised urgency signal could help preserve more transaction value during congestion, especially for transactions whose value is highly delay-sensitive.

> Candidate solutions should be evaluated by how they handle prioritising high-urgency transactions, and by how they affect ordinary and low-urgency users during sustained congestion.

<!-- PORTABILITY: once CPS-0031 merges, repoint this at the repo-relative ../CPS-0031 and confirm the assigned number -->

See [CPS-0031](https://github.com/cardano-foundation/CIPs/pull/1194) for more information.

We initially planned a mechanism based on full tiered pricing, then set it aside in favour of the two-lane design specified here. The Rationale section [Why not full tiered pricing?](#why-not-full-tiered-pricing) gives the comparison.

## Specification

The mechanism at a glance, with a legend below. Fee settlement and the EB announcement gate are drawn in their own figures in the sections that specify them.

```mermaid
flowchart LR
    TX["Transaction:<br/>lane choice + max fee<br/>+ refund account"] --> ADM{"Admission:<br/>max fee covers the quote<br/>one controller step ahead?"}
    ADM -- "no: BidBelowQuote" --> REJ["Rejected"]
    ADM -- "yes" --> Q
    subgraph MP["Mempool"]
        Q["Canonical FIFO queue"]
        UV["Urgent view:<br/>indices into the queue"]
    end
    Q -- "evicted if the quote rises<br/>above max fee;<br/>wallet resubmits" --> EV["Evicted"]
    UV --> RULE["Ledger rule:<br/>every RB transaction<br/>pays the urgent quote"]
    RULE --> RB["Ranking Block"]
    Q -- "FIFO, gated by the<br/>announcement rule<br/>(threshold section)" --> EB["Endorser Block"]
    RB --> CTRL["Per-lane EIP-1559 controller:<br/>lane utilisation vs target;<br/>each lane's quote<br/>updated independently"]
    EB --> CTRL
    CTRL -.-> ADM
    classDef enforced stroke-width:3px,font-weight:bold;
    classDef policy stroke-dasharray:5 5;
    class RULE,CTRL enforced;
    class ADM,REJ,MP,Q,UV,EV policy;
```

```mermaid
%%{init: {"themeVariables": {"fontSize": "11px"}, "flowchart": {"nodeSpacing": 15, "rankSpacing": 15, "padding": 4}}}%%
flowchart LR
    subgraph LEG["Legend"]
        direction LR
        LG2["Ledger-enforced rule"] ~~~ LG3["Node policy"] ~~~ LG4{"Decision"}
        LA1(("·")) -- "transaction path" --> LA2(("·"))
        LB1(("·")) -. "quote feedback" .-> LB2(("·"))
    end
    classDef enforced stroke-width:3px,font-weight:bold;
    classDef policy stroke-dasharray:5 5;
    class LG2 enforced;
    class LG3 policy;
```

This CIP introduces a transaction-level urgency signal with two lanes: standard and urgent. Both lanes are dynamically priced, each with its own fee quote controlled by its own EIP-1559 controller. Urgent transactions are eligible for inclusion in both Ranking Blocks and Endorser Blocks. Standard transactions are eligible only for Endorser Blocks. The ledger enforces that Ranking Blocks contain only urgent-paying transactions.

We specify that Ranking Blocks can contain only transactions whose on-ledger fee authorisation covers the urgent quote. A block that breaks the rule is invalid, so a producer cannot substitute a transaction admitted below that quote. The premium goes to the treasury, not the producer. An urgent-paying transaction therefore pays a producer no more than any other transaction of the same size. This rule does not by itself prevent off-chain side payments or other producer manipulation. Those remain part of the Incentives analysis. The rule does remove a key bribery incentive: a producer cannot include a standard-paying transaction in an RB at any price.

We also specify a modification that corrects a problem at moderate load, when RB fill sits between the fill target (0.5 by default) and the RB maximum. Under the modification, a certificate for a non-empty EB can enter the chain in only two cases: the EB's payload reaches the threshold described below, or, as an age-gated escape, at least K Ranking Blocks have been produced since an EB certificate last entered the chain. The modification defends against the following case. At this load, a steady flow mixes some standard transactions with urgent transactions. Without the modification, a producer announces an EB at every opportunity, so RBs frequently carry EB certificates. The outcome is self-sabotaging. Standard transactions wait longer, because a non-full RB cannot carry them. Urgent transactions also wait longer, because certificate-carrying RBs exclude them.

<details>
<summary>Show glossary of terms</summary>

**Transactions and reservation**

- **Standard transaction:** A transaction that does not pay to enter the urgent lane. Cardano's current transactions.
- **Urgent transaction:** A transaction that pays to enter the urgent lane. The payment asks nodes to include the transaction before standard transactions, where possible.
- **Reserved:** An urgent-lane mechanism that reserves RB block space for urgent transactions. The ledger enforces the reservation.

**Lanes and routing**

- **Standard lane:** A pathway for transactions that do not pay the urgent fee.
- **Urgent lane:** A pathway for transactions that pay the urgent fee.
- **Lane selection (the user-side decision):** The choice of lane, made by the constructor of a transaction.

**Pricing primitives**

- **Pricing coefficient:** The value that multiplies the base fee to produce the quote. Also called *tier coefficient*.
- **Quote:** The pricing coefficient multiplied by the base fee. In effect, a snapshot of the dynamic fee for a given transaction.
- **Urgent premium:** The difference between the urgent lane quote and the standard lane quote.
- **Absolute coefficient floor:** The minimum allowed lane pricing coefficient, set to `1.0`: no quote can fall below the ordinary Cardano minimum fee.
- **Fixed (pricing):** Basic Cardano fee, as today.
- **Dynamic (pricing):** EIP-1559 style dynamic fee.
- **EIP-1559 (controller):** The feedback mechanism that adjusts a lane's pricing coefficient after each block: up when utilisation is above target, down when below, by a bounded step.
- **Max-change denominator (D):** The scale in the controller update. Before the coefficient floor, the largest downward step is `1/D`. The largest upward step is `(1 - targetUtilisation) / (targetUtilisation × D)`. At target 0.5 these are equal, so the price rises and falls at the same maximum rate. Below 0.5 the price can rise faster than it falls. Above 0.5 it rises more slowly.
- **Signal window:** The number of recent blocks over which the controller measures utilisation, so a single unusual block cannot swing the price.
- **Target utilisation:** The block fill level the controller steers towards (0.5 for the urgent controller and 0.75 for the standard controller in the default configuration). Utilisation above the target raises the price. Utilisation below it lowers the price.
- **Quote drift:** The difference between the quote at submission time and the quote at inclusion time.

**User-side fee fields**

- **Posted fee vs actual fee:** The posted fee is the amount attached to the transaction at submission. The actual fee is the quote at inclusion time. The ledger refunds the difference.
- **Refund:** The return of the excess fee to a specified address.
- **Max fee (`max_fee_lovelace` / fee ceiling on the user side):** The most a user agrees to pay, posted with the transaction. It buffers against quote drift. If the quote exceeds it, the transaction cannot be included.

**Value / actors**

- **Urgency:** The rate at which the value of a transaction decays.
- **Urgent demand class:** The fastest-decaying demand class in the model. It is independent of the lane a transaction selects.
- **Retained value metric:** The numerator is the sum of modelled delay-discounted gross transaction value remaining at inclusion, before fees. It is not the simulator's fee-subtracted utility measure, and it is not an observed economic quantity. Unless a table states a different denominator, a retained-value ratio is `retained / (retained + lost)`. The ratio excludes value still unresolved at the simulation horizon.
</details>

<br>

### The recommended construction

The settled recommendation in one place. Each component is specified in detail in the sections that follow, except the controller update rule and signals, which are defined immediately below the table.

| Component | Specification |
|---|---|
| Lanes | Two: standard and urgent |
| Ranking Blocks | Urgent-only at all loads (ledger-enforced). FIFO selection over the urgent view |
| Endorser Blocks | Open to both lanes. FIFO selection over the canonical queue |
| EB announcement threshold | Let `thresholdFraction = max(1 - urgentTargetUtilisation, 1/2)`. Unless the age escape applies, a non-empty certified EB qualifies when any one of its serialised transaction size, its reference-script size, or either component of its execution-unit budget reaches that fraction of the corresponding RB limit. The default fraction is 1/2 (45,056 B for the simulated transaction-size component) |
| EB announcement age escape | A certificate for a non-empty EB below the threshold can enter an RB once at least K = 10 Ranking Blocks have been produced since the last certified EB |
| Fee semantics | Per-lane EIP-1559: each lane's quote is its pricing coefficient × the ordinary min fee |
| Fee-cap basis | For an urgent transaction under rb-only settlement, wallet choice and every max-fee validity check use max(standard quote, urgent quote). Temporary quote crossings are permitted and do not alter either controller |
| Premium scope | rb-only: the applicable inclusion quote is the urgent quote in an RB and the standard quote in an EB |
| Admission, revalidation, and selection (node policy) | Admission requires the posted max fee to cover the maximum applicable lane quote after one conservative lane-specific controller step. While the transaction is queued, the max fee must cover the current fee-cap quote, or the node evicts the transaction. Prudent EB selection takes a transaction only if the max fee also covers one further lane-specific step (RB selection needs only the current quote, since inclusion is immediate) |
| Settlement and refund | Inclusion charges the applicable inclusion quote. The ordinary min-fee component goes to the fee pot, the premium above it goes to the treasury, and the posted excess goes back to the refund account. A posted maximum below the applicable quote is invalid |
| Standard controller | Target utilisation 0.75, max-change denominator 16, capacity-weighted utilisation over a 20-block window, initial coefficient 1.0 |
| Urgent controller | Target utilisation 0.5, max-change denominator 16, reservation utilisation over a 5-sample window, initial coefficient 2.0 |
| Floors | Absolute coefficient floor 1.0 (no quote below the ordinary min fee). No cross-lane multiplier floor |
| Enforcement boundary | Ledger rules enforce RB lane eligibility, inclusion-point fee validity, settlement, and the deterministic per-lane quote update. The EB threshold and the age escape are ledger rules checked at certificate inclusion, specified in the Endorser Block announcement threshold section. Wallet choice, the urgent queue view, FIFO construction, admission headroom, revalidation, eviction, and producer headroom are node policy |

The canonical simulator configuration for the experiments is [`thr-k10.json`](./thr-k10.json), a copy of `abstract-sim-hs/config/variants/trickle-aging/thr-k10.json` from the tiered-pricing repository. Its embedded load is only the simulator's default workload, and experiment manifests override it. It is not part of the mechanism recommendation. The simulator implements only the byte component of the multidimensional EB threshold specified above. The max-of-two fee-cap rule is the simulator's rb-only fee semantics rather than a configurable alternative.

The parameter values, the grid points tested, and the loads at which each was stressed are tabulated in the "Endorser Block announcement threshold" section.

### Controller updates and signals

The design uses two independent pricing controllers, one for each lane:

- the **urgent controller** sets the urgent-lane pricing coefficient;
- the **standard controller** sets the standard-lane pricing coefficient.

A pricing controller is a feedback mechanism. It observes recent utilisation of its lane and adjusts that lane's pricing coefficient. Utilisation above the target raises the coefficient; utilisation below the target reduces it. The lane's quote is obtained by applying its current coefficient to the ordinary minimum fee.

The two controllers use the same update rule and the same recommended `D` value. They differ in their recommended target utilisation, their initial coefficient and, more importantly, in how they calculate utilisation:

| Controller | Target utilisation | Initial `coeff` | Utilisation signal | Window |
|---|---:|---:|---|---:|
| Urgent | `0.5` | `2.0` | Urgent usage measured against RB reservation capacity | **5 payload samples** |
| Standard | `0.75` | `1.0` | Standard usage measured against the combined capacity of recent blocks | **20 block summaries** |

The urgent window of **5 samples** is an experimentally selected compromise between responsiveness and stability. Windows of `3` and `5` samples were tested, while windows of `10–20` reduced the number of price shocks but generally increased latency and peak-to-trough price movement and reduced urgent retained value.

The standard window of **20 block summaries** is the recommended default, but its length has not been independently swept.

An experiment removed each window in turn, on fresh seeds, at the recommended calibration ([Window-removal experiment](#window-removal-experiment)). The result confirms that both windows are necessary, for different reasons. Without the standard window, launch-day throughput and retained value decrease. Without the urgent window, retained value shows no measurable loss, but the quotes move much more at every load.

The recommended `D = 16` was tested for both controllers in lockstep. The standard-lane target of `0.75` was selected and confirmed by a dedicated experiment that swept it independently while the urgent target stayed at `0.5`; see [Independent standard-lane target screen and confirmation](#independent-standard-lane-target-screen-and-confirmation).

#### Shared update rule

Both controllers are evaluated independently once per slot in which a block is produced. Each applies the same update rule to its own utilisation signal:

$$ \mathrm{coeff}' = \max\left( 1.0,\; \mathrm{coeff} \times \max\left( 0,\; 1+ \frac{\mathrm{utilisation}-\mathrm{target}} {\mathrm{target}\times D} \right) \right) $$

Before applying the rule, the controller clamps `utilisation` to `[0, 1]`. The outer `max` prevents the coefficient from falling below `1.0`, so a lane's quote cannot fall below the ordinary minimum fee.

The recommended shared parameters are:

| Parameter | Recommended value | Effect |
|---|---:|---|
| `target` | `0.5` urgent, `0.75` standard | Utilisation above the lane's target raises the coefficient; utilisation below it lowers the coefficient |
| `D` | `16` | Bounds the size of each coefficient update |
| Absolute coefficient floor | `1.0` | Prevents a quote from falling below the ordinary minimum fee |

With `D = 16`, a single update changes a coefficient by no more than `±6.25%`. At the urgent target of `0.5` the largest step is `6.25%` in both directions. At the standard target of `0.75` the largest upward step is `2.08%`, while the largest downward step remains `6.25%`.

#### When block activity enters the signals

A block-production event contributes utilisation information only when transaction payload is applied:

| Block-production event | Transaction payload applied? | Contributes utilisation? |
|---|---|---|
| Non-certificate Ranking Block | Yes — RB payload | Yes |
| Certificate-carrying Ranking Block | No — payload-free | No |
| Endorser Block announcement | No — announcement only | No |
| Certified Endorser Block | Yes — EB payload | Yes |

Each transaction payload therefore enters the controller signals exactly once:

- an RB payload when its non-certificate Ranking Block is produced;
- an EB payload when its Endorser Block is certified.

Both controllers observe the same payload-application events, but they calculate different utilisation values from them.

#### Urgent controller: 5-sample reservation-utilisation window

The urgent controller measures urgent demand against the scarce capacity available for urgent service. Its window contains the **five most recent payload samples**, not necessarily the five most recent slots or block-production events.

For each sample:

1. Measure urgent-lane usage in bytes and execution units.
2. Compare that usage with the urgent reservation capacity of a Ranking Block.
3. Cap the measured usage at that reservation capacity.
4. Compute utilisation separately for bytes and execution units.
5. Use the larger of the two results.

A certified EB uses the same RB reservation capacity as its denominator, rather than the EB's much larger capacity. The urgent signal therefore asks how many Ranking Blocks' worth of urgent traffic the payload carried; it does not measure how full the Endorser Block was.

Across the five-sample window:

$$ \mathrm{urgentUtilisation} = \max\left( \frac{\sum \mathrm{urgentBytes}}      {\sum \mathrm{RBReservationBytes}}, \; \frac{\sum \mathrm{urgentExUnits}}      {\sum \mathrm{RBReservationExUnits}} \right) $$

Each sample's urgent usage is capped at the corresponding RB reservation capacity before being added to the numerator.

#### Standard controller: 20-block capacity-weighted window

The standard controller measures standard-lane usage against the combined capacity represented by the **twenty most recent block summaries**.

Across the window:

1. Sum standard-lane usage.
2. Sum the corresponding block capacities.
3. Divide total usage by total capacity.
4. Compute the ratio separately for bytes and execution units.
5. Use the larger ratio.

The resulting signal is:

$$ \mathrm{standardUtilisation} = \max\left( \frac{\sum \mathrm{standardBytes}}      {\sum \mathrm{blockCapacityBytes}}, \; \frac{\sum \mathrm{standardExUnits}}      {\sum \mathrm{blockCapacityExUnits}} \right) $$

The block-production events contribute as follows:

- certificate-carrying RBs and EB announcements contribute neither usage nor capacity;
- non-certificate RBs contribute their full capacity to the denominator, even when empty and even though standard transactions cannot occupy them;
- certified EBs contribute their EB capacity and the standard traffic they carry.

This is a capacity-weighted signal: each block affects the result in proportion to its capacity. At the capacities used in the experiments, a certified EB provides `12,000,000` bytes of capacity, compared with `90,112` bytes for a Ranking Block. A certified EB therefore carries approximately 133 times the byte weight of an RB, so the standard quote responds primarily to Endorser Block utilisation.


### Mempool

Our priority signaling design includes changes to the consensus protocol, the Leios protocol specifically. 
For this reason, we [specified](https://github.com/IntersectMBO/ouroboros-consensus/compare/polina/mempool-spec?expand=1) three mempool variants in Agda:
- the **Praos mempool** (`Mempool.lagda.md`);
- the **baseline Leios mempool**;
- the **Leios mempool with priority signaling**.

In all three specifications, the mempool imposes the same constraints on the block as enforced at the ledger level 
by the corresponding consensus protocol,
e.g. block size limits, the constraint that an RB has either transactions or an EB certificate and never both, etc. 
The Praos specification is based on the current mempool design directly. 

#### Praos and Leios Mempool Specifications 

The Leios mempool (`MempoolLeios.lagda.md`) design features two distinct types of blocks (RB and EB), and 
an extra stored ledger state `ebLedger`. This ledger state 
variable is `Nothing` whenever *no EB* has been sent across the network for inclusion in a future RB, and is set to 
the ledger state updated with `heldEB` when that EB arrives across the network. All transactions in the mempool 
are applied to/validated against `ebLedger` (if non-`Nothing`), and the resulting updated ledger state is stored 
in `updatedLedger`.

For *non-epoch boundary blocks*, the ledger state can be updated in O(1) in some cases. Specifically, when an RB 
arrives containing a certificate for the `heldEB` block, the `ebLedger` becomes the new ledger state at the tip
with only some minor additional block-level bookkeeping. For epoch boundary blocks, a full ledger state recomputation 
for the incoming RB/EB must be performed. The function generating blocks from mempool content, `forgeBlock`, 
returns a pair `(RB, Maybe EB)`. The `RB` is sent across the network to be added to nodes' chain tips, whereas 
`EB` is sent across the network to be added to the nodes' mempools `heldEB` variable. 

#### Priority Signaling Mempool Specifications

The priority signaling mempool design, specified in `MempoolLeiosPricing.lagda.md`,
features two distinct ledger states in place of `updatedLedger`: 

- **`priorityUpdatedLedger`**: the result of applying every transaction in `priorityTxs` to:
  - `ebLedger`, when a valid EB has arrived; or
  - `ledger`, otherwise.

  The `priorityTxs` queue contains transactions that specify the priority tier.
- **`standardUpdatedLedger`**: the result of applying every transaction in `standardTxs` on top of `priorityUpdatedLedger`.

  The `standardTxs` queue contains transactions that specify the standard tier.

The mempool is able to request transactions of a specific tier from its peers. 
It first requests the priority tier transactions, 
and only when none are available, requests standard transactions. 

#### Safe Reordering of Priority Transactions 

The mempool designed to support urgency signaling in Leios requires priority transactions to be placed ahead 
of standard ones, out of FIFO queue order. 
That is, an incoming priority transaction enters the queue after all `priorityTxs` and in front of `standardTxs`.
The mempool update cost for each priority transaction is proportional to the number of standard transactions 
that need to be revalidated against the new `priorityUpdatedLedger`.

To address this inefficiency, we have 
[proved](https://github.com/IntersectMBO/formal-ledger-specifications/compare/polina/txcomm?expand=1) 
a theorem stating that two lists containing the same transactions 
produce the same *updated* ledger state 
when applied to the same state and in the same environment.
It has the following (also proved) corollary:

::: {#cor-simple}
Let `standardTxs` and `priorityTxs` be lists of transactions, `tx : Tx`, 
`s : LState`, and `e : LEnv`.
Suppose the conditions `Cstd(standardTxs)`
and `Cpri(tx::priorityTxs)` are satisfied. Suppose also that `standardTxs ++ priorityTxs`,
`(tx :: standardTxs) ++ priorityTxs`, and `tx :: priorityTxs` are all valid in `e, s`.
Then, applying `(tx :: standardTxs) ++ priorityTxs` or `standardTxs ++ (tx :: priorityTxs)`
to `s` in `e` gives the same updated ledger state.
:::

We do not specify `Cstd` and `Cpri` here exactly (both are encoded in the commutativity proof).
Instead, we list the resulting types of conflict an incoming priority transaction may have with transactions 
in `standardTxs` that we must check for:

- the priority transaction **consumes an input that some standard transaction reads without consuming**: a reference
  input, collateral it does not also spend, or (for a phase-2-invalid standard transaction) an unconsumed input;
- overlapping **certificate targets** or **deposit keys**;
- overlapping **withdrawal credentials**, or a withdrawal credential matching the other transaction's certificate target;
- overlapping **governance vote targets** (same action and voter);
- **governance proposals and DRep (de)registration**, which conflict *globally* (they re-filter all pending votes), so no
  pairwise check can license commuting past them. This category is restricted to a single tier
  rather than being conflict-checked, and that tier must be the priority one: when Leios falls back
  to Praos-mode operation, only the priority queue feeds Ranking Blocks, and no standard transaction
  gets in — placing governance proposals in the standard tier would mean censoring governance for
  the whole duration of the fallback. **Decision required**: whether governance-proposing
  transactions, despite living in the priority queue, are allowed to pay the standard quote rather
  than the urgent one (so that governance participation is not priced as urgency).

To address these conflicts and maintain a better than linear (in the size of the `standardTxs` queue) time 
for including incoming priority transactions, we adopt the following strategy, proved correct in the
commutativity branch linked above. An incoming priority transaction is validated twice: once at the back of
the standard queue, and once at its insertion point. It is then checked for the conflicts listed above against
the `standardTxs` queue. If all checks pass, no standard transaction needs revalidation.
On any conflict, the priority transaction is simply discarded. 

The lookups use one counted multiset (a map from key to occurrence count) per conflict source, maintained
alongside the standard queue: non-consumption reads keyed by `TxIn`, certificate targets and withdrawal
credentials keyed by `Credential`, deposit keys keyed by `DepositPurpose`, and vote targets keyed by
`(GovActionId, Voter)`. Disjointness from these unions indicates a lack of conflict, so admission
costs `O(|footprint of the incoming transaction|)` lookups. Counts are incremented at standard admission,
decremented when a single transaction leaves, and rebuilt for free during events that already reapply the
whole queue. There may be some over-approximation in this strategy; however, users of the same Voter 
ID or credential are likely the same user, and therefore it is not necessary to provide 
a scheme for users to undercut their own actions. Governance proposals form a kind of linked list in the 
order of appearance. Reordering this list by allowing priority transactions to get ahead of standard 
ones is not a necessary feature to support. 

##### Alternative: a lazy inclusion buffer for non-commuting transactions

An alternative to discarding a conflicting priority transaction is to hold it rather than drop it.
Besides `priorityTxs` and `standardTxs`, the mempool keeps a third list, `lazyTxs`, reserved for
incoming priority transactions that fail the conflict check above. Instead of being applied against
`priorityUpdatedLedger` or the standard queue's ledger state, a `lazyTxs` entry is applied against the
ledger state of the currently selected chain (i.e. the state before any mempool transaction is applied),
and `lazyTxs` behaves as an ordinary FIFO queue on top of that state.

`lazyTxs` is flushed on every new chain selection: its contents are appended to the end of `priorityTxs`,
and the whole queue is then revalidated exactly as it already is whenever a new block arrives (see
[Capacity, eviction, and DoS](#capacity-eviction-and-dos)). Basing `lazyTxs` on the chain-selected state
rather than on `priorityUpdatedLedger` or the standard queue's ledger state is what makes this flush cheap
to reason about: once a new chain is selected, none of the existing guarantees about which transactions
from `priorityTxs` or `standardTxs` survive revalidation hold anyway, so merging `lazyTxs` in at that
point adds no new uncertainty. Diffusion of `lazyTxs` contents is unchanged: they propagate to peers
exactly as any other mempool transaction does.

**Evaluation against the discard mechanism above.**

- *Loss of transactions.* The discard mechanism above drops a conflicting priority transaction outright;
  the submitter must notice and resubmit. The lazy buffer instead holds it, so a transaction that only
  conflicted transiently (the standard transaction it clashed with is later evicted, or a new chain
  changes the picture entirely) gets a chance at inclusion without user action.
- *Latency.* A transaction sitting in `lazyTxs` cannot be selected into an RB or EB until the next
  chain-selection event flushes it into `priorityTxs`. The discard mechanism resolves immediately
  (accept or reject at submission); the lazy buffer trades that immediacy for a wait of up to one block.
- *State and complexity.* The lazy buffer is a third piece of mempool state, with its own admission
  logic (Phase-1 validation against the chain-tip ledger state) and its own capacity/eviction policy,
  which is not yet specified here and would need the same treatment
  [Capacity, eviction, and DoS](#capacity-eviction-and-dos) gives `priorityTxs` and `standardTxs` —
  otherwise it is a new, uncapped surface for a submitter to fill with non-commuting transactions.
- *Cost at the point it matters.* Because `lazyTxs` is only merged in at a chain-selection boundary, and
  the entire queue is revalidated at that boundary regardless, the merge itself adds no revalidation pass
  beyond what [Capacity, eviction, and DoS](#capacity-eviction-and-dos) already performs. The counted-multiset
  conflict check above runs once, at admission to `lazyTxs`; nothing pays for it again while the transaction waits.
- *Intra-buffer conflicts.* The design does not yet say what happens when two `lazyTxs` entries conflict
  with each other rather than with `standardTxs`. Treating `lazyTxs` as an ordinary FIFO queue over the
  chain-tip state (as `standardTxs` is treated today) resolves this the same way order-of-arrival resolves
  any other queue conflict, but this should be stated explicitly if the buffer is adopted.

**Behaviour change**.
Both mechanisms are compatible with the commutativity result above. The main deciding factor should be
the change in behaviour in terms of what transactions survive in the `standardTxs` queue once an RB 
is released containing the `lazyTxs`. Because flushed `lazyTxs` entries are appended ahead of
`standardTxs` before the whole queue is revalidated, any standard transaction they still conflict
with will now fail validation and be evicted — the lazy buffer gives priority transactions a way to
evict standard ones on purpose. That is precisely what admission is designed to prevent (see
[Dependencies and conflicts](#dependencies-and-conflicts): a conflicting priority transaction is
discarded rather than admitted because admitting it would require evicting a standard transaction).
If adopted, the flush should therefore re-apply the same rule — dropping or re-buffering any entry
that would still evict a standard transaction — rather than letting it through unconditionally.

The evictions enabled by an unconditional flush are not limited to a user displacing their own
transactions: most of the conflict classes above can arise between transactions with fully disjoint
signature sets. Reference inputs are unwitnessed reads, so a priority transaction consuming a shared
UTxO (an oracle updating its datum, a maintainer rotating a reference script) would evict every
standard transaction of strangers who merely read it; two unrelated users contending for the same
script UTxO (a DEX pool, a batcher) would resolve the race in favour of whoever paid the urgent
quote; and a registration certificate requires no witness at all, so a priority transaction could
evict a targeted standard transaction without any authorization from its victim. An unconditional
flush would therefore amount to a purchasable eviction mechanism over contested state — exactly the
kind of priority auction this CIP does not intend to create.


**TODO** : decide whether to adopt the lazy inclusion buffer in place of, or alongside, outright discard,
and if adopted, specify its capacity and eviction policy. 

Note that while the queue structure in the 
specification is made up of two lists, it can also be expressed via a view (as discussed next).

#### Queue structure

RB construction must identify the urgent transactions in the mempool without a scan of the whole queue. The queue structure therefore remains as it is today, but with an additional component: a view of urgent transaction indices (the indices point at the main queue). RB construction consults this view.

EB construction operates the same way block construction operates on Cardano today: the node consults the canonical queue in FIFO order.

Mempool structure remains node policy, so the ledger does not enforce it.

#### Revalidation and stale fees

A dynamic quote can rise after admission. A posted max fee that covered the quote at submission can then fall short when the transaction is selected. We handle this with three layers of node policy, ordered by when each acts.

The two controllers are independent, so the standard quote can temporarily rise above the urgent quote. This is a permitted controller state, not a reason to impose a cross-lane multiplier floor. Because an urgent transaction can settle through either path, its fee-cap quote is `max(standard quote, urgent quote)` throughout wallet lane choice, admission, revalidation, and producer selection. Its actual fee remains inclusion-point-specific: the urgent quote in an RB and the standard quote in an EB.

A possible alternative is a 1× cross-lane clamp, which enforces `urgent quote ≥ standard quote`: it raises the urgent quote whenever the lanes invert. We do not adopt it because it couples the controllers and can raise the RB price when urgent-lane utilisation does not justify it. Max-of-two instead changes only the fee cap needed to cover both settlement paths. It does not change either controller or the inclusion-point-specific charge.

At admission, the posted max fee must cover the applicable lane quotes one worst-case controller step ahead: both lanes for an urgent transaction, since it can settle at either quote, and the standard lane alone otherwise. One step is the right horizon because an EB producer requires the same at selection, so nothing enters the mempool that a producer then refuses. At the recommended target 0.5 and D = 16 on both lanes, that is around 6.25% of headroom. The urgent lane requires headroom because of eviction. An urgent transaction that offers exactly the urgent fee and no more can be priced out while it waits during a price increase. The node must then evict it, and the transaction wasted mempool space for its whole stay.

If lane $l$ has a controller:

$$ \mathrm{stepBound}_{l} = \max\left( \frac{1}{D}, \frac{1-\mathrm{targetUtilisation}_{l}}{\mathrm{targetUtilisation}_{l} \times D} \right) $$

Otherwise:

$$ \mathrm{stepBound}_{l} = 0 $$

For a standard transaction:

$$ \mathrm{maxFee} \ge \mathrm{quote}_{\mathrm{standard}} \times \left(1+\mathrm{stepBound}_{\mathrm{standard}}\right) $$

For an urgent transaction:

$$ \mathrm{maxFee} \ge \max\left( \mathrm{quote}_{\mathrm{standard}} \times \left(1+\mathrm{stepBound}_{\mathrm{standard}}\right), \mathrm{quote}_{\mathrm{urgent}} \times \left(1+\mathrm{stepBound}_{\mathrm{urgent}}\right) \right) $$

The node rejects a transaction that cannot survive even one price update at the door. The rejection is visible, and the user can cheaply resubmit with a larger buffer. The alternative is worse: an admitted transaction sits against the mempool cap until it goes stale.

At selection into an EB, a producer takes only transactions that remain valid through the one further price update that can fire before the certification check. This guarantees that a certified EB cannot fail fee validation. The producer re-checks against current prices because prices can rise while a transaction queues. This extra step applies only to EBs. RB inclusion is immediate: no price update can fire between selection and inclusion, so RB selection checks the current quote alone.

The node evicts an admitted transaction whose max fee is overtaken anyway. Eviction must be the outcome here. The transaction must not enter an invalid block, and a transaction that cannot be included wastes mempool space.

The ledger enforces none of this, since mempool state is not observable on-chain.

#### Dependencies and conflicts

A priority transaction may be in conflict with transactions in the standard queue.
Conflict is detected whenever a transaction `tx` is Phase-1 validated both at the end of the priority 
queue and the end of the standard queue, and one of those validations fails. Then, `tx` will not be 
admitted to the mempool because doing so requires evicting one or more standard transactions from the 
standard queue, which is outside the scope of the kind of priority signaling this CIP is designed to enable. 
A common cause of such conflict is that `tx` is spending the same UTxO entry as some transaction in the 
standard queue. Additional conflicts requiring rejection of an incoming priority transaction may arise,
see [Safe Reordering of Priority Transactions](#safe-reordering-of-priority-transactions).

#### Capacity, eviction, and DoS

Priority transactions get at least an RB's worth of space and ExUnits allocated to them in the 
mempool, and may be admitted to an EB when that space is full. The eviction process for 
transactions that become Phase-1 invalid remains the same as in prior eras. That is, 
when a new block arrives, the entire mempool is revalidated (kicking out stale transactions, 
or ones whose inputs were spent, etc.). If the newly arrived block is an RB containing 
an EB certificate which (1) matches the one in the `heldEB` variable in the mempool, and (2) the RB 
is not an epoch boundary block, revalidation of the entire queue is not required. 
Otherwise, it is required.

### Ledger

Only transactions that pay a sufficient fee for the urgent lane can enter Ranking Blocks. To enforce this rule, we must make [ledger changes](https://github.com/IntersectMBO/formal-ledger-specifications/compare/polina/dynamic).

#### Transaction representation

The CDDL changes are as follows :

```
tier_no    = 0 / 1          ; 0 = priority, 1 = standard
tier_coeff = uint           ; price multiplier γ, ≥ 1 
tx_tier    = [tier_no, tier_coeff]

transaction_body = { ...
  , 23 : tx_tier          ; claimed tier
  , ? 24 : reward_account   ; fee change address
  }
```

That is, a transaction body must specify the `tier_no` which indicates whether it's a priority or standard transaction, 
and a positive integer `tier_coeff`. This tier coefficient 
is what the transaction expects its `minfee` will be multiplied by to obtain the amount 
of fee it has to pay to get into its specified tier. 

The `reward_account` specifies the address of the account to which change is returned when a transaction 
specifies a `txfee` that is larger than necessary. 


#### Ledger Rule Changes

We define an `SDPolicy` record containing five variables that are used in the following way :

  1. `diversityPolicy : TierNo ⇀ PolicyClause` - a map assigning to each tier a policy clause, which specifies, 
  in particular, the tier coefficient
  1. `coeffWindow : List (TierNo ⇀ ℕ)` - the most recent tier coefficients, one entry per processed block, 
  of which at most `windowSize` (a new protocol parameter, see below) are stored; this is the data over which 
  the moving average used by the coefficient update is computed
  1. `totalSize : TierNo ⇀ ℕ` - the total size computed by adding up the size in bytes of all transactions in the list inside a block body, aggregated by tier
  1. `totalRefScriptSize : TierNo ⇀ ℕ` - the total size computed by adding up the size in bytes of all reference scripts and datums 
  referenced by all the transactions in the list inside a block body, aggregated by tier
  1. `totalExUnits : TierNo ⇀ ExUnits` - the total amounts computed by adding up all the 
  execution units (memory and CPU, 
  separately) specified by all scripts in all the transactions in the list inside a block body, aggregated by tier

There is a new state variable `policyState : SDPolicy` in the `UTxOState`.

Let `adjusted_tier_no` be `priority` if `tx` was in an RB with a *transaction list*, 
and `standard` if `tx` was in an EB. Let `adjusted_tier_coeff` be the coefficient given for `adjusted_tier_no`
by the `diversityPolicy` map in `policyState`. The following are the key rule changes (to transaction application)
having to do with processing the *fee payment* :

  1. updated min-fee constraint (enough to cover *targeted* tier) : `tier_coeff·minfee ≤ txFee`
  1. `txfee - minfee * adjusted_tier_coeff` is the amount of change sent to `reward_account` if it exists, 
  and to the treasury if it does not
  1. exactly `minfee` is sent to the fee pot
  1. `minfee * (adjusted_tier_coeff - 1)` is sent to the treasury

The following changes to transaction application ensure correct tier specification 
with respect to `policyState` :

  1. The tier coefficient that the `diversityPolicy` map in `policyState` associates with the `tier_no` specified in 
  the `tx_tier` in the transaction body is `≤ tier_coeff` in `tx_tier`
  1. The tier number in `tx_tier` must be either `priority` or `regular`
  1. The tier number in `tx_tier` is `≤ adjusted_tier_no` 
  1. `policyState` is updated so that the current aggregated values 2-4 reflect `tx`

Note that these constraints together guarantee that the change amount is never negative: the transaction 
covers the coefficient quoted for its targeted tier, and it can only be included in that tier or a lower one, 
whose coefficient is no higher.

**TODO** : the two controllers are independent, so the coefficient computation can output a standard 
coefficient higher than the urgent (priority) one. Specify that in this case the ledger sets the urgent 
quote equal to the standard quote, so that the guarantee above holds across temporary quote crossings.

#### Block validity

This CIP relies on Leios block structure. For this reason, we change the top-level block processing.
The block requires an additional field `ebCert : Maybe EBCert`, which is an endorsement block certificate, 
and the block header body also must specify the block type (`EB` or `RB`). 

A block can either contain a list of transactions or an `ebCert`. If a block is of `RB` type and contains a list of transactions, 
it is processed similarly to a Praos block :
  - block-level checks are performed (including that `ebCert` is not included), 
  - the list of transactions is processed
  - after processing the transactions, additional 
  [computation and validation](#additional-post-transaction-application-validation) 
  is performed to modify the state variables used to 
keep track of dynamic pricing.

A block of `RB` type that contains an `ebCert` requires that :
  - block-level checks are performed (same as above),
  - the block-processing rule is called again on the `EB` block corresponding to the `ebCert`

If a block is one that corresponds to an `ebCert` (and is therefore an `EB` block), 
  - it must contain a list of transactions,
  - block-level checks are performed (may be specific to `EB` blocks)
  - each transaction is processed
  - additional [computation and validation](#additional-post-transaction-application-validation) is performed


#### Additional post-transaction-application validation 

The `SDPolicy` state is updated during the application of the transactions in the block. After this is complete,
additional checks 
are performed, and this state is further updated, given the current protocol parameters and 
the block type, with the following steps:

  1. Check that if the block containing the transaction list is an EB, it qualifies under the 
  [Endorser Block announcement threshold](#endorser-block-announcement-threshold) rule: unless the age escape applies, 
  at least one of the resource totals (`totalSize`, `totalRefScriptSize`, or either component of `totalExUnits`) 
  reaches the threshold fraction of the corresponding per-block RB limit specified in the protocol parameters
  1. Reset `totalSize , totalRefScriptSize , totalExUnits` to be empty, so that the variables can be reused to 
  track data in the next block
  1. Update the `diversityPolicy : TierNo ⇀ PolicyClause` field of the `SDPolicy` state to specify new 
  coefficients associated with each tier, computed using the moving average over `coeffWindow`
  1. Append the newly computed tier coefficients to `coeffWindow`, dropping the oldest entry whenever 
  the window exceeds `windowSize` (the protocol parameter) entries
  
**NOTE : The calculation in the final step is left unspecified only in the Agda specification, where it is kept 
abstract. The recommended update rule is the controller formula given in the 
[Controller updates and signals](#controller-updates-and-signals) section; its final calibration will be the 
result of further experimental data**. 

#### New protocol parameter

This CIP introduces a new protocol parameter `windowSize : ℕ`. It specifies the maximum number of per-block 
tier coefficient entries retained in the `coeffWindow` variable of `policyState`, and therefore the length 
of the sliding window over which the moving average used by the coefficient update is computed. Like other 
protocol parameters, it is updatable by governance. Larger values smooth the coefficient trajectory at the 
cost of a slower response to changes in demand; a natural initial value is the largest signal window used 
by the recommended construction (20 blocks, the standard controller's window). 


### Block production and node policy

Block producers must account for fee change over time under dynamic fees. Consider this case:

1. A transaction is submitted to the dynamically priced urgent lane during a time of congestion, with more urgent transactions than Ranking Block space. The transaction's posted fee covers the necessary fee _at that time_ but no more.
2. A Ranking Block is produced, but the submitted transaction misses it due to the congestion.
3. The price increases, and the submitted transaction becomes stale. It wasted mempool space while it queued.

The producer-side rule follows from this: a prudent producer fills an EB only with transactions whose max fee covers the quote one price update ahead. One update can fire between selection and the certification check, and an EB filled this way cannot fail fee validation when certified. The rule is EB-specific: RB inclusion is immediate, so RB selection needs only the current quote. The "Revalidation and stale fees" section describes the admission-side counterpart of this rule.

<!-- PORTABILITY: the fee change CIP link below points at a fork branch; repoint at its CIPs-repo PR (or CIP number) once one exists -->

Reminder:

If lane $l$ has a controller:

$$ \mathrm{stepBound}_{l} = \max\left( \frac{1}{D}, \frac{1-\mathrm{targetUtilisation}_{l}}{\mathrm{targetUtilisation}_{l} \times D} \right) $$

Otherwise:

$$ \mathrm{stepBound}_{l} = 0 $$

For a standard transaction:

$$ \mathrm{maxFee} \ge \mathrm{quote}_{\mathrm{standard}} \times \left(1+\mathrm{stepBound}_{\mathrm{standard}}\right) $$

For an urgent transaction:

$$ \mathrm{maxFee} \ge \max\left( \mathrm{quote}_{\mathrm{standard}} \times \left(1+\mathrm{stepBound}_{\mathrm{standard}}\right), \mathrm{quote}_{\mathrm{urgent}} \times \left(1+\mathrm{stepBound}_{\mathrm{urgent}}\right) \right) $$

These fee-cap rules mean the bare current quote is never sufficient: a user must submit with a buffer against quote movement. With the lane-specific $\mathrm{stepBound}$ values defined under the "Revalidation and stale fees" section, a lane's quote can rise to at most $\mathrm{quote} \times (1 + \mathrm{stepBound})^k$ over $k$ worst-case updates. The ledger itself demands no buffer at all: at inclusion, the posted maximum need only cover the quote at that moment. The one-step requirements are node policy: admission checks one worst-case step ahead of the quote at admission, and an EB producer repeats the same check against the quote at selection. Anything beyond that is the user's insurance against eviction while they wait. A transaction that queues through $k$ price updates keeps its place only while its posted maximum covers the current fee-cap quote. A user who expects to wait $k$ updates must therefore post enough to cover every applicable lane's quote after $k$ worst-case steps (for an urgent transaction, the larger of the two). At the recommended $D = 16$, both lanes' $\mathrm{stepBound}$ is $1/16$: the urgent target of $0.5$ makes the two branches of the $\mathrm{stepBound}$ expression equal, and at the standard target of $0.75$ the $1/D$ branch is the larger. The requirement is therefore $(1 + 1/16)^k$ times the current fee-cap quote. A transaction that expects to wait four updates posts roughly 27% above it. A buffer is palatable only with a refund of the difference between the posted fee and the actual quote charged at inclusion. [The fee change CIP](https://github.com/polinavino/CIPs/tree/fee-change/CIP-%3F%3F%3F%3F) describes this refund mechanism.

The urgent premium is scoped to the Ranking Block (rb-only). An urgent transaction included via an Endorser Block instead pays the standard quote at inclusion time, and the refund returns everything above it. The premium buys the reserved lane. A user whose transaction does not receive Ranking Block inclusion does not pay for it.

Settlement must never silently cap the charge below the applicable quote. If the posted maximum does not cover the inclusion-point quote, the transaction is invalid for inclusion. The max-of-lane fee-cap rule above makes that invariant hold even while the lane quotes are inverted.

Settlement at inclusion splits the posted bid three ways:

```mermaid
flowchart LR
    INC["Transaction included:<br/>posted bid b,<br/>quote q at inclusion time"] --> BASE["Base: the ordinary min fee,<br/>to the fee pot"]
    INC --> PREM["Premium: q minus base,<br/>to the treasury"]
    INC --> REF["Refund: b minus q,<br/>to the refund account"]
    EBNOTE["Urgent transaction included<br/>via EB: q is the<br/>standard quote"] -.-> INC
```

### Endorser Block announcement threshold

The name describes the producer policy: a prudent producer does not announce an EB whose certificate cannot yet qualify. Nodes check the consensus rule itself when an RB includes the EB certificate.

The reservation rule above creates a problem at light loads. When the RB is reserved for urgent transactions, any standard traffic, however small, can trigger the announcement of an Endorser Block. Each EB that is later certified consumes Ranking Block space for its certificate. Announcements that cannot be certified are discarded. At loads below RB saturation the EBs are thin. A certificate then costs more RB capacity than the payload it delivers, and urgent transactions lose Ranking Blocks to certificates.

At certificate inclusion, the certificate-bearing RB rule evaluates the `qualifies(EB)` predicate below. The rule uses the three resource groups that `SDPolicy` accumulates for the certified EB: `totalSize`, `totalRefScriptSize`, and the memory and step components of `totalExUnits`. The rule derives the K age escape from the chain state.

Each total below means the sum across tiers over the whole immutable EB. With the corresponding positive RB limits (`maxBlockSize`, `maxRefScriptSizePerBlock`, and `maxBlockExUnits`) from the protocol parameters that validate the certifying RB, define

```
thresholdFraction  = max(1 - urgentTargetUtilisation, 1/2)
txThreshold        = ceil(thresholdFraction × maxBlockSize)
refScriptThreshold = ceil(thresholdFraction × maxRefScriptSizePerBlock)
memoryThreshold    = ceil(thresholdFraction × maxBlockExUnits.memory)
stepsThreshold     = ceil(thresholdFraction × maxBlockExUnits.steps)

resourceQualified(EB) = totalSize >= txThreshold
                        or totalRefScriptSize >= refScriptThreshold
                        or totalExUnits.memory >= memoryThreshold
                        or totalExUnits.steps >= stepsThreshold

qualifies(EB) = EB is non-empty
                and (resourceQualified(EB) or the K age escape applies)
```

Any single resource component reaching its threshold qualifies the EB alone. The rule neither adds ratios nor treats the resources as interchangeable. At the default urgent-controller target utilisation of 0.5, each threshold is half its corresponding RB limit, including a transaction-size threshold of 45,056 B under the simulated RB cap.

Comparisons use integer totals and rounded-up integer thresholds. Floating-point arithmetic is not part of consensus. For any scalar component with usage `x` and positive RB limit `L`, if `urgentTargetUtilisation = p/q`, where `0 <= p <= q` and `q > 0`, the component reaches its threshold exactly when `q × x >= (q - p) × L` and `2 × x >= L`. Implementations evaluate products as mathematical natural numbers or with checked, sufficiently wide intermediates.

The fraction follows the urgent target because a displaced non-certificate Ranking Block carries urgent traffic. A lower urgent target runs Ranking Blocks deliberately emptier, so the urgent lane needs more of them to move the same traffic. Certificates must then be rarer, and qualifying EBs correspondingly fuller. At the default target, qualification requires half the RB limit in any single resource component. This does not claim that the resources are fungible or that an EB replaces the displaced RB component by component. When the controller target rises above 0.5, the half-RB floor holds the threshold at half. Under the threshold alone, an EB below the qualifying fraction cannot be certified, and standard transactions queue for the next worthwhile batch. The age escape below relaxes that per-certificate property to an amortised one. The Ranking Block rule remains untouched: RBs carry only urgent-paying transactions, at all loads, at all times.

None of these rules requires a validator to know anything about any mempool. Fee validation enforces that every Ranking Block transaction pays the urgent quote, and the quote itself is recomputable from the chain alone: each controller update is a fixed formula over the utilisation of the blocks before it. When `LEDGERS` processes the immutable EB named by a certificate, it accumulates the `SDPolicy` resource totals, and the certificate-inclusion rule compares them with the RB-relative thresholds above. The age escape only counts Ranking Blocks since an EB certificate last entered the chain. A validator that holds only the chain can decide every rule in this section. What a producer's mempool contained never enters into it.

A valid Ranking Block cannot contain a transaction whose on-ledger fee authorisation fails to cover the applicable urgent quote. The premium goes to the treasury rather than the producer, so the protocol offers the producer no direct fee revenue when it undercuts that quote or suppresses an EB. This is an incentive argument, not a broader anti-bribery guarantee: off-chain rebates and side payments, paid ordering within the urgent lane, censorship, withholding, and MEV remain open for the Incentives section. The residual behaviour here is EB suppression: a producer declines to announce a qualifying EB. The RB remains urgent-only regardless, and a later producer can announce the batch. The simulator announces eligible EBs eagerly and does not model withholding, off-protocol side payments, or other adversarial producer behaviour. We explored work-conserving variants that admitted standard transactions into underfull RBs at the standard rate. They retain more value at light loads, but they do not ledger-enforce the applicable urgent quote for RB inclusion, and they leave below-quote side-payment incentives open. We rejected them.

The threshold by itself can starve a trickle load. At very light standard traffic, pooled transactions below every resource threshold can wait indefinitely, and anything that depends on their outputs waits with them. We therefore add a time-gated escape: a certificate for a below-threshold EB can enter the chain once at least K Ranking Blocks have been produced since an EB certificate was last included. The inclusion of any EB certificate resets the count. Both the threshold and the escape are ledger rules, checked when a Ranking Block includes a certificate. A Ranking Block that includes a certificate for a non-qualifying EB is invalid. The rule extends the certificate-inclusion checks that CIP-164 already defines, which every node performs before it accepts a block. Both inputs are on the chain: the count comes from the chain itself, and the certified EB's immutable body determines the `SDPolicy` resource totals that the certificate-inclusion rule checks. Because every certificate inclusion resets the count, at most one below-threshold certificate can appear per K intervals. A reset on certificate inclusion, rather than on announcement, matches what the rule rations: an announced EB that never certifies consumes no Ranking Block space, so it does not reset the count. The escape is permissive, not compulsory. Announcement remains a producer action, and the suppression analysis above is unchanged. The rule remains removable without change to any other rule.

For a candidate certificate-bearing RB `R`, the age count is the number of Ranking Blocks in `(lastCertificateRb, R]`, including `R` itself. Acceptance of any EB certificate resets the count, whether the EB qualified by resource use or by age. If no earlier EB certificate exists, the count starts at mechanism activation.

The certificate-inclusion decision for a non-empty certified payload:

```mermaid
flowchart LR
    P["Certified EB payload"] --> T1{"Transaction size, reference scripts,<br/>or either execution-unit component at threshold?"}
    T1 -- "yes" --> A["Certificate may enter the RB"]
    T1 -- "no" --> T2{"At least K Ranking Blocks<br/>since the last included<br/>EB certificate?"}
    T2 -- "yes: age escape" --> A
    T2 -- "no" --> W["Certificate is invalid<br/>for this RB"]
```

#### Validation evidence

Every threshold experiment below used the simulator's byte-only gate. The results inform the transaction-size branch and the choice of qualification fraction. They do not validate the reference-script or execution-unit branches of the normative rule.

The experiment report and linked configurations provide the supporting setup and detailed results.

##### Experiment 1: low-load threshold

**Hypothesis.** Preventing thin EB announcements should remove the urgent-lane regression caused by certificate overhead at low load.

**Experiment description.** Paired low-load runs compared two otherwise identical reserved-RB designs: one allowed any non-empty EB to be announced, while the other required the EB payload to reach half the RB byte cap ([supporting setup and detailed results](https://github.com/input-output-hk/tiered-pricing/blob/main/docs/phase-2/preliminary-experiment-report.md#low-load-below-rb-capacity)).

**Result.** In the historical shared-stream runs, the runs with the threshold recorded +3.03 ± 1.11 percentage points more urgent-class retained value than plain reservation and no statistically detectable difference from the flat-fee baseline (+1.01 ± 1.46 percentage points, interval spanning zero).

**Interpretation.** The observed pattern is consistent with the intended mechanism: standard traffic batches into fuller EBs, which avoids spending Ranking Block capacity on certificates for thin EBs, while Ranking Blocks stay urgent-only.

##### Experiment 2: announcement age escape

**Hypothesis.** An age escape at K = 10 should repair simulated standard-lane starvation under trickle traffic while remaining inert at ordinary low load.

**Experiment description.** Runs compared the pure threshold with the K = 10 age escape at 0.1 tx/slot and under the ordinary low-load profile ([full methods and results](https://github.com/input-output-hk/tiered-pricing/blob/main/docs/phase-2/preliminary-experiment-report.md#trickle-loads-and-the-announcement-age-escape)).

**Result.** At 0.1 tx/slot the runs with the age escape recorded +83.39 ± 8.59 percentage points more standard retained value, while at ordinary low load they were bit-identical to the pure threshold.

**Interpretation.** The escape corrects the threshold's low-volume starvation edge case without adding observed certificate overhead at ordinary low load. The simulator is idealised: it decides eligibility and resets the count at announcement, and every announced EB certifies, so a reset at announcement and a reset at certification coincide there, apart from the pipeline delay. The specified rule resets on certificate inclusion. That timing offset, and the behaviour when announced EBs fail to certify, were not separately simulated.

##### Experiment 3: parameter stress

**Hypothesis.** The announcement threshold should rise with controller headroom but should not fall below half the Ranking Block byte cap, and the controller defaults should remain robust across different loads.

**Experiment description.** Runs under four load profiles (low, severe congestion, launch day, and EB-capacity stress) swept target utilisation and max-change denominator, and used fixed-threshold variants intended to isolate the headroom term and half-RB floor ([full methods and results](https://github.com/input-output-hk/tiered-pricing/blob/main/docs/phase-2/preliminary-experiment-report.md#parameter-stress-test-controller-settings-and-the-threshold-rule)).

**Result.** Target utilisation 0.25 failed under launch-day load, denominator 4 was unstable, and the historical fixed-threshold comparison runs favoured retaining both branches of the threshold expression.

**Interpretation.** Most sweep comparisons are descriptive because paired runs saw different random demand. The audited target-0.25 comparison is the exception. It provides clean evidence that the threshold should track urgent-controller headroom. Evidence for the half-RB floor is weaker: it relies on the unaudited target-0.75 comparison and the fixed cost of a certificate. The independent-stream D16/K10 rerun and the thousand-seed replication separately confirmed the recommended defaults. Retuning outside the tested range requires new analysis. See the Rationale for the full evidence scope.

##### Experiment 4: default-point byte-threshold sensitivity

**Hypothesis.** At low and mid load under the canonical controller settings, a material byte gate should improve urgent outcomes, while progressively higher gates should expose the corresponding cost to standard traffic.

**Experiment description.** A post-correction experiment compared byte gates of 1 B, one-quarter, one-half, and three-quarters of the RB byte cap under the target-0.5, D16, K = 10 mechanism. It paired seeds 0-99 for 2,000 slots with independent random streams across the five headline loads. The arms differed only in the byte threshold. A directional 20-seed check extended the low- and mid-load comparison from three-quarters to one RB.

**Result.** At low and mid load, higher gates improved urgent retention and latency while delaying standard traffic. Relative to half an RB, three-quarters raised the urgent retained-value ratio by 1.94 and 0.52 percentage points and increased standard latency by 0.232 and 0.026 blocks, respectively. The recorded decision-facing contrasts at the three heavier loads were zero or near zero. The one-RB extension continued the same trade-off. The [preserved ablation record](../experiment-results/default-threshold-ablation.json) holds the paired contrasts and provenance, and the [one-RB check report](../experiment-results/full-rb-quick-20.md) records the directional extension's summary statistics and simulator hash (its per-seed data is not preserved).

**Interpretation.** These experiments map an urgent/standard policy frontier. They do not identify an unconditional optimum. Half an RB remains the conservative initial default because, in the byte-only simulator branch at target 0.5, it matches the minimum qualifying EB byte payload to the expected urgent byte payload displaced by a certificate. The heavier-load checks had no prespecified equivalence margin, and the one-RB run is directional rather than a replacement for the 100-seed experiment. Every arm retained K = 10 and tested only a byte gate. Neither experiment validates an execution-unit predicate or the threshold expression away from target 0.5.

The starvation and its repair are visible directly in the simulation's demand-fate panels (one representative seed, identical crop and scale):

![Demand fate and retained value at a 0.1 tx/slot trickle with no age escape: every standard class is entirely unresolved and no standard value is retained](images/trickle-0p1-thr-noescape-seed-2.png)

![Demand fate and retained value at the same trickle with the age escape at K = 10: all standard units are included and most standard value is retained](images/trickle-0p1-thr-k10-seed-2.png)

The threshold fraction tracks the urgent controller's headroom, but never falls below half an RB. The normative rule applies the same fraction to each `SDPolicy` resource component, and any single component can qualify independently. The experiments varied only the transaction-size threshold and therefore do not validate the other branches.

The historical parameter stress test (ten seeds, four load profiles, detailed in the experiment report) motivates each half of the fraction separately. The sweep derived its byte thresholds from the headroom term at each swept urgent target, while it moved both controller targets together. Fixed-byte-threshold comparison runs at targets 0.25 and 0.75 favoured a qualifying bar that never drops below half the RB byte cap.

In the corrected target-0.25 low-load comparison there were no conditional retry draws, so same-seed runs of the two configurations faced the same exogenous demand and Ranking Block opportunities. The lane and submission outcomes that differ are part of the simulated threshold response. The target-0.75 comparison remains descriptive because no equivalent path audit was preserved. At urgent targets at or below 0.5 the floor does not bind (`1 - urgentTargetUtilisation` is at least `1/2`), so those grid runs realise the completed fraction's values. The completed max() expression was therefore exercised through them and the target-0.75 fixed variant, rather than swept as a unit.

The intuition has two parts. A low urgent target deliberately runs Ranking Blocks emptier, so the urgent lane needs more of them to move the same traffic, and certificates must be correspondingly rarer: the threshold rises with urgent-lane headroom. But a certificate's cost does not shrink when the urgent controller runs blocks hotter, so the threshold must not follow shrinking headroom downward: hence a conservative half-RB floor that limits certificate overhead as headroom shrinks.

The same stress test explores the controller parameters themselves. The sweep set both controllers together at each grid point. Independent per-lane settings were not swept in this stress test, so its results apply only to the two lanes retuned in lockstep. A later dedicated experiment swept the standard-lane target independently and selected 0.75; see [Independent standard-lane target screen and confirmation](#independent-standard-lane-target-screen-and-confirmation).

At the tested grid points with both targets at 0.5 or both at 0.75, and max-change denominator 8 or 16, the observed comparisons were generally favourable or near-baseline. The strength of the evidence differs by load. Launch-day grid points are paired against flat fee, with intervals that exclude zero. Severe-congestion and EB-capacity-stress grid points are paired against the anchor calibration rather than flat fee. The low-load comparison is unpaired against the flat-fee aggregate and sits within about a point either way (two target-0.75 points marginally below it).

With both targets at 0.5 the advantage holds at every contended load. At 0.75 the EB-saturating result is 31.4% urgent-class retained value against flat fee's 30.1%, without an equivalence margin. With both targets at the tested 0.25, the mechanism retains less value than a flat fee under launch-day load.

The threshold expression uses the urgent-controller target. The historical sweep changed both targets together, so it does not estimate independent standard-controller retuning. The controller parameters are specified as updatable protocol parameters, with the tested grid recorded alongside them. Retuning to untested settings is a mechanism change that requires re-analysis, not a routine parameter update. The parameters, their recommended defaults, and the tested points:

| Parameter | Recommended default | Tested points and observations |
|---|---|---|
| Target utilisation (standard and urgent controllers, swept in lockstep) | 0.5 urgent, 0.75 standard | lockstep grid points 0.5 and 0.75 tested. 0.25 tested and excluded (retains less value than flat fee under launch-day load). At lockstep 0.75 the EB-saturating result is 31.4% urgent-class retained value against flat fee's 30.1%, with no equivalence margin. The independent standard-lane target of 0.75 (urgent held at 0.5) was screened on seeds 0-199 and confirmed on seeds 200-299; see [Independent standard-lane target screen and confirmation](#independent-standard-lane-target-screen-and-confirmation) |
| Max-change denominator (both lanes, swept in lockstep) | 16 | grid points 8 and 16 tested. 4 tested and excluded (price instability at every load). Independent per-lane settings not swept |
| Urgent signal window | 5 samples | {3, 5}. Windows of 10-20 trade retention for larger price swings |
| Standard signal window | 20 blocks, capacity-weighted | not swept |
| EB announcement threshold | `thresholdFraction = max(1 - urgentTargetUtilisation, 1/2)`. Unless the age escape applies, a non-empty EB qualifies when any single resource component (transaction size, reference-script size, or either execution-unit component) reaches `ceil(thresholdFraction × corresponding RB limit)`. The default fraction is 1/2 (45,056 B for the simulated transaction-size component) | only the transaction-size threshold was tested: the headroom branch was swept while both controller targets moved together over 0.25-0.75. Historical shared-stream fixed-byte-threshold comparisons exercised 45,056 B at targets 0.25 and 0.75. The reference-script and execution-unit branches remain untested |
| EB announcement age escape (K) | 10 RB intervals | K ∈ {5, 10, 20} swept under the simulator's announcement-reset policy. 10 is bit-identical to no escape at ordinary low load and repairs trickle starvation with no statistically detectable urgent-class cost |
| Absolute coefficient floor | 1.0 × ordinary min fee | not swept |
| Cross-lane multiplier floor | none. Temporary quote crossings are permitted, and urgent max-fee checks use the larger current quote | tested at 3× and 16×, rejected |

<!-- PORTABILITY: blob/main link; replace with a commit-pinned permalink before the CIPs-repo PR -->

See the [parameter stress test section of the experiment report](https://github.com/input-output-hk/tiered-pricing/blob/main/docs/phase-2/preliminary-experiment-report.md#parameter-stress-test-controller-settings-and-the-threshold-rule).

The instability that excludes denominator 4 is visible directly in the price trace. The figures below show the per-lane price coefficient over historical severe-congestion runs with the same numbered seed (2,000 slots, seed 0, target utilisation 0.5) and configurations that differ in the max-change denominator. Because these runs used the shared random stream described above, they are descriptive traces rather than a same-exogenous-draw counterfactual. At denominator 4 the run records 88 price moves over 10% (the largest a 25% jump), and the urgent coefficient completes six full oscillation cycles with a peak-to-trough amplitude of 6.7×. At denominator 16 the run records no move over 10% (the largest is 6.3%), one oscillation cycle, and an amplitude of 1.8×, with similar service rates (98.8% vs 97.9%).

![Per-lane price coefficient under severe congestion at max-change denominator 4: the urgent coefficient repeatedly overshoots and collapses](images/d4.png)

![Per-lane price coefficient under severe congestion at max-change denominator 16: both coefficients track demand smoothly](images/d16.png)

### Incentives

#### Giorgos

From an incentives perspective, we focus on three properties introduced in the [analysis](https://timroughgarden.org/papers/eip1559.pdf) of EIP-1559:

**Incentive-compatibility for myopic miners (MMIC):** Block producers should be incentivized to follow the prescribed transaction inclusion rule.

**User-incentive compatibility (UIC):** There should be an obvious optimal bidding strategy when creating a new transaction. This property is related to having good user experience, and thus making it easy for users to use the system efficiently.

**Off-chain agreement proof (OCA-proof):** It should not be profitable for a user and a block producer to collude in order to include some transaction in the blockchain.

Next, we briefly argue that our design satisfies all three properties outlined above.

**MMIC:** The utility maximizing strategy for the block producer is to include all transactions that pay their fees (if possible), and avoid including any "fake" traffic it generates as it will have to pay the cost. This is exactly the transaction inclusion rule defined by our mechanism. Note, that in Cardano fees are redistributed through the reward mechanism to all block producers.

**UIC:** When prices are not set excessively low, which should be the typical case as prices dynamically adapt to traffic, the optimal bidding strategy for an *urgent* user is to set its fee cap to the maximum value it is willing to pay and submit to the urgent lane. Otherwise, it may either (i) not get the best possible service, if it sets a lower fee-cap or submits its tx to the standard lane, or (ii) risk losing money, if it sets a higher fee-cap.

**OCP:** A user directly paying/bribing a block producer for quick inclusion does not increase their joint utility; someone at the end of the day has to pay the inclusion fee.


As a side-effect, the mechanism proposed may reveal/leak information about the value of certain transactions; a transaction submitted to the urgent lane may be correlated 
with high-value, and further with high miner extractable value (MEV). While this may make MEV attacks simpler to launch, as it would be easier to identify 
potential targets, we note that the quick inclusion of txs submitted to the urgent lane offers some protection against them--high value urgent txs settle faster through our mechanism. 
Further, a tx that is highly sensitive to MEV attacks can still be submitted privately to some node operator first. 
Note also, that in Cardano today high-MEV txs are not fully  protected against attacks, i.e., a block producer that 
observes such a tx in some block, may decide to fork the chain in order to take the MEV opportunity itself. Thus, besides possibly leaking some information about a tx's value,  
a public urgency signal does not create extra/new MEV opportunities compared to a publicly submitted txs in current mainnet. 

#### Nicolas

From an incentives perspective, we use three properties introduced in the [analysis](https://timroughgarden.org/papers/eip1559.pdf) of EIP-1559 as evaluation criteria:

- **Myopic miner incentive compatibility (MMIC):** a block producer should be incentivised to follow the prescribed transaction-inclusion rule.
- **User incentive compatibility (UIC):** a user should have a clear bidding strategy when creating a transaction.
- **Off-chain agreement proofness (OCA-proofness):** a user and a block producer should not be able to increase their joint utility by replacing the protocol payment with an off-chain agreement.

The mechanism is designed to improve these properties, but the arguments below do not constitute a formal proof that all three hold in the presence of off-chain payments, paid ordering, censorship, Endorser Block withholding, or other forms of MEV.

Settlement splits every posted bid into three components:

- the ordinary min-fee component, which enters the existing fee pot;
- the urgency premium, which is donated to the treasury rather than paid to the current block producer; and
- any excess above the applicable quote, which is returned to the transaction's refund account.

Each destination has an incentive effect. Keeping the ordinary min fee in the fee pot preserves the existing protocol fee flow. Sending the urgency premium to the treasury prevents the current producer from directly capturing the congestion rent; burning the premium would be similarly neutral for that producer, but donating it keeps the value within the protocol. Refunding the excess makes `maxFee` a spending ceiling rather than the amount the user necessarily pays.

| Property | Contribution of the mechanism | Remaining limitations |
| --- | --- | --- |
| **MMIC** | The producer continues to receive protocol rewards through the existing fee pot, while the urgency premium is not paid directly to the current producer. Ledger validation also prevents Ranking Block inclusion below the urgent quote. | FIFO ordering is node policy, not a ledger rule. Tips, paid ordering among eligible urgent transactions, fake traffic, censorship, and Endorser Block withholding remain possible. |
| **UIC** | A user can set `maxFee` to the maximum amount it is willing to pay without automatically paying that amount. The urgency premium is charged only when the transaction receives Ranking Block delivery; an urgent transaction delivered through an Endorser Block is charged the standard quote. | Lane choice and the required fee-cap headroom still depend on changing quotes, congestion, and the user's latency requirements. This section does not establish a dominant bidding strategy. |
| **OCA-proofness** | Routing the premium to the treasury and enforcing the urgent quote remove the direct benefit of selling below-quote Ranking Block access. | Off-chain rebates, side payments, paid ordering among already eligible transactions, and other agreements remain possible. The mechanism therefore closes a specific collusion channel rather than establishing full OCA-proofness. |

The standard lane is insulated from urgent demand because its quote responds only to standard-lane utilisation and cannot fall below the ordinary min fee. An uncontended standard transaction therefore pays the same protocol fee as today. The trade-off is batching latency: standard transactions wait until the selected Endorser Block payload reaches the announcement threshold or until the `K = 10` age escape opens.

Settlement is conservative: for every included transaction, the base fee, premium, and refund sum to the posted bid, and every component is checkable from on-chain data. Fee handling therefore neither mints nor destroys value.

Finally, the urgency signal may reveal that a transaction is latency-sensitive or high-value. Faster settlement can shorten its exposure to some forms of MEV, but it does not provide general MEV protection and may itself reveal useful information to block producers. A sensitive transaction may still be submitted privately, but private submission and protection from reordering or chain forks are outside the guarantees of this mechanism.




## Rationale: how does this CIP achieve its goals?

This CIP specifies a design, reinforces the design choice with experimental evidence, validates the design with formal specifications and proofs, and demonstrates the mechanism's behaviour in a working simulator.

### How this CIP addresses CPS-0031

<!-- PORTABILITY: CIP-9999 link becomes the repo-relative ../CIP-9999 once inside the CIPs repo -->

[CIP-9999](https://github.com/cardano-foundation/CIPs/tree/master/CIP-9999) requires a proposal that answers a problem statement to carry that statement's open questions in its Rationale and to give an argued answer to each. This section does that for [CPS-0031](https://github.com/cardano-foundation/CIPs/pull/1194), and it also answers the goals against which CPS-0031 asks candidate solutions to be judged. Where an answer rests on measurements, the measurements are in [Experimental evidence](#experimental-evidence) below and the argument here does not repeat them.

#### Goal 1: reduce value destroyed by avoidable delay

CPS-0031 asks that urgent transactions have a way to avoid value-destroying delay when they compete with traffic that has no time sensitivity. Retained value measures the value that delay destroys, so this goal is judged on the retained-value results. The mechanism preserves more urgent-class value than the flat fee at every contended load: +7.16 percentage points under severe congestion in the thousand-seed replication, +3.69 at mid load, and +8.57 under EB-capacity stress. Under the launch-day profile the measured improvement covers all traffic: overall retained value rises by +8.15 points. Urgent-class mean latency falls with it, from 2.97 to 2.51 blocks under severe congestion. Two channels carry the improvement. The reserved Ranking Block path gives urgent-paying transactions an earlier inclusion opportunity, and the rising standard quote makes low-surplus demand decline to submit during congestion, which leaves capacity for the demand that values inclusion most. The cost sits at low load, where overall retained value is 0.40 percentage points below flat fee while any urgent-class effect is bounded within roughly ±0.15 points. CPS-0031 lists guaranteed value retention as a non-goal, and this mechanism does not promise it: the improvement is a measured shift in the distribution of outcomes, detailed in [Experimental evidence](#experimental-evidence).

#### Goal 2: permissionless access

CPS-0031 requires that priority be available to anyone willing to fulfil the necessary prerequisites, and not negotiated through relationships or private arrangements. Under this mechanism the only prerequisite is the fee. Any transaction whose posted fee covers the urgent quote is eligible for the urgent lane. The quote is a single posted price that every node computes from the chain by the same formula, so every user sees the same cost at the same time, and no gatekeeper sits between a user and the lane. A relationship cannot substitute for the quote either: a Ranking Block that contains a transaction whose fee authorisation does not cover the urgent quote is invalid, so a producer cannot sell below-quote access at any price. The premium above the ordinary min fee goes to the treasury rather than to the producer, and [Incentives](#incentives) argues that this removes the direct benefit of an off-chain agreement to sell below-quote access, while recording that other off-chain arrangements remain possible. This is the public counterpart of the private SPO arrangements that CPS-0031 reports and rejects: the same priority, but uniformly priced, open to all, and verifiable on-chain.

#### Goal 3: predictability over raw speed

CPS-0031 asks for a more consistent wait for high-urgency transactions. We report two figures, counted in Ranking Blocks: the wait of a typical transaction, which is the median, and the wait of the 95th percentile. The distance between them is the width of the band that separates the two cases. We count this in blocks because this mechanism decides which block a transaction enters, not when that block is produced.

The answer is affirmative at both replicated loads. Under severe congestion the urgent class's 95th-percentile wait falls from 5.74 to 4.79 blocks, and the distance between its median and its 95th percentile falls from 3.30 to 2.79 blocks. At low load that distance falls from 0.96 to 0.54 blocks, in a regime where the retained-value difference is null. These waits were measured with both controller targets at 0.5. Under the recommended 0.75 standard target the low-load outcomes are bit-identical, and the severe-congestion improvement remains but is smaller: over the confirmation seeds the urgent class's 95th-percentile wait improves on flat fee by 0.56 blocks, against 0.79 for the 0.5 calibration on the same seeds ([Independent standard-lane target screen and confirmation](#independent-standard-lane-target-screen-and-confirmation)). The [thousand-seed replication](#thousand-seed-replication-at-low-and-severe-congestion-load) gives the confidence intervals, the per-seed counts, the slot measurements, and the two limits that qualify the result.

A testnet pass criterion follows. Because a percentile is itself a wait, it can be written straight into an acceptance criterion, in the form "the urgent class's 95th-percentile wait improves by at least N blocks". A tester reads one number off a run and decides whether the criterion holds. [Path to Active](#acceptance-criteria) uses this form.

#### Constraints

CPS-0031 sets four constraints on candidate solutions.

**Ledger determinism.** Every rule this CIP adds is a deterministic function of the chain. Each lane's quote follows from a fixed update formula over the utilisation of the blocks before it, the threshold comparisons use integer totals against rounded-up integer thresholds with no floating-point arithmetic, and the age escape counts Ranking Blocks on the chain itself. A validator that holds only the chain can decide every rule. Mempool contents, wall-clock time, and producer behaviour never enter validation.

**On-chain record.** The cost of signalling and its update rule are ledger state and ledger rules. The per-tier coefficients live in the `SDPolicy` state inside `UTxOState`, the `DIVUP` rule updates them during block processing, and the controller parameters are protocol parameters. Anyone can read the current cost of urgency from the chain and verify that every update was computed correctly, which is the equal access the constraint asks for.

**Censorship resistance.** Access to priority is by posted price alone, as argued under Goal 2, so the mechanism replaces the pressure toward opaque off-chain priority with a public channel. The premium goes to the treasury, so preferential treatment earns a producer no direct fee revenue. The constraint also asks for an evaluation of new opportunities for selective exclusion, and there are two. The public lane field gives a censoring producer one extra bit to select on, which is inseparable from having a signal at all. A producer can also suppress a qualifying Endorser Block by declining to announce it. The harm is limited, because a later producer can announce the batch and Ranking Blocks remain urgent-only regardless ([Endorser Block announcement threshold](#endorser-block-announcement-threshold)). Censorship and withholding beyond these arguments stay within the scope of the independent audit required in [Path to Active](#acceptance-criteria).

**Linear-Leios compatibility.** The mechanism is not an existing design ported onto linear-Leios: it is made of linear-Leios parts. Its two lanes are the two CIP-164 block types, its certificate rules extend the certificate-inclusion checks that CIP-164 already defines, and the acceptance criteria require CIP-164 to be active before or with this mechanism. The block-structure question below gives the detail.

#### Open questions

CPS-0031 closes with seven open questions. Each is answered in turn.

**How can whatever protocol-level commitments are decided upon be enforced or incentivised?**

This CIP separates protocol commitments, which ledger rules enforce, from implementation policies, which incentives hold in place. The "Enforcement boundary" row in [The recommended construction](#the-recommended-construction) records the split. The ledger enforces the protocol commitments: Ranking Block lane eligibility, inclusion-point fee validity, settlement, deterministic quote updates, and Endorser Block certificate eligibility. A block that breaks any of them is invalid, so every validating node enforces them and no trust in the producer is needed. Mempool organisation, transaction ordering, admission headroom, revalidation, and eviction are implementation policies rather than protocol commitments. The reference policy preserves the canonical FIFO queue and adds an urgent view, providing an efficient implementation. [Incentives](#incentives) argues that the fee flows support the policies: the producer keeps its existing protocol rewards while the premium goes to the treasury, a user can post its maximum willingness to pay without automatically paying it, and selling below-quote Ranking Block access earns a producer nothing. The same section records the limits of these arguments. Sending the premium to the treasury removes the key bribery incentive, because a producer cannot earn anything by placing a standard-paying transaction in a Ranking Block. Two producer behaviours remain outside ledger enforcement: suppression of a qualifying Endorser Block, and off-chain side payments. Suppression does limited harm: a later producer can announce the withheld Endorser Block, and Ranking Blocks stay urgent-only in the meantime ([Endorser Block announcement threshold](#endorser-block-announcement-threshold)). The incentive argument above removes the direct fee benefit of side payments, but it does not rule them out.

**How should updated fee or priority quotes be propagated?**

They do not need to be propagated at all. Each lane's quote is recomputable from the chain alone: every controller update is a fixed formula over the utilisation of the blocks before it, so any two nodes that agree on the chain tip agree on both quotes. Quote propagation therefore reduces to block propagation, which linear-Leios already provides, and a wallet reads the current quotes from any synchronised node. What remains is drift between the quote at submission and the quote at inclusion. That is a validity question, not a propagation question, and the repricing-interval answer below covers it.

**How would a priority signal interact with the linear-Leios block structure?**

The signal is built out of the block structure rather than layered over it. Linear-Leios provides two block types with different latencies, and the mechanism assigns one lane to each: Ranking Blocks carry only urgent-paying transactions, while Endorser Blocks can carry both lanes. Vanilla linear-Leios uses Endorser Blocks only when traffic exceeds Ranking Block capacity; this proposal routes standard traffic through them at every load, and the resulting latency gap between the two paths is what the urgent premium buys. The structural point of friction is that a certificate-carrying Ranking Block is payload-free, so Endorser Block certificates and urgent transactions compete for the same Ranking Block space. The announcement threshold and its age escape exist to manage exactly that competition. The interaction extends into transaction diffusion: the transaction envelope repeats the lane field, so a mempool can request urgent transactions from its peers before standard ones without inspecting transaction bodies ([Mempool](#mempool)).

**Can we achieve our goals without starving low-urgency users of block space, especially in the context of linear-Leios?**

Yes, at a measured delay cost and with a rule that repairs the worst case. Standard transactions keep access to Endorser Blocks at every load, and an Endorser Block carries two orders of magnitude more capacity than a Ranking Block. The starvation risk is specific to light traffic, where the announcement threshold makes standard transactions pool until an Endorser Block is worth its certificate. At a trickle the pure threshold does starve them, and the age escape repairs it: at 0.1 tx/slot the escape restored +83.39 ± 8.59 percentage points of standard retained value, and at ordinary low load it was bit-identical to the pure threshold. Under sustained congestion, the case the CPS highlights, standard traffic pays no measured delay cost, because Endorser Blocks fill on their own: every standard-lane mean and quantile difference is at or near zero. The cost falls on the loads where standard traffic must pool. At low load, transactions that stay standard wait a mean of 2.99 blocks against 1.78 under flat fee, and overall retained value sits 0.40 percentage points below the flat fee. At mid load the ten-seed rerun also records a longer standard wait. The [thousand-seed replication](#thousand-seed-replication-at-low-and-severe-congestion-load) prices that trade, and the acceptance criteria require declared standard-class regression bounds, including the 95th-percentile delay, before any testnet pass.

**How can we retain fee quote validity across repricing intervals?**

Three tools keep a queued transaction's fee valid while prices move: a posted maximum fee, a refund, and bounded price steps. A user posts the most they agree to pay. Settlement charges only the applicable quote at inclusion and returns the excess to the refund account, so posting a buffer against repricing costs nothing at inclusion. Each controller step is bounded, within ±6.25% per update at the recommended parameters, so the buffer that survives k updates is computable in advance: roughly 27% covers four worst-case updates at the defaults. Node policy then layers the checks by when they act. Admission requires the posted maximum to cover one worst-case controller step, an Endorser Block producer requires one further step at selection so that a certified Endorser Block can never fail fee validation, and eviction removes the transactions that a rising quote overtakes anyway. The ledger itself demands validity only at the inclusion point. [Revalidation and stale fees](#revalidation-and-stale-fees) and [Block production and node policy](#block-production-and-node-policy) specify the two sides.

**What information is leaked when a transaction signals urgency?**

One bit of lane choice, plus a fee ceiling. The lane field appears in the transaction body and is repeated in the transaction envelope, so the network can read it without inspecting the body. The posted fee reveals an upper bound on what the submitter will pay, and the bidding strategy that [Incentives](#incentives) describes sets that bound at the submitter's true maximum, so an observer can estimate what timely inclusion is worth to an urgent submitter. The refund account is the only other new field. One bit is the minimum for any mechanism that signals urgency at all, and a two-lane design stays at that minimum. Finer tiers would reveal a transaction's urgency, and so potentially its purpose, more precisely; [Why not full tiered pricing?](#why-not-full-tiered-pricing) notes this front-running surface as a security-adjacent concern with the tiered design. The CPS's own on-chain-record constraint requires the price of the signal to be public.

**What MEV opportunities are created or amplified by public urgency signals?**

The new information an extractor gains is the mark itself: the urgent lane collects transactions that are worth watching, such as liquidations and oracle updates, without any inspection of their contents. The design limits what that mark is worth. The quote is a posted price set by the controller, not a bid set by competition between users, so the signal creates no priority auction that an extractor can outbid, and the optimal user strategy stays the obvious one. The premium goes to the treasury, so a producer gains no direct fee revenue by reordering or displacing urgent transactions, and [Incentives](#incentives) argues that the collusive variant, an off-chain agreement between a user and a producer, earns the producer no direct fee revenue, though such agreements remain possible. Ordering within a lane is node policy rather than a ledger rule, so ordering-dependent extraction that exists today is neither created nor removed by this CIP. Two limits remain. Once urgent demand alone exceeds Ranking Block capacity the signal stops discriminating, and the [Tipping](#tipping) extension discusses the explicit in-lane priority market that could follow. And the simulator does not model adversarial producer behaviour, so these are incentive arguments rather than measurements; the independent audit in [Path to Active](#acceptance-criteria) covers the incentives analysis.

### Experimental evidence

> **Evidence scope.** Most quantitative tables below and in the preliminary report predate the max-of-two fee-cap correction and used the simulator’s historical shared random stream. Apart from the dedicated 3×/16× multiplier-floor experiment, they also used no cross-lane floor (multiplierFloor: null). Treat their non-identical comparisons as descriptive historical results, not post-correction causal estimates. The matched denominator-16 and integrated canonical D16/K10 checks were exactly unchanged across all 550 reported scalars, but were bounded checks rather than equivalence tests. The D16/K10 headline rerun, thousand-seed replication, default-point byte-threshold sensitivity experiment, the standard-lane target screen, replication, and confirmation (seeds 0-299), and the window-removal experiment (pilot seeds 300-304, confirmation seeds 400-409) postdate the correction and use independent random streams. Confidence intervals measure seed-sampling uncertainty conditional on the simulator and its fixed behavioural assumptions; they do not capture model or calibration uncertainty. In a one-round severe-congestion sensitivity check, recalibrating lane-latency beliefs moved the ten-seed urgent-class retained-value difference from +7.29 to +6.69 percentage points.

Our experimental setup was as follows:

| Family | Reservation policy | Standard lane | Urgent lane | Signal variants |
|---|---|---|---|---|
| flat-fee | none | fixed | n/a | n/a |
| single-lane-eip1559 | none | dynamic | n/a | n/a |
| priority-only-open | open priority-first | fixed | dynamic | instant, windowed 3-20 |
| priority-only-reserved | RB reserved | fixed | dynamic | instant, windowed 3-20 |
| priority-only-strict-threshold | RB reserved. Simulator announces an EB only at ≥ half-RB byte payload | fixed | dynamic | windowed 5 |
| both-dynamic-open | open priority-first | dynamic | dynamic | instant, windowed 3-20 |
| both-dynamic-reserved | RB reserved | dynamic | dynamic | instant, windowed 3-20 |
| both-dynamic-strict-threshold | RB reserved. Simulator announces an EB only at ≥ half-RB byte payload | dynamic | dynamic | windowed 5 |

We ran 10 seeds of a 2000 slot simulation under five load profiles: `severe-congestion` (mean 40 tx/slot in slots 0-249 and 1750-1999, mean 160 tx/slot in slots 250-1749), `low` (constant 3 tx/slot, below RB saturation), `mid-load` (constant 5 tx/slot, just above RB saturation), `eb-capacity-stress` (repeated peaks up to 400 tx/slot driving demand against the EB byte cap), and `launch-day` (measured January 2022 SundaeSwap byte-fullness stages rescaled to the simulator's EB capacity, with modelled onset overshoot and urgency multipliers).

The byte-only simulator realisation of the recommended mechanism is both-dynamic-strict-threshold with a 5-sample signal window and max-change denominator 16. The headline figures in this section were generated with both controller targets at 0.5. The recommended standard-lane target was later raised to 0.75 by a dedicated experiment; on its confirmatory seeds, the 0.75 standard target retained at least as much overall and urgent-class value as the 0.5 calibration at every load where the comparison exists, so the headline value figures are not overstated for the recommended configuration ([Independent standard-lane target screen and confirmation](#independent-standard-lane-target-screen-and-confirmation)). Under severe congestion it improves urgent-class retained value from 43.56% (flat fee) to 50.85% (+7.29 ± 1.29 percentage points, paired over ten seeds against a matched flat-fee control). It also reduces urgent-class mean latency from 2.98 to 2.50 blocks.

All per-load figures in this and the following paragraph are from the D16/K10 headline rerun below. The experiment report's comparison tables were generated on the denominator-8 anchor configuration. In the historical shared-stream parameter sweep, the reported paired D16-versus-D8 retained-value intervals for severe congestion and EB-capacity stress span zero. Low-load context is unpaired, and mid load was not swept. Denominator 16 bounds every controller step to 6.25%, so single-step price shocks cannot occur by construction. The empirical stability evidence is the oscillation behaviour: in the severe-congestion price traces, one cycle at 1.8× peak-to-trough amplitude against six cycles at 6.7× for denominator 4. Under an extreme high-value demand mix, denominator 8 retains ~2 percentage points more at a large stability cost. 16 remains the default because of the asymmetry of those error modes. 8 remains among the tested settings if persistently steep demand emerges.

At mid load the mechanism beats flat fee by +3.69 ± 1.06 percentage points (ten of ten seeds), and under the EB-stressing load by +8.57 ± 2.24 (ten of ten). Low load is the regime where plain reservation regresses below flat fee. There, the EB threshold repairs the regression and leaves no statistically detectable urgent-class difference from the flat-fee baseline (+0.47 ± 1.62 over ten seeds, and the thousand-seed replication below tightens the bound to -0.03 ± 0.12). Under the launch-day profile the mechanism beats flat fee by +8.15 ± 2.04 percentage points of overall offered value (ten of ten seeds, where offered value is proxied per seed by the flat-fee control's total submitted value, retained + lost + unresolved, because the summary output does not record demand that declines before first submission). Admission is a central channel. The rising standard quote makes low-surplus demand decline to submit. The demand that remains is included with much higher probability, and included transactions also wait less than under flat fee.

We eliminated the other families as follows. Flat fee and single-lane EIP-1559 provide no way to signal urgency, and they leave urgent-class value on the table at every contended load. The historical open-versus-reserved runs record a small gap where capacity is slack (~1-1.6 percentage points at low and mid load). Because the variants differ in more than ledger enforcement, this is a descriptive tradeoff rather than an isolated price of enforcement. Plain reservation falls below the flat-fee baseline at low load, because any standard overflow, however small, triggers a thin EB whose certificate consumes Ranking Block space. Work-conserving variants that admitted standard transactions into underfull RBs at the standard rate retained the most value at light loads. But they likewise leave below-quote RB access and side-payment incentives open, so we rejected them. Long signal windows (10-20 samples) reduce shock counts but trade retention for larger peak-to-trough price swings. The 5-sample window is the compromise point. We prefer both-dynamic over priority-only for two reasons. First, under the EB-stressing load (37.50% vs 32.92% urgent-class retained value), the recorded behaviour is consistent with a standard-lane price that sheds the demand that saturates the Endorser Block. Second, under the historical launch-day load, reservation over a statically-priced standard lane showed no statistically detectable improvement over flat fee, while both-dynamic under the same reservation rule recorded a clear improvement. Within the tested simulator designs, this favours both-dynamic rather than making it a protocol-level requirement. At low and mid load the two families produce identical results: standard traffic never touches the Ranking Block and the standard controller rests at its floor, so both-dynamic degenerates to its priority-only counterpart. Under severe congestion they differ slightly on the denominator-8 anchor tables (51.55% vs 50.74% urgent-class retained value, with both-dynamic carrying roughly five fewer transactions per slot as the standard price sheds demand).

The launch-day contrast is visible in the demand-fate and value panels for a representative seed. In the first figure, note the priority (Pri) rows: under reservation over a statically-priced standard lane, priority demand itself is heavily abandoned, because it bounces at admission behind the standard-lane jam. Under both-dynamic with the same reservation rule, most demand is included and most value retained.

![Demand fate and retained value by urgency class under launch-day load with reservation over a statically-priced standard lane: heavy abandonment and lost value across both standard and priority classes](images/launch-day-priority-only-reserved-seed-2.png)

![Demand fate and retained value by urgency class under launch-day load with the recommended both-dynamic mechanism: most demand included and most value retained](images/launch-day-both-dynamic-strict-threshold-seed-2.png)

Finally, we stress-tested the controller calibration and byte-only threshold design along the parameter axis as well as the load axis: a sweep of target utilisation {0.25, 0.5, 0.75} × max-change denominator {4, 8, 16}, applied in lockstep to both controllers, ten seeds, under low, severe-congestion, launch-day, and EB-capacity-stress loads. At the tested grid points with target utilisation 0.5 or 0.75 and denominator 8 or 16, the observed comparisons were generally favourable or near-baseline. The strength of the evidence differs by load. Launch-day was paired against flat fee, with intervals that exclude zero. Severe and EB-stress were paired against the anchor calibration. Low load was unpaired and within about a point of the flat-fee aggregate. At target 0.5 the advantage holds at every contended load. At 0.75 the EB-stressing result is 31.4% urgent-class retained value against flat fee's 30.1%, without an equivalence margin. At the tested target utilisation of 0.25 the mechanism retains less value than flat fee under launch-day load, and at denominator 4 price stability degrades at every load. This sweep moved both targets together; the later independent standard-lane target experiment, which holds the urgent target at 0.5 and moves only the standard target, is described under [Independent standard-lane target screen and confirmation](#independent-standard-lane-target-screen-and-confirmation). We also tested and rejected a cross-lane multiplier floor (a rule that holds the urgent quote at or above a fixed multiple of the standard quote). It overprices the urgent lane precisely when capacity is slack, at a cost of 9-15 percentage points of urgent-class retained value at low load. A demand-elasticity stress test (all values scaled 10×, 10-25% of arrivals at 100× values, each mix against its own flat-fee control) preserves the advantage at every mix. Under launch-day load the advantage grows with the share of high-value demand, while under severe congestion it stays roughly constant across mixes. The byte-threshold fraction and the simulated announcement age escape are direct products of these tests. The application of that fraction to the other `SDPolicy` resource dimensions is a specification completion rather than an experimentally validated result. Protocol enforcement is defined in the Endorser Block announcement threshold section.

#### D16/K10 headline rerun

<!-- PORTABILITY: the experiment report link below is relative; replace with a commit-pinned permalink (same form as the link in the next paragraph) before the CIPs-repo PR -->

We separated the simulator's fresh-demand, ranking-block-production, and retry-jitter random streams. We then reran the canonical byte-only D16/K10 simulator configuration against flat fee over the five headline loads (paired seeds 0–9, 2,000 slots each). The rerun was successful. Low load showed no statistically detectable urgent-class difference (+0.47 percentage points retained value, 95% CI [-1.16, +2.09]). Mid, severe-congestion, and EB-capacity-stress loads improved by +3.69, +7.29, and +8.57 percentage points respectively, with all ten seeds better in each case. Launch-day overall retained value improved by +8.15 percentage points (95% CI [+6.11, +10.19], ten of ten seeds). The recommendation is unchanged within the tested scope. Full results are in the [experiment report](../preliminary-experiment-report.md#d16k10-headline-rerun), and the [preserved headline record](../experiment-results/canonical-headlines.json) retains every table-driving per-seed scalar plus the raw-output, effective-input, executable, and comparison-time source hashes.

#### Thousand-seed replication at low and severe-congestion load

The ten-seed headlines bound seed-sampling error loosely, so we replicated the flat-fee versus canonical byte-only D16/K10 pairing at 1,000 paired seeds (0-999, 2,000 slots, summary-only, independent random streams) under the two extremes of the load axis: low and severe congestion. The other three loads remain ten-seed.

Under severe congestion the replication confirms and sharpens the headline. Urgent-class retained value improves from 43.56% to 50.72%, a paired difference of +7.16 percentage points (95% CI [+7.06, +7.26]), with improvement in all 1,000 seeds. Urgent-class mean latency falls from 2.97 to 2.51 blocks, again in every seed. Overall retained value rises by +0.38 percentage points (95% CI [+0.37, +0.39]).

The gain at this load is larger at the tail of the wait distribution than at its centre. The urgent class's 95th-percentile wait falls from 5.74 to 4.79 blocks (-0.95, 95% CI [-1.00, -0.90], longer in 2 of 1,000 seeds), while its median wait falls from 2.45 to 2.00 blocks. The distance between the median and the 95th percentile therefore narrows from 3.30 to 2.79 blocks (-0.50, 95% CI [-0.56, -0.44], wider in 119 seeds). The mechanism's median is 2.00 blocks in almost every seed, so part of that narrowing is a median that has stopped moving. The 95th percentile fell by more than the median did, so the narrowing is not only that effect.

Measured in slots rather than blocks, the same distribution shifts earlier by a comparable margin (95th percentile -7.52 slots, 95% CI [-7.91, -7.14]) but does not narrow: the distance between the median and the 95th percentile is 63.09 slots under flat fee and 63.99 slots under the mechanism. Block position is the quantity this mechanism decides. The interval between Ranking Blocks is exogenous and has the same distribution under both variants, so it contributes dispersion to both arms that no fee rule can remove. We therefore report the block measure as the predictability result. The slot spread does not improve: it widens by 0.90 slots (95% CI [+0.54, +1.26], wider in 619 of 1,000 seeds).

These latency distributions describe populations that differ at the entry margin. Urgent-class submissions fall by about 9.8% under the mechanism at this load, from a mean 3,439 to 3,103 per run, while the urgent-class service rate rises from 98.20% to 98.42%. The experiment report measures that entry effect directly and bounds its contribution to the retained-value headline.

At low load the interval narrows by an order of magnitude. The urgent-class retained-value difference is -0.03 percentage points with a 95% CI of [-0.15, +0.10], so the 95% interval bounds any effect on urgent-class retention at this load within roughly ±0.15 percentage points. Urgent-class service rate is likewise indistinguishable, and urgent-class mean latency is marginally lower (-0.29 slots, 95% CI [-0.47, -0.11]). The urgent class's wait distribution nonetheless tightens in block terms here too: its 95th-percentile wait falls from 2.91 to 2.53 blocks (-0.38, 95% CI [-0.42, -0.34]), and the distance between its median and its 95th percentile falls from 0.96 to 0.54 blocks (-0.41, 95% CI [-0.45, -0.37], wider in 40 of 1,000 seeds). Measured in slots that distance does not move (+0.64, 95% CI [-0.06, +1.34]). These are demand-class metrics: they track the fastest-decaying demand across both variants, whichever lane it uses, not the set of transactions that pay the urgent quote. The mechanism records about 7.6% more urgent-class submissions than flat fee, with the retention and service-rate differences inside the reported intervals. That is consistent with a reserved lane that makes entry attractive to marginal demand, but this comparison does not isolate that channel from the rest of the mechanism.

The larger sample also resolves a cost invisible at ten seeds: overall retained value at low load sits 0.40 percentage points below flat fee (95% CI [-0.41, -0.39], flat fee better in 982 of 1,000 seeds, where ratios count value whose fate resolved within the horizon). The urgent demand class's measured retention and service-rate differences sit within the bounds above, and its mean latency is slightly lower. Attribution of the 0.40 is less clean than a per-lane split, because the mechanism also changes lane choice. Under flat fee effectively all transactions travel the standard path (a mean 5,945 per run). Under the mechanism most of that demand selects the urgent lane instead, leaving 2,139. The transactions that stay standard wait a mean of 57.96 slots against 34.25 under flat fee (2.99 against 1.78 blocks) while they pool for Endorser Blocks worth their certificate. Their tail lengthens by proportionally more than their mean: the 95th-percentile standard wait rises from 2.96 to 5.73 blocks (+2.77, 95% CI [+2.69, +2.85]), and from 87.93 to 113.92 slots. Under severe congestion every standard-lane mean and quantile difference is at or near zero, so pooling for worthwhile Endorser Blocks costs standard traffic nothing once Endorser Blocks fill on their own. Yet the retained ratio among them is marginally higher than flat fee's (+0.68 percentage points, higher in 910 of 1,000 seeds). The 0.40 is therefore the net of longer standard-lane waits and the shifted lane composition, not a retention loss inside either lane taken alone. It prices the low-load trade in the byte-only simulator realisation. That simulated mechanism costs 0.40 ± 0.01 percentage points of overall retained value against flat fee at this load, in exchange for the +7.16-point urgent-class improvement under severe congestion. (The pairing does not isolate the announcement threshold's own contribution from the reservation rule's.)

A dedicated hundred-seed attribution rerun (paired seeds 0-99, same configuration, per-lane value levels preserved) decomposes this cost. It reproduces the overall difference at -0.399 percentage points (95% CI [-0.443, -0.356]). The deficit does not come from lost entry. The mechanism submits marginally more units and more value than flat fee, in all 100 seeds. The per-lane value levels sum exactly to the overall totals in every seed. The difference therefore arises among submitted transactions. There are two accountings, kept separate. The -0.399 ratio counts only value whose fate resolved, and its driver is more value that decays before inclusion (+39.6M lovelace lost). In absolute terms the mechanism also carries more value still unresolved at the horizon (+38.8M), which the ratio excludes. The lane split at this load: about 71% of submitted value selects the priority lane and keeps the flat-fee latency profile (mean 35.0 slots against flat fee's 35.2), while the value that stays standard waits 59.1 slots. Both per-lane retained ratios are composition-shifted. The slower-decaying value stays standard, so its ratio rises even as its wait lengthens. The faster-decaying value migrates to the urgent lane, whose ratio sits below flat fee's overall figure despite equal latency. The [attribution record](./low-load-attribution-smoke.json) preserves the per-seed values for every metric, the derived residual check, and provenance hashes. `scripts/smoke_low_load_attribution.sh` reproduces it.

The [preserved evidence record](./thousand-seed-low-severe.json) holds the per-seed values for every metric above, the paired statistics, provenance hashes, and the reproduction commands (`scripts/compare_thousand_seed.py` regenerates the record from the sweep outputs).

#### Independent standard-lane target screen and confirmation

The parameter stress test above retunes both controllers in lockstep. This experiment moves only the standard-lane target, holding the urgent target at 0.5 and every other input fixed, to ask whether the two lanes want different equilibrium loads.

An exploratory screen compared standard targets 0.625, 0.75, and 0.875, a fixed-standard arm (a variant with no standard controller), the canonical 0.5/0.5 calibration, and flat fee over 100 paired seeds (0-99) and 2,000 slots, with independent random streams, under severe-congestion and launch-day loads. A patient demand-census arm supplied each seed's common offered-demand denominators. A full rerun of the screen on seeds 100-199 reproduced every arm ordering. In both seed ranges, targets 0.625 and 0.75 retained more overall value than the canonical calibration on both loads, while 0.875 and the fixed-standard arm gave up most of the launch-day gain. We selected 0.75, the arm with the largest minimum improvement over canonical across the two loads, and reran only that arm against flat fee and canonical on the disjoint seed range 200-299.

The confirmation reproduces the screen. All figures below are paired over the 100 confirmation seeds with 95% confidence intervals, in modelled gross value (mean offered value 456 G lovelace under severe congestion, 1,357 G under launch day). Under severe congestion the 0.75 standard target retains +6.84 G [+5.86, +7.82] more than canonical, and unlike canonical it also beats flat fee (+0.94 G [+0.87, +1.02] against canonical's -5.90 G [-6.90, -4.89]); urgent-class retained value is higher than canonical's (+0.144 G [+0.127, +0.163]) at a wait cost of +0.06 blocks mean and +0.23 blocks p95. Under launch day it retains +31.13 G [+23.65, +38.60] more than canonical. At low and mid load the 0.75 and 0.5 configurations are bit-identical over all 100 seeds and every reported scalar: the standard coefficient rests at its 1.0 floor under either target, so the change is inert there.

EB-capacity stress prices the change. Lower standard quotes draw 29% more urgent-class submissions (3,889 to 5,015 per run), so relative to canonical the urgent-class service rate falls by 4.1 percentage points, mean wait rises +0.35 blocks, and p95 wait rises +1.05 blocks, while urgent-class retained value still rises (+0.094 G [+0.066, +0.121]) and overall retained value rises (+9.28 G [+7.05, +11.50]). Relative to flat fee every urgent-class comparison remains an improvement: +0.327 G [+0.311, +0.343] retained, -0.39 blocks mean wait, -0.62 blocks p95 wait. At this load the 0.75 target converts part of the 0.5 calibration's wait margin into admitted demand rather than surrendering any value.

The screen and confirmation manifests and runners are [standard-target-screen.json](../../../abstract-sim-hs/config/sweeps/standard-target-screen.json), [standard-target-confirm.json](../../../abstract-sim-hs/config/sweeps/standard-target-confirm.json), [run_standard_target_screen.sh](../../../abstract-sim-hs/scripts/run_standard_target_screen.sh), and [run_standard_target_confirm.sh](../../../abstract-sim-hs/scripts/run_standard_target_confirm.sh); `scripts/compare_standard_target_screen.py` generates the paired comparison reports from the sweep outputs.

#### Window-removal experiment

The recommended signals smooth over windows: 20 block summaries for the standard controller and 5 payload samples for the urgent controller. This experiment asks whether the windows are necessary at all. It replaces each window with an instantaneous signal that reads only the current block-production event. All arms use the recommended calibration (standard target 0.75, urgent target 0.5, D = 16, the announcement threshold, and the K = 10 age escape).

A four-arm pilot (paired seeds 300-304, five loads) selected the two effects to confirm: the launch-day throughput loss without the standard window, and the severe-congestion increase in quote movement without the urgent window. We recorded hashes of the pilot report and the analysis plan before any confirmation seed ran. The confirmation then compared three arms on fresh seeds 400-409. Under the plan, an effect is confirmed only if it passes two tests. Its 97.5% confidence interval must exclude zero. The stricter level (97.5% in place of the usual 95%) keeps the total chance of a lucky pass across the two effects at 5%. The effect must also point in the expected direction in at least nine of the ten seed pairs. Both effects passed both tests, in ten of ten seeds:

- **Instantaneous standard signal.** Launch-day throughput decreases by 18.69 tx/slot (97.5% CI [-22.28, -15.11]) against the windowed design. Overall retained value decreases by 121.3 G lovelace (95% CI [-139.1, -103.5]). The retained ratio decreases from 68.8% to 56.3%. The cause is structural. Certified Endorser Blocks arrive only every ten to twenty blocks. A signal that reads one block-production event at a time sees mostly payload-free Ranking Blocks. It therefore holds the standard quote at its floor, and it admits demand that the lane cannot serve (757k versus 658k submissions). At low and mid load the two signals give bit-identical results, because the standard coefficient rests at its floor under both.
- **Instantaneous urgent signal.** Quote movement increases at every load. Under severe congestion, the excess log-coefficient travel increases from 0.53 to 3.49 (+2.96, 97.5% CI [+2.50, +3.41]). The increase is comparable at the other loads, from +1.56 at low load to +3.09 at launch day. Retained value and latency show no meaningful change. Under severe congestion, urgent-class retained value is marginally higher without the window (+0.013 G [+0.001, +0.025]). The benefit of the urgent window is therefore quote stability, not retained value.

The result supports both windows: the standard window for service during demand surges, and the urgent window for quote stability. It does not show that the window lengths 20 and 5 are optimal. Manifests and runners: [window-ablation-smoke.json](../../../abstract-sim-hs/config/sweeps/window-ablation-smoke.json), [window-ablation-confirm.json](../../../abstract-sim-hs/config/sweeps/window-ablation-confirm.json), [run_window_ablation_smoke.sh](../../../abstract-sim-hs/scripts/run_window_ablation_smoke.sh), and [run_window_ablation_confirm.sh](../../../abstract-sim-hs/scripts/run_window_ablation_confirm.sh). `scripts/compare_window_ablation_confirm.py` regenerates the confirmation report and verifies the pinned analysis plan.

<!-- PORTABILITY: blob/main link; replace with a commit-pinned permalink before the CIPs-repo PR -->

Full details, including method, configs, per-load tables, paired seed deltas, and figures: [preliminary experiment report](https://github.com/input-output-hk/tiered-pricing/blob/main/docs/phase-2/preliminary-experiment-report.md).

### Prototype

https://github.com/user-attachments/assets/6a4ef69a-516f-4517-bfbd-d7b8a97b09cf

The two lanes running on the devnet (8 min 50 s): a walkthrough of the mechanism, followed by sustained demand, quote increases and transaction evictions, certification pressure and recovery, and the return to an idle network. Captions are included.

The mechanism has been implemented end to end in a prototype based on the Linear Leios prototype node. The implementation covers four integration boundaries:

- ledger validation, controller updates, fee settlement, and refunds;
- consensus mempool admission, selection, and revalidation;
- transaction construction and submission; and
- Endorser Block announcement, voting, certification, and tracing.

The prototype runs a three-node Dijkstra devnet with a live dashboard and generated senders that select lanes using the published quotes. It therefore complements the discrete-event simulator with implementation evidence from a running multi-node protocol. The code, launcher, change sets, design notes, and the mapping between the prototype terminology and this CIP are available in the [prototype repository](https://github.com/nhenin/dynamic-pricing).

The prototype exercises the following transaction lifecycle:

- A Ranking Block containing a transaction whose fee cap does not cover the applicable urgent quote fails ledger validation with `BidBelowQuote`.
- Controller updates are executed as part of block processing. A certified Endorser Block contributes its payload to the utilisation signals exactly once, when it is certified.
- At settlement, the posted fee cap is decomposed as:

  $$ \mathrm{postedFeeCap} = \mathrm{baseFee} + \mathrm{premium} + \mathrm{refund} $$

  The base fee enters the fee pot, the premium enters the treasury through the donation pot, and the excess is credited to the refund account specified by the transaction.
- Node policy admits transactions using the one-step fee-cap buffer and revalidates them as quotes move. A transaction is evicted when its fee cap no longer satisfies the applicable admission requirement.
- Endorser Block announcement is gated by the byte threshold—45,056 bytes with the tested configuration—and by the `K = 10` age escape. Below the threshold, standard transactions remain pooled until additional transactions arrive or the age escape opens.
- All three nodes participate in the Linear Leios voting and certification pipeline. The certification-miss scenario suppresses votes at their source rather than fabricating a certificate or ledger outcome.

The tested configuration uses a target utilisation of `0.5`, a max-change denominator of `16`, a five-sample urgent window, a twenty-block standard window, the urgent lane's initial `2×` coefficient, the announcement threshold, and the `K = 10` age escape. Utilisation is computed independently for bytes and execution units, using the larger ratio.

Ledger validity requires the posted fee cap to cover the applicable quote at inclusion. Admission and producer selection additionally apply the one-step node-policy buffer described above. For an urgent transaction, this policy considers both possible delivery paths. If the transaction is delivered through a certified Endorser Block, it is charged the standard quote and the excess is refunded.

The prototype FIFO-merges the urgent remainder with the standard lane when constructing an Endorser Block. A certificate-carrying Ranking Block contains no separate transaction payload: its unresolved transaction sequence is empty, and block resolution applies only the certified Endorser Block payload. This gives every transaction in the resolved block the same certified-delivery semantics.

The most difficult integration issue was preserving a single transaction lifecycle while an Endorser Block was in flight. Announced transactions must leave the selectable mempool to prevent duplicate inclusion. Transactions from superseded uncertified payloads must be readmitted without violating dependencies, while certified transactions must be applied in dependency order. The prototype includes targeted tests and live traces for these behaviours.

The prototype provides implementation evidence, but it is not production-ready. Production integration would additionally require governed protocol parameters, era-versioned serialization, complete dependency and governance-action handling, adversarial mempool and DoS analysis, wallet support, restart and rollback testing, fork-convergence testing, and shared conformance tests across the formal model, simulator, ledger, consensus, and wallet implementations.

### Why not full tiered pricing?

We initially planned a mechanism based on the paper [Tiered Mechanisms for Blockchain Transaction Fees by Kiayias et al](https://arxiv.org/pdf/2304.06014) as the subject of this CIP. After discussion with stakeholders and investigation into the technical requirements, we decided that a reduced-complexity version is adequate for community needs. A simpler version is also easier to prove, is less likely to cause regression, and ships sooner. Earlier delivery can offset any value-retention differential anyway.

The discarded tiered mechanism involved n tiers. Some designs used n tiers for each block type, and others used n tiers across block types. In both cases, each tier was independently and dynamically priced, and each tier carried an artificial delay: a transaction assigned to that tier had to wait out the delay before it became eligible for inclusion in a block. The most fundamental reason against the paper's model was that linear-Leios' structure supports multiple lanes naturally (one lane per block type), without manufactured delays. Beyond that, the main challenges we encountered were:

* The paper specifies delay eligibility abstractly: a transaction assigned to a tier is ignored until that tier's delay elapses. It does not specify how to implement that rule in Cardano or linear-Leios. In our attempted mapping, enforcement appeared to require ledger machinery to anchor and track the delay, define admission and validation of waiting transactions, handle dependencies and rollbacks, and bound the associated mempool and DoS exposure. It also mapped awkwardly onto linear-Leios: EB-bound transactions already follow a higher-latency path, so it is unclear whether they must incur the same additional artificial delay.
* Division of block space by tier is complex even without linear-Leios, because of Cardano's multi-dimensional fee and fullness model (bytes and execution units)
* The paper did not handle UX, retry, rejection, or mempool overflow, which added more complexity to the design process
* A security-adjacent concern: more tiers reveal a transaction's urgency, and thus potentially its purpose, more precisely, which increases the surface for front-running

The two-lane mechanism specified here is a first increment, not a ceiling: if evidence emerges that finer-grained tiers retain meaningfully more value, a successor CIP can extend it.

### Optional extensions

#### Tipping

The urgency signal stops discriminating once urgent demand itself exceeds Ranking Block capacity: every RB candidate already pays the urgent quote, so the flag can no longer separate them. In this case, users could use [nested transactions](https://github.com/cardano-foundation/CIPs/pull/862) to offer the block producer a tip that buys priority within the urgent lane. The nested-transactions proposal does not itself specify producer tips or selection priority. Specifying a tipping mechanism is outside the scope of this CIP; this section only points out that one is possible using existing proposals.

With the nested transactions [CIP-0118](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0118) implemented, any user can create an incomplete transaction whose `produced` value 
is less than its `consumed` value. The funds
that make up the difference can be directed to any user running a nested transaction aggregator. The aggregator 
can then construct a complete transaction `tx'` containing `tx` directing the funds difference to the aggregator's
address. This schema for paying an agreggator and thereby incentivising them to include your
incomplete transaction in the next block they produce is how tipping is expected to work.  

The schema works regardless of which aggregator receives `tx`, but only the aggregator who successfully submits 
a block that contains `tx'` (and gets included in the chain) will receive the "tip". This way, the transaction author 
can tip whoever's completed transaction makes it on-chain, likely the same user that is running the aggregator alongside 
a node whose turn it happened to be to produce the block. Note here that the aggregator is not yet a standardized 
piece of software, however, we assume that it would be capable of identifying incomplete transactions that are 
offering a tip. The criteria would be as follows: any sub-transaction that (1) consumes more ada than it produces, 
and (2) does not require a top-level script. As an alternative to (2), specific analysis of required top-level scripts
may determine that the sub-transaction is offering a tip *conditionally*, but investigating the details of this
approach is outside the scope of this CIP.

## Path to Active

### Acceptance Criteria

The proposal is active when all of the following are true:

- [ ] CIP-164 (linear-Leios) is active on mainnet, or one hard fork activates both CIP-164 and this mechanism.
- [ ] The fee change CIP, which specifies the refund mechanism, is merged and active.
- [ ] The formal ledger specification of the new rules is complete and merged upstream. The rules are: RB lane eligibility, fee settlement, the per-lane quote update, and the EB certificate threshold with the age escape.
- [ ] The Agda mempool specifications for Leios with urgency signalling are complete and merged upstream.
- [ ] Simulation or testnet evidence covers the reference-script and execution-unit branches of the EB threshold rule.
- [ ] A peer-reviewed node implementation of the mempool changes, the block production policy, and the ledger rules is available.
- [ ] An implementation-independent conformance test suite covers the new rules, and the node implementation passes it.
- [ ] Load tests on a public testnet replay a published synthetic workload with assigned value-decay profiles against a flat-fee comparator. The metrics and pass margins are declared before the tests run, and they include both a central measure and a 95th-percentile measure of urgent-class inclusion delay in blocks. Under congestion, urgent-class results improve by at least the declared margins. At low and trickle load, standard-class and overall results stay within the declared regression bounds, and those bounds cover the 95th-percentile standard-class delay as well as its mean.
- [ ] An independent audit of the specification, the implementation, and the incentives analysis is published. Material findings from the audit are resolved.
- [ ] Wallets and transaction-construction libraries support the new transaction fields: lane choice, maximum fee, and refund account.
- [ ] The community agrees on the initial values of the protocol parameters listed in "The recommended construction".
- [ ] A hard fork enables the mechanism on mainnet.

### Implementation Plan

The work items, in approximate order:

- [ ] Merge the ledger and mempool specifications into their upstream repositories.
- [ ] Extend the experimental evidence to the reference-script and execution-unit branches of the EB threshold.
- [ ] Write a node-level technical specification, coordinated with the CIP-164 node work.
- [ ] Implement the changes in `cardano-node`, integrated with the linear-Leios implementation.
- [ ] Develop implementation-independent conformance tests, aligned with the CIP-164 blueprint work.
- [ ] Update wallets and libraries for the new CDDL fields.
- [ ] Publish the synthetic testnet workload, the metrics, and the pass margins. Then run load tests on a Leios testnet, and compare the results with the declared margins.
- [ ] Audit the specification, the implementation, and the incentives analysis.
- [ ] Schedule the hard-fork activation with, or after, the linear-Leios rollout.

## Versioning

<!-- PORTABILITY: CIP-84 link becomes the repo-relative ../CIP-0084 once inside the CIPs repo -->

Transaction urgency signalling changes the rules by which transactions are admitted to Ranking Blocks under linear-Leios. Where this affects ledger validation, transaction format, fee calculation, or block validity, it requires a new major protocol version and a new ledger era, and [CIP-84](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0084) applies.

A hard-fork event enables the mechanism, either as part of the linear-Leios hard fork or in a later hard fork. Incompatible changes require a successor CIP and a subsequent protocol version.

<!-- PORTABILITY: the fee change CIP link below points at a fork branch; repoint at its CIPs-repo PR (or CIP number) once one exists -->

This CIP also depends on [the fee change CIP](https://github.com/polinavino/CIPs/tree/fee-change/CIP-%3F%3F%3F%3F).

### Relationship to CIP-183: conflict-based fee priority

[CIP-183](https://github.com/cardano-foundation/CIPs/pull/1178) proposes a node-local replacement policy for transactions that conflict over spent or reference UTxOs. This CIP addresses a different scarcity: timely block inclusion under linear-Leios. CIP-183 affects mutually conflicting transactions while leaving uncontested traffic under FIFO; this CIP assigns transactions to standard or urgent delivery paths and adjusts protocol quotes from lane utilisation. CIP-183 therefore does not provide a general urgency signal, while this CIP does not auction access to contested UTxOs.

The proposals could coexist in principle, but their current specifications cannot simply be layered. CIP-183 compares a fee that the winning transaction pays. Here, the posted fee is a maximum authorisation whose excess over the inclusion quote is refunded; using that maximum as a replacement bid would make refundable headroom determine conflict priority. The reference mempool policy in this CIP also rejects an incoming urgent transaction when admitting it would evict a conflicting standard transaction, whereas such eviction is CIP-183's defining operation.

A combined design would need to define an economically binding contention bid or other comparison metric, cross-lane replacement rules, descendant handling, fee settlement, and the point at which a transaction committed to an Endorser Block can no longer be replaced. CIP-183 is therefore neither a dependency nor an alternative to this proposal. It is related work on a separate fee-market dimension, and the experimental and prototype evidence in this CIP does not evaluate the combined mechanism.

### If the linear-Leios mechanism changes

If a future linear-Leios revision permits a certificate-bearing RB to also carry a transaction payload, the effective capacity for urgent-lane transactions increases.

Under this CIP, only urgent-paying transactions can enter a direct RB payload. Because of this rule, the change can only increase urgent-lane throughput. If the added capacity is not used, urgent-lane throughput does not change.

The change also decreases the opportunity cost of EB certification. As a result, a revision can relax or remove the announcement threshold and the age escape.

## Copyright

This CIP is licensed under [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/legalcode).
