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

## Abstract

We propose two lanes by which a user can submit a transaction to a node: urgent and standard. Only urgent transactions can enter Ranking Blocks. Both urgent and standard transactions can enter Endorser Blocks. Nodes produce Ranking Blocks more frequently than Endorser Blocks, and a Ranking Block enters the chain immediately. When capacity and queue order permit, an urgent transaction can therefore enter an earlier Ranking Block instead of the later Endorser Block path. This creates an earlier inclusion opportunity, not a guarantee of earlier inclusion.

The ledger enforces the urgency signalling rule: every transaction in a valid Ranking Block must carry a fee that covers the urgent quote for that block. In simulation under severe congestion, the mechanism preserves more urgent-class transaction value than today's flat fee. Retained value means the modelled gross transaction value that remains at inclusion, before fees. Urgent-class retained value improved across most simulated loads. At light load, the mechanism slightly reduces overall retained value, because transactions on the standard path wait longer while Endorser Blocks fill. The Rationale gives exact figures.

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

<br>

**Standard transaction**: A transaction that does not pay to enter the urgent lane. Cardano's current transactions.

**Urgent transaction**: A transaction that pays to enter the urgent lane. The payment asks nodes to include the transaction before standard transactions, where possible.

**Reserved**: An urgent-lane mechanism that reserves RB block space for urgent transactions. The ledger enforces the reservation.

#### Lanes and routing

**Standard lane**: A pathway for transactions that do not pay the urgent fee.

**Urgent lane**: A pathway for transactions that pay the urgent fee.

**Lane selection (the user-side decision)**: The choice of lane, made by the constructor of a transaction.


#### Pricing primitives

**Pricing coefficient**: The value that multiplies the base fee to produce the quote. Also called *tier coefficient*.

**Quote**: The pricing coefficient multiplied by the base fee. In effect, a snapshot of the dynamic fee for a given transaction.

**Urgent premium**: The difference between the urgent lane quote and the standard lane quote.

**Absolute coefficient floor**: The minimum allowed lane pricing coefficient, set to `1.0`: no quote can fall below the ordinary Cardano minimum fee.

**Fixed (pricing)**: Basic Cardano fee, as today.

**Dynamic (pricing)**: EIP-1559 style dynamic fee.

**EIP-1559 (controller)**: The feedback mechanism that adjusts a lane's pricing coefficient after each block: up when utilisation is above target, down when below, by a bounded step.

**Max-change denominator (D)**: The scale in the controller update. Before the coefficient floor, the largest downward step is `1/D`. The largest upward step is `(1 - targetUtilisation) / (targetUtilisation × D)`. At target 0.5 these are equal, so the price rises and falls at the same maximum rate. Below 0.5 the price can rise faster than it falls. Above 0.5 it rises more slowly.

**Signal window**: The number of recent blocks over which the controller measures utilisation, so a single unusual block cannot swing the price.

**Target utilisation**: The block fill level the controller steers towards (0.5 in the default configuration). Utilisation above the target raises the price. Utilisation below it lowers the price.

**Quote drift**: The difference between the quote at submission time and the quote at inclusion time.


#### User-side fee fields

**Posted fee vs actual fee**: The posted fee is the amount attached to the transaction at submission. The actual fee is the quote at inclusion time. The ledger refunds the difference.

**Refund**: The return of the excess fee to a specified address.

**Max fee (max_fee_lovelace / fee ceiling on the user side)**: The most a user agrees to pay, posted with the transaction. It buffers against quote drift. If the quote exceeds it, the transaction cannot be included.


#### Value / actors

**Urgency**: The rate at which the value of a transaction decays.

**Urgent demand class**: The fastest-decaying demand class in the model. It is independent of the lane a transaction selects.

**Retained value metric**: The numerator is the sum of modelled delay-discounted gross transaction value remaining at inclusion, before fees. It is not the simulator's fee-subtracted utility measure, and it is not an observed economic quantity. Unless a table states a different denominator, a retained-value ratio is `retained / (retained + lost)`. The ratio excludes value still unresolved at the simulation horizon.
</details>

<br>

### The recommended construction

The settled recommendation in one place. Each component is specified in detail in the sections that follow, except the controller update rule and signals, which are defined immediately below the table.

| Component | Specification |
|---|---|
| Lanes | Two: standard and urgent |
| Ranking Blocks | Urgent-only at all loads (ledger-enforced). FIFO selection over the urgent view |
| Endorser Blocks | Open to both lanes. FIFO selection over the canonical queue |
| EB announcement threshold | Let `thresholdFraction = max(1 - urgentTargetUtilisation, 1/2)`. Unless the age escape applies, a non-empty certified EB qualifies when its serialised transaction size or reference-script size reaches that fraction of the corresponding RB limit, or when both components of its execution-unit budget do. The default fraction is 1/2 (45,056 B for the simulated transaction-size component) |
| EB announcement age escape | A certificate for a non-empty EB below the threshold can enter an RB once at least K = 10 Ranking Blocks have been produced since the last certified EB |
| Fee semantics | Per-lane EIP-1559: each lane's quote is its pricing coefficient × the ordinary min fee |
| Fee-cap basis | For an urgent transaction under rb-only settlement, wallet choice and every max-fee validity check use max(standard quote, urgent quote). Temporary quote crossings are permitted and do not alter either controller |
| Premium scope | rb-only: the applicable inclusion quote is the urgent quote in an RB and the standard quote in an EB |
| Admission, revalidation, and selection (node policy) | Admission requires the posted max fee to cover the maximum applicable lane quote after one conservative lane-specific controller step. While the transaction is queued, the max fee must cover the current fee-cap quote, or the node evicts the transaction. Prudent EB selection takes a transaction only if the max fee also covers one further lane-specific step (RB selection needs only the current quote, since inclusion is immediate) |
| Settlement and refund | Inclusion charges the applicable inclusion quote. The ordinary min-fee component goes to the fee pot, the premium above it goes to the treasury, and the posted excess goes back to the refund account. A posted maximum below the applicable quote is invalid |
| Standard controller | Target utilisation 0.5, max-change denominator 16, capacity-weighted utilisation over a 20-block window, initial coefficient 1.0 |
| Urgent controller | Target utilisation 0.5, max-change denominator 16, reservation utilisation over a 5-sample window, initial coefficient 2.0 |
| Floors | Absolute coefficient floor 1.0 (no quote below the ordinary min fee). No cross-lane multiplier floor |
| Enforcement boundary | Ledger rules enforce RB lane eligibility, inclusion-point fee validity, settlement, and the deterministic per-lane quote update. The EB threshold and the age escape are ledger rules checked at certificate inclusion, specified in the Endorser Block announcement threshold section. Wallet choice, the urgent queue view, FIFO construction, admission headroom, revalidation, eviction, and producer headroom are node policy |

The canonical simulator configuration for the experiments is [`thr-k10.json`](./thr-k10.json), a copy of `abstract-sim-hs/config/variants/trickle-aging/thr-k10.json` from the tiered-pricing repository. Its embedded load is only the simulator's default workload, and experiment manifests override it. It is not part of the mechanism recommendation. The simulator implements only the byte component of the multidimensional EB threshold specified above. The max-of-two fee-cap rule is the simulator's rb-only fee semantics rather than a configurable alternative.

The parameter values, the grid points tested, and the loads at which each was stressed are tabulated in the "Endorser Block announcement threshold" section.

#### Controller updates and signals

Both controllers update independently once per slot in which a block is produced. Each controller applies the following rule using its lane's utilisation signal, target utilisation, and max-change denominator:

```
coeff' = max(1.0, coeff × max(0, 1 + (utilisation - target) / (target × D)))
```

The controller clamps utilisation to [0, 1] before the update. The outer `max` applies the absolute coefficient floor. The urgent and standard utilisation signals are defined separately below. At the recommended target of 0.5 and D = 16, every step stays within ±6.25%.

There are four block production kinds: non-certificate Ranking Blocks, certificate-carrying Ranking Blocks, Endorser Block announcements, and certified Endorser Blocks. Two of the four carry a controller sample: non-certificate Ranking Blocks and certified Endorser Blocks. A certificate-carrying RB is payload-free by construction, and an EB announcement carries no sample. An EB's payload enters the signals exactly once, at certification.

**Urgent signal (reservation utilisation, 5-sample window).** Each sample measures the urgent lane's usage in the sampled block against the RB's capacity. A certified EB's sample uses the same reservation capacity as its denominator, not the EB's own capacity: the sample asks how many Ranking Blocks' worth of urgent traffic the EB carried, not how full the EB was. The window utilisation is the sum of urgent usage over the last five samples (each capped at the reservation capacity) divided by the sum of the reservation capacities. The controller computes this ratio separately in bytes and ex-units and takes the larger one.

**Standard signal (capacity-weighted utilisation, 20-block window).** The window utilisation is the total standard-lane usage across the last twenty block summaries divided by the total capacity of those blocks. The controller again computes this ratio separately in bytes and ex-units and takes the larger one. Certificate-carrying RBs and EB announcements contribute neither usage nor capacity. Non-certificate RBs contribute their full capacity to the denominator even when empty, though standard transactions cannot occupy them. The capacity weighting is implicit in the sums: each block counts in proportion to its capacity. At the capacities used throughout the experiments, a certified EB (12,000,000 bytes) outweighs a Ranking Block (90,112 bytes) by two orders of magnitude, so the standard quote tracks Endorser Block fill.

The specification covers several areas:

### Mempool

Our priority signaling design includes changes to the consensus protocol, the Leios protocol specifically. 
For this reason we have 
[specified](https://github.com/IntersectMBO/ouroboros-consensus/compare/polina/mempool-spec?expand=1) in Agda both 
the Praos (`Mempool.lagda.md`), Leios, and Leios with priority signaling mempools. 
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
with inly some minor additional block-level bookkeeping. For epoch boundary blocks, a full ledger state recomputation 
for the incoming RB/EB must be performed. The function generating blocks from mempool content, `forgeBlock`, 
returns a pair `(RB, Maybe EB)`. The `RB` is sent across the network to be added to nodes' chan tips, whereas 
`EB` is sent across the network to be added to the nodes' mempools `heldEB` variable. 

#### Priority Signaling Mempool Specifications

The priority signaling mempool design, specified in `MempoolLeiosPricing.lagda.md`,
features two distinct ledger states in place of `updatedLedger`: 

  (1) `priorityUpdatedLedger`, corresponding to the application of all transactions in the `priorityTxs` queue 
  (transactions specifying the priority tier) to `ebLedger` or `ledger`, depending on if a valid EB has arrived
  (2) `standardUpdatedLedger`, corresponding to the application of all transactions in the `standardTxs` queue
  (transactions specifying the standard tier) to `priorityUpdatedLedger`

The mempool is able to request transactions of a specific tier from its peers. 
It first requests the priority tier transactions, 
and only when none are available, requests standard transactions. 

#### Transaction Reordering for the Leios with Priority Signaling Mempool 

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
Let `txs1` and `txs2` be lists of transactions, `tx : Tx`, `s : LedgerState`, 
and `e : LEnv`. Given that `txs1 ++ txs2`, 
`(tx :: txs1) ++ txs2`, and `tx :: txs2` are valid in `e, s`, then `(tx :: txs1) ++ txs2 == txs1 ++ (tx :: txs2)`
:::

Priority transactions that can be commuted to the front of the `standardTxs` queue (i.e. back of `priorityTxs` queue)
are limited. We refer to transactions that can have priority status as `SimpleTx`, and this constraint is as
follows :

```
record SimpleTx (t : Tx) : Type where
  field
    -- reason : certificates may overwrite each other
    noCerts : t .Tx.body .TxBody.txCerts ≡ []
    -- reason : withdrawals read exact reward balances, which subsequent writes may change
    noWdrls : t .Tx.body .TxBody.txWithdrawals ˢ ≡ᵉ ∅
    -- reason : governance proposals get recorded in an ordered list 
    noGovProps : t .Tx.body .TxBody.txGovProposals ≡ []
    -- reason : governance votes may be doing conflicting writes
    noGovVotes : t .Tx.body .TxBody.txGovVotes ≡ []
```

There is also a constraint on all standard transactions that must be imposed for the commutativity property to hold :

```
record SpendOnly (t : Tx) : Type where
  field
    valid   : t .Tx.isValid ≡ true
    noRefs  : t .Tx.body .TxBody.refInputs ≡ᵉ ∅
    collSub : collIns t ⊆ ins t
```

The reason for the `SpendOnly` constraint is that all standard transaction must spend all the inputs 
they read. Otherwise,
a regular transaction may reference an input a freshly inserted priority transaction spends, and this will not be 
observable using only the two validations (but it will cause validation failure when validating a block in 
regular order). Neither of these constraints need to be enforced at the ledger level. 

**Decision required** Transactions with `isValid` set to `false` should probably be admitted to the priority lane 
without paying priority prices to ensure Phase-2 validation work is always paid for. 

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

```
step_bound = max(1/D, (1 - targetUtilisation_l) / (targetUtilisation_l × D)), or 0 if lane l has no controller

standard transaction:  max fee ≥ quote_standard × (1 + step_bound_standard)
urgent transaction:    max fee ≥ max(quote_standard × (1 + step_bound_standard), quote_urgent × (1 + step_bound_urgent))
```

The node rejects a transaction that cannot survive even one price update at the door. The rejection is visible, and the user can cheaply resubmit with a larger buffer. The alternative is worse: an admitted transaction sits against the mempool cap until it goes stale.

At selection into an EB, a producer takes only transactions that remain valid through the one further price update that can fire before the certification check. This guarantees that a certified EB cannot fail fee validation. The producer re-checks against current prices because prices can rise while a transaction queues. This extra step applies only to EBs. RB inclusion is immediate: no price update can fire between selection and inclusion, so RB selection checks the current quote alone.

The node evicts an admitted transaction whose max fee is overtaken anyway. Eviction must be the outcome here. The transaction must not enter an invalid block, and a transaction that cannot be included wastes mempool space.

The ledger enforces none of this, since mempool state is not observable on-chain.

#### Dependencies and conflicts

A priority transaction may be in conflict with transactions in the standard queue (even if it satisfies 
the `SimpleTx` constraint).
Conflict is detected whenever a transaction `tx` is Phase-1 validated both at the end of the priority 
queue and the end of the standard queue, and one of those validations fails. Then, `tx` will not be 
admitted to the mempool because doing so requires evicting one or more standard transactions from the 
standard queue, which is outside the scope of the kind of priority signaling this CIP is meant to enable. 
A common cause of such conflict is that `tx` is spending the same UTxO entry as some transaction in the 
standard queue. 

#### Capacity, eviction, and DoS

Priority transactions get at least an RB's worth of space and ExUnits allocated to them in the 
mempool, and may be admitted to an EB when that space if full. The eviction process for 
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
and the `tier_coeff` positive integer. This tier coefficient 
is what the transaction expects its `minfee` will be multiplied by to obtain the amount 
of fee it has to pay to get into its specified tier. 

The `reward_account` is specifies the address of the account to which change is returned when a transaction 
specifies a `txfee` that is larger than necessary. 

```
transaction =
  [ transaction_body
  , transaction_witness_set
  , bool                     ; isValid  (producer-set)
  , auxiliary_data / nil
  , tier_no                  ; actualTier (producer-set)
  ]
```

The `tier_no` is also included by the producer in the transaction itself.
Validation fails if it differs from the `tier_no` inside the transaction body. However, it is not signed by 
anyone (similar to `isValid`). The purpose of this field is to allow the networking/consensus/DApps to 
have access to the tier without having to inspect the transaction body (e.g. the mempool will request 
only priority tier transactions first).

#### Ledger Rule Changes

We define an `SDPolicy` record containing four variables that are used in the following way :

  1. `diversityPolicy : TierNo ⇀ PolicyClause` - a set of tiers and their associated tier coefficients
  1. `totalSize : TierNo ⇀ ℕ` : the total size computed by adding up the size in bytes of all transactions in the list inside a block body, aggregated by tier
  1. `totalRefScriptSize` - the total size computed by adding up the size in bytes of all reference scripts and datums 
  referenced by all the transactions in the list inside a block body, aggregated by tier
  1. `totalExUnits : TierNo ⇀ ℕ` - the total amounts computed by adding up the size in bytes of all 
  execution units (memory and CPU, 
  separately) specified by all scripts in all the transactions in the list inside a block body, aggregated by tier

There is a new parameter `policyState : SDPolicy`  in the `UTxOState`.

Let `adjusted_tier_coeff` be `priority` if it was in an RB with a transaction list, and `standard` 
if it was in an EB. following are the key ledger rule changes having to do with processing the *fee payment* :

  1. updated min-fee constraint (enough to cover *targeted* tier) : `tier_coeff·minfee ≤ txFee`
  1. `txfee - minfee * adjusted_tier_coeff` is the amount of change sent to `reward_account` if it exists, 
  and to the treasury if it does not
  1. exactly `minfee` is sent to the fee pot
  1. `minfee * (adjusted_tier_coeff - 1)` is sent to the treasury

The following have to do with correct tier specification `poilcyState`, and the change given :

  1. Tier coefficient in `poilcyState` associated with the transaction body-specified 
  `tier_no` is `≤ tier_coeff` in the `tx` body
  1. The tier number in the body is `≤ adjusted_tier_coeff` and such that it is 
  `priority` if `tx` was in an RB with a transaction list, and `standard` if `tx` was in an EB
  1. `policyState` is updated to reflect the current aggregated values 2-4 to reflect `tx`
  1. the the change given (as calculated above) is sent to the specified account address

#### Block validity

This CIP relies on Leios block structure. For this reason, we change the top-level block processing.
The block requires an additional field `ebCert : Maybe EBCert`, which is an endorsement block certificate, 
and the block header body also must specify the block type (`EB` or `RB`). 

A block can either contain a list of transactions or an `ebCert`. A block is of `RB` type and contains a list of transactions, 
it is processed similar to a Praos block :
  - block-level checks are performed (including that `ebCert` is not included), 
  - the list of transactions is processed
  - after processing the transactions, the `DIVUP` rule is applied (see below) to modify the state variables used to 
keep track of dynamic pricing.

A block of `RB` type that contains an `ebCert` requires that :
  - block-level checks are performed (same as above),
  - the block-processing rule is called again on the `EB` block corresponding to the `ebCert`

If a block is one that corresponds to an `ebCert` (and is therefore an `EB` block), 
  - it must contain a list of transactions,
  - block level checks are performed (may be specific to `EB` blocks)
  - each transaction is processed
  - the `DIVUP` rule is applied 


#### DIVUP Rule

There is a new rule called `DIVUP` that updates the `SDPolicy` state. The same state that was previously 
updated by `LEDGERS` during block processing is passed to this rule as the input state. Given protocol parameters and 
the block type and its environment, the update does the following :

  1. Checks that if the block containing the transaction list is an EB, at least one of 
  `totalSize , totalRefScriptSize , totalExUnits` exceeds the per-block limits for an RB specified in the protocol parameters
  1. Resets `totalSize , totalRefScriptSize , totalExUnits` to be empty, so that the variables can be reused to 
  track data in the next block
  1. Updates the `diversityPolicy : SDPolicy` to specify new coefficients associated with each tier. **Note that 
  this calculation remains unspecified and should be the result of experimental data**. 


### Block production and node policy

Block producers must account for fee change over time under dynamic fees. Consider this case:

1. A transaction is submitted to the dynamically priced urgent lane during a time of congestion, with more urgent transactions than Ranking Block space. The transaction's posted fee covers the necessary fee _at that time_ but no more.
2. A Ranking Block is produced, but the submitted transaction misses it due to the congestion.
3. The price increases, and the submitted transaction becomes stale. It wasted mempool space while it queued.

The producer-side rule follows from this: a prudent producer fills an EB only with transactions whose max fee covers the quote one price update ahead. One update can fire between selection and the certification check, and an EB filled this way cannot fail fee validation when certified. The rule is EB-specific: RB inclusion is immediate, so RB selection needs only the current quote. The "Revalidation and stale fees" section describes the admission-side counterpart of this rule.

<!-- PORTABILITY: the fee change CIP link below points at a fork branch; repoint at its CIPs-repo PR (or CIP number) once one exists -->

Reminder:

```
step_bound = max(1/D, (1 - targetUtilisation_l) / (targetUtilisation_l × D)), or 0 if lane l has no controller

standard transaction:  max fee ≥ quote_standard × (1 + step_bound_standard)
urgent transaction:    max fee ≥ max(quote_standard × (1 + step_bound_standard), quote_urgent × (1 + step_bound_urgent))
```

These fee-cap rules mean the bare current quote is never sufficient: a user must submit with a buffer against quote movement. With the lane-specific `step_bound` values defined under the "Revalidation and stale fees" section, a lane's quote can rise to at most `quote × (1 + step_bound)^k` over `k` worst-case updates. The ledger itself demands no buffer at all: at inclusion, the posted maximum need only cover the quote at that moment. The one-step requirements are node policy: admission checks one worst-case step ahead of the quote at admission, and an EB producer repeats the same check against the quote at selection. Anything beyond that is the user's insurance against eviction while they wait. A transaction that queues through `k` price updates keeps its place only while its posted maximum covers the current fee-cap quote. A user who expects to wait `k` updates must therefore post enough to cover every applicable lane's quote after `k` worst-case steps (for an urgent transaction, the larger of the two). At the recommended target 0.5 and D = 16 for both lanes, that is `(1 + 1/16)^k` times the current fee-cap quote. A transaction that expects to wait four updates posts roughly 27% above it. A buffer is palatable only with a refund of the difference between the posted fee and the actual quote charged at inclusion. [The fee change CIP](https://github.com/polinavino/CIPs/tree/fee-change/CIP-%3F%3F%3F%3F) describes this refund mechanism.

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
                        or (totalExUnits.memory >= memoryThreshold
                            and totalExUnits.steps >= stepsThreshold)

qualifies(EB) = EB is non-empty
                and (resourceQualified(EB) or the K age escape applies)
```

Either size branch can qualify alone. The execution-units branch qualifies only when both memory and steps reach their thresholds. The rule neither adds ratios nor treats the resources as interchangeable. At the default urgent-controller target utilisation of 0.5, each threshold is half its corresponding RB limit, including a transaction-size threshold of 45,056 B under the simulated RB cap.

Comparisons use integer totals and rounded-up integer thresholds. Floating-point arithmetic is not part of consensus. For any scalar component with usage `x` and positive RB limit `L`, if `urgentTargetUtilisation = p/q`, where `0 <= p <= q` and `q > 0`, the component reaches its threshold exactly when `q × x >= (q - p) × L` and `2 × x >= L`. Implementations evaluate products as mathematical natural numbers or with checked, sufficiently wide intermediates.

The fraction follows the urgent target because a displaced non-certificate Ranking Block carries urgent traffic. A lower urgent target runs Ranking Blocks deliberately emptier, so the urgent lane needs more of them to move the same traffic. Certificates must then be rarer, and qualifying EBs correspondingly fuller. At the default target, qualification requires half the RB limit in either size branch, or half in both execution-unit components. This does not claim that the resources are fungible or that an EB replaces the displaced RB component by component. When the controller target rises above 0.5, the half-RB floor holds the threshold at half. Under the threshold alone, an EB below the qualifying fraction cannot be certified, and standard transactions queue for the next worthwhile batch. The age escape below relaxes that per-certificate property to an amortised one. The Ranking Block rule remains untouched: RBs carry only urgent-paying transactions, at all loads, at all times.

None of these rules requires a validator to know anything about any mempool. Fee validation enforces that every Ranking Block transaction pays the urgent quote, and the quote itself is recomputable from the chain alone: each controller update is a fixed formula over the utilisation of the blocks before it. When `LEDGERS` processes the immutable EB named by a certificate, it accumulates the `SDPolicy` resource totals, and the certificate-inclusion rule compares them with the RB-relative thresholds above. The age escape only counts Ranking Blocks since an EB certificate last entered the chain. A validator that holds only the chain can decide every rule in this section. What a producer's mempool contained never enters into it.

A valid Ranking Block cannot contain a transaction whose on-ledger fee authorisation fails to cover the applicable urgent quote. The premium goes to the treasury rather than the producer, so the protocol offers the producer no direct fee revenue when it undercuts that quote or suppresses an EB. This is an incentive argument, not a broader anti-bribery guarantee: off-chain rebates and side payments, paid ordering within the urgent lane, censorship, withholding, and MEV remain open for the Incentives section. The residual behaviour here is EB suppression: a producer declines to announce a qualifying EB. The RB remains urgent-only regardless, and a later producer can announce the batch. The simulator announces eligible EBs eagerly and does not model withholding, off-protocol side payments, or other adversarial producer behaviour. We explored work-conserving variants that admitted standard transactions into underfull RBs at the standard rate. They retain more value at light loads, but they do not ledger-enforce the applicable urgent quote for RB inclusion, and they leave below-quote side-payment incentives open. We rejected them.

The threshold by itself can starve a trickle load. At very light standard traffic, pooled transactions below every resource threshold can wait indefinitely, and anything that depends on their outputs waits with them. We therefore add a time-gated escape: a certificate for a below-threshold EB can enter the chain once at least K Ranking Blocks have been produced since an EB certificate was last included. The inclusion of any EB certificate resets the count. Both the threshold and the escape are ledger rules, checked when a Ranking Block includes a certificate. A Ranking Block that includes a certificate for a non-qualifying EB is invalid. The rule extends the certificate-inclusion checks that CIP-164 already defines, which every node performs before it accepts a block. Both inputs are on the chain: the count comes from the chain itself, and the certified EB's immutable body determines the `SDPolicy` resource totals that the certificate-inclusion rule checks. Because every certificate inclusion resets the count, at most one below-threshold certificate can appear per K intervals. A reset on certificate inclusion, rather than on announcement, matches what the rule rations: an announced EB that never certifies consumes no Ranking Block space, so it does not reset the count. The escape is permissive, not compulsory. Announcement remains a producer action, and the suppression analysis above is unchanged. The rule remains removable without change to any other rule.

For a candidate certificate-bearing RB `R`, the age count is the number of Ranking Blocks in `(lastCertificateRb, R]`, including `R` itself. Acceptance of any EB certificate resets the count, whether the EB qualified by resource use or by age. If no earlier EB certificate exists, the count starts at mechanism activation.

The certificate-inclusion decision for a non-empty certified payload:

```mermaid
flowchart LR
    P["Certified EB payload"] --> T1{"Transaction size, reference scripts,<br/>or paired execution units at threshold?"}
    T1 -- "yes" --> A["Certificate may enter the RB"]
    T1 -- "no" --> T2{"At least K Ranking Blocks<br/>since the last included<br/>EB certificate?"}
    T2 -- "yes: age escape" --> A
    T2 -- "no" --> W["Certificate is invalid<br/>for this RB"]
```

#### Validation evidence

Every threshold experiment below used the simulator's byte-only gate. The results inform the transaction-size branch and the choice of qualification fraction. They do not validate the reference-script or paired-execution-unit branches of the normative rule.

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

**Experiment description.** A post-correction ablation compared byte gates of 1 B, one-quarter, one-half, and three-quarters of the RB byte cap under the target-0.5, D16, K = 10 mechanism. It paired seeds 0-99 for 2,000 slots with independent random streams across the five headline loads. The arms differed only in the byte threshold. A directional 20-seed check extended the low- and mid-load comparison from three-quarters to one RB.

**Result.** At low and mid load, higher gates improved urgent retention and latency while delaying standard traffic. Relative to half an RB, three-quarters raised the urgent retained-value ratio by 1.94 and 0.52 percentage points and increased standard latency by 0.232 and 0.026 blocks, respectively. The recorded decision-facing contrasts at the three heavier loads were zero or near zero. The one-RB extension continued the same trade-off.

**Interpretation.** These experiments map an urgent/standard policy frontier. They do not identify an unconditional optimum. Half an RB remains the conservative initial default because, in the byte-only simulator branch at target 0.5, it matches the minimum qualifying EB byte payload to the expected urgent byte payload displaced by a certificate. The heavier-load checks had no prespecified equivalence margin, and the one-RB run is directional rather than a replacement for the 100-seed ablation. Every arm retained K = 10 and tested only a byte gate. Neither experiment validates an execution-unit predicate or the threshold expression away from target 0.5.

The starvation and its repair are visible directly in the simulation's demand-fate panels (one representative seed, identical crop and scale):

![Demand fate and retained value at a 0.1 tx/slot trickle with no age escape: every standard class is entirely unresolved and no standard value is retained](images/trickle-0p1-thr-noescape-seed-2.png)

![Demand fate and retained value at the same trickle with the age escape at K = 10: all standard units are included and most standard value is retained](images/trickle-0p1-thr-k10-seed-2.png)

The threshold fraction tracks the urgent controller's headroom, but never falls below half an RB. The normative rule applies the same fraction to the three `SDPolicy` resource groups: transaction size or reference-script size can qualify independently, while execution memory and steps must both qualify. The experiments varied only the transaction-size threshold and therefore do not validate the other two branches.

The historical parameter stress test (ten seeds, four load profiles, detailed in the experiment report) motivates each half of the fraction separately. The sweep derived its byte thresholds from the headroom term at each swept urgent target, while it moved both controller targets together. Fixed-byte-threshold comparison runs at targets 0.25 and 0.75 favoured a qualifying bar that never drops below half the RB byte cap.

In the corrected target-0.25 low-load comparison there were no conditional retry draws, so same-seed runs of the two configurations faced the same exogenous demand and Ranking Block opportunities. The lane and submission outcomes that differ are part of the simulated threshold response. The target-0.75 comparison remains descriptive because no equivalent path audit was preserved. At urgent targets at or below 0.5 the floor does not bind (`1 - urgentTargetUtilisation` is at least `1/2`), so those grid runs realise the completed fraction's values. The completed max() expression was therefore exercised through them and the target-0.75 fixed variant, rather than swept as a unit.

The intuition has two parts. A low urgent target deliberately runs Ranking Blocks emptier, so the urgent lane needs more of them to move the same traffic, and certificates must be correspondingly rarer: the threshold rises with urgent-lane headroom. But a certificate's cost does not shrink when the urgent controller runs blocks hotter, so the threshold must not follow shrinking headroom downward: hence a conservative half-RB floor that limits certificate overhead as headroom shrinks.

The same stress test explores the controller parameters themselves. The sweep set both controllers together at each grid point. Independent per-lane settings were not swept, so the results apply only to the two lanes retuned in lockstep.

At the tested grid points with both targets at 0.5 or both at 0.75, and max-change denominator 8 or 16, the observed comparisons were generally favourable or near-baseline. The strength of the evidence differs by load. Launch-day grid points are paired against flat fee, with intervals that exclude zero. Severe-congestion and EB-capacity-stress grid points are paired against the anchor calibration rather than flat fee. The low-load comparison is unpaired against the flat-fee aggregate and sits within about a point either way (two target-0.75 points marginally below it).

With both targets at 0.5 the advantage holds at every contended load. At 0.75 the EB-saturating result is 31.4% urgent-class retained value against flat fee's 30.1%, without an equivalence margin. With both targets at the tested 0.25, the mechanism retains less value than a flat fee under launch-day load.

The threshold expression uses the urgent-controller target. The historical sweep changed both targets together, so it does not estimate independent standard-controller retuning. The controller parameters are specified as updatable protocol parameters, with the tested grid recorded alongside them. Retuning to untested settings is a mechanism change that requires re-analysis, not a routine parameter update. The parameters, their recommended defaults, and the tested points:

| Parameter | Recommended default | Tested points and observations |
|---|---|---|
| Target utilisation (standard and urgent controllers, swept in lockstep) | 0.5 for each | grid points 0.5 and 0.75 tested. 0.25 tested and excluded (retains less value than flat fee under launch-day load). At 0.75 the EB-saturating result is 31.4% urgent-class retained value against flat fee's 30.1%, with no equivalence margin. Independent per-lane settings not swept |
| Max-change denominator (both lanes, swept in lockstep) | 16 | grid points 8 and 16 tested. 4 tested and excluded (price instability at every load). Independent per-lane settings not swept |
| Urgent signal window | 5 samples | {3, 5}. Windows of 10-20 trade retention for larger price swings |
| Standard signal window | 20 blocks, capacity-weighted | not swept |
| EB announcement threshold | `thresholdFraction = max(1 - urgentTargetUtilisation, 1/2)`. Unless the age escape applies, a non-empty EB qualifies when transaction size or reference-script size reaches `ceil(thresholdFraction × corresponding RB limit)`, or both execution-unit components do. The default fraction is 1/2 (45,056 B for the simulated transaction-size component) | only the transaction-size threshold was tested: the headroom branch was swept while both controller targets moved together over 0.25-0.75. Historical shared-stream fixed-byte-threshold comparisons exercised 45,056 B at targets 0.25 and 0.75. The reference-script and paired-execution-unit branches remain untested |
| EB announcement age escape (K) | 10 RB intervals | K ∈ {5, 10, 20} swept under the simulator's announcement-reset policy. 10 is bit-identical to no escape at ordinary low load and repairs trickle starvation with no statistically detectable urgent-class cost |
| Absolute coefficient floor | 1.0 × ordinary min fee | not swept |
| Cross-lane multiplier floor | none. Temporary quote crossings are permitted, and urgent max-fee checks use the larger current quote | tested at 3× and 16×, rejected |

<!-- PORTABILITY: blob/main link; replace with a commit-pinned permalink before the CIPs-repo PR -->

See the [parameter stress test section of the experiment report](https://github.com/input-output-hk/tiered-pricing/blob/main/docs/phase-2/preliminary-experiment-report.md#parameter-stress-test-controller-settings-and-the-threshold-rule).

The instability that excludes denominator 4 is visible directly in the price trace. The figures below show the per-lane price coefficient over historical severe-congestion runs with the same numbered seed (2,000 slots, seed 0, target utilisation 0.5) and configurations that differ in the max-change denominator. Because these runs used the shared random stream described above, they are descriptive traces rather than a same-exogenous-draw counterfactual. At denominator 4 the run records 88 price moves over 10% (the largest a 25% jump), and the urgent coefficient completes six full oscillation cycles with a peak-to-trough amplitude of 6.7×. At denominator 16 the run records no move over 10% (the largest is 6.3%), one oscillation cycle, and an amplitude of 1.8×, with similar service rates (98.8% vs 97.9%).

![Per-lane price coefficient under severe congestion at max-change denominator 4: the urgent coefficient repeatedly overshoots and collapses](images/d4.png)

![Per-lane price coefficient under severe congestion at max-change denominator 16: both coefficients track demand smoothly](images/d16.png)

### Incentives

Settlement splits every posted bid three ways: the ordinary min-fee component, the premium above it, and the refunded excess (see Block production and node policy). Each destination is chosen for its incentive effect.

The premium is donated to the treasury. It does not go to the block producer: a producer who keeps the premium is no longer indifferent between a legitimate urgent transaction and a side-payment, which recreates the bribery incentive the reservation rule exists to remove. Burning it would be equally neutral for producers; donation is preferred because it keeps congestion revenue inside the protocol's existing funding mechanism.

Protocol fee revenue for producers is unchanged by this proposal. The min-fee component of every included transaction enters the fee pot exactly as fees do today, regardless of lane. The default node policy selects FIFO in both lanes, so the pricing mechanism does not introduce a fee-ordering auction. FIFO is not ledger-enforced, however. Once the urgent lane is saturated, a user may offer a producer tip through nested transactions, as described below. Reservation prevents a producer from selling RB inclusion below the urgent quote; it does not claim to prevent competition among transactions that already cover that quote.

An urgent user pays only for the service received. The premium is scoped to the Ranking Block: an urgent transaction included via an Endorser Block is charged the standard quote. The refund returns everything above the applicable quote, so the posted max fee is a genuine ceiling, and headroom against quote drift costs nothing at settlement. A transaction whose max fee is insufficient is rejected (`BidBelowQuote`) or evicted; both outcomes are visible to the submitter, and neither leaves the transaction queued while its value decays. Eligibility for the urgent lane requires only paying the posted quote, never a prior arrangement with a producer. That is an access guarantee, not a guarantee of inclusion priority once the lane is full.

A standard user is insulated from urgent demand. The standard quote responds only to standard-lane utilisation; it starts at the ordinary min fee, and the absolute coefficient floor prevents it from ever falling below that. An uncontended standard transaction therefore pays what it pays today. The cost this design does impose on the standard lane is batching: standard transactions pool until the selected Endorser Block payload reaches the announcement byte threshold, or until the K = 10 age escape opens. The escape bounds that wait at K Ranking Block intervals, and at typical light load it costs p95 roughly four slots over plain reservation.

Settlement is conservative: base plus premium plus refund equals the posted bid for every transaction, and each component is checkable from on-chain data alone. Fee handling neither mints nor destroys value.

## Rationale: how does this CIP achieve its goals?

This CIP specifies a design, reinforces the design choice with experimental evidence, validates the design with formal specifications and proofs, and proves implementability with a prototype.

### Experimental evidence

> **Evidence scope.** Most quantitative tables below and in the preliminary report predate the max-of-two fee-cap correction and used the simulator’s historical shared random stream. Apart from the dedicated 3×/16× multiplier-floor experiment, they also used no cross-lane floor (multiplierFloor: null). Treat their non-identical comparisons as descriptive historical results, not post-correction causal estimates. The matched denominator-16 and integrated canonical D16/K10 checks were exactly unchanged across all 550 reported scalars, but were bounded checks rather than equivalence tests. The D16/K10 headline rerun, thousand-seed replication, and default-threshold ablation postdate the correction and use independent random streams. 

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

We ran 10 seeds of a 2000 slot simulation under five load profiles: `severe-congestion` (mean 40 tx/slot in slots 0-249 and 1750-1999, mean 160 tx/slot in slots 250-1749), `low` (constant 3 tx/slot, below RB saturation), `mid-load` (constant 5 tx/slot, just above RB saturation), `eb-capacity-stress` (repeated peaks up to ~396 tx/slot driving demand against the EB byte cap), and `launch-day` (measured January 2022 SundaeSwap byte-fullness stages rescaled to the simulator's EB capacity, with modelled onset overshoot and urgency multipliers).

The byte-only simulator realisation of the recommended mechanism is both-dynamic-strict-threshold with a 5-sample signal window, calibrated at target utilisation 0.5 with max-change denominator 16. Under severe congestion it improves urgent-class retained value from 43.56% (flat fee) to 50.85% (+7.29 ± 1.29 percentage points, paired over ten seeds against a matched flat-fee control). It also reduces urgent-class mean latency from 2.98 to 2.50 blocks.

All per-load figures in this and the following paragraph are from the D16/K10 headline rerun below. The experiment report's comparison tables were generated on the denominator-8 anchor configuration. In the historical shared-stream parameter sweep, the reported paired D16-versus-D8 retained-value intervals for severe congestion and EB-capacity stress span zero. Low-load context is unpaired, and mid load was not swept. Denominator 16 records zero price shocks at all four swept loads. Under an extreme high-value demand mix, denominator 8 retains ~2 percentage points more at a large stability cost. 16 remains the default because of the asymmetry of those error modes. 8 remains among the tested settings if persistently steep demand emerges.

At mid load the mechanism beats flat fee by +3.69 ± 1.06 percentage points (ten of ten seeds), and under the EB-stressing load by +8.57 ± 2.24 (ten of ten). Low load is the regime where plain reservation regresses below flat fee. There, the EB threshold repairs the regression and leaves no statistically detectable urgent-class difference from the flat-fee baseline (+0.47 ± 1.62 over ten seeds, and the thousand-seed replication below tightens the bound to -0.03 ± 0.12). Under the launch-day profile the mechanism beats flat fee by +8.15 ± 2.04 percentage points of overall offered value (ten of ten seeds, where offered value is proxied per seed by the flat-fee control's total submitted value, retained + lost + unresolved, because the summary output does not record demand that declines before first submission). Admission is a central channel. The rising standard quote makes low-surplus demand decline to submit. The demand that remains is included with much higher probability, and included transactions also wait less than under flat fee.

We eliminated the other families as follows. Flat fee and single-lane EIP-1559 provide no way to signal urgency, and they leave urgent-class value on the table at every contended load. The historical open-versus-reserved runs record a small gap where capacity is slack (~1-1.6 percentage points at low and mid load). Because the variants differ in more than ledger enforcement, this is a descriptive tradeoff rather than an isolated price of enforcement. Plain reservation falls below the flat-fee baseline at low load, because any standard overflow, however small, triggers a thin EB whose certificate consumes Ranking Block space. Work-conserving variants that admitted standard transactions into underfull RBs at the standard rate retained the most value at light loads. But they likewise leave below-quote RB access and side-payment incentives open, so we rejected them. Long signal windows (10-20 samples) reduce shock counts but trade retention for larger peak-to-trough price swings. The 5-sample window is the compromise point. We prefer both-dynamic over priority-only for two reasons. First, under the EB-stressing load (37.50% vs 32.92% urgent-class retained value), the recorded behaviour is consistent with a standard-lane price that sheds the demand that saturates the Endorser Block. Second, under the historical launch-day load, reservation over a statically-priced standard lane showed no statistically detectable improvement over flat fee, while both-dynamic under the same reservation rule recorded a clear improvement. Within the tested simulator designs, this favours both-dynamic rather than making it a protocol-level requirement. At low and mid load the two families produce identical results: standard traffic never touches the Ranking Block and the standard controller rests at its floor, so both-dynamic degenerates to its priority-only counterpart. Under severe congestion they differ slightly on the denominator-8 anchor tables (51.55% vs 50.74% urgent-class retained value, with both-dynamic carrying roughly five fewer transactions per slot as the standard price sheds demand).

The launch-day contrast is visible in the demand-fate and value panels for a representative seed. In the first figure, note the priority (Pri) rows: under reservation over a statically-priced standard lane, priority demand itself is heavily abandoned, because it bounces at admission behind the standard-lane jam. Under both-dynamic with the same reservation rule, most demand is included and most value retained.

![Demand fate and retained value by urgency class under launch-day load with reservation over a statically-priced standard lane: heavy abandonment and lost value across both standard and priority classes](images/launch-day-priority-only-reserved-seed-2.png)

![Demand fate and retained value by urgency class under launch-day load with the recommended both-dynamic mechanism: most demand included and most value retained](images/launch-day-both-dynamic-strict-threshold-seed-2.png)

Finally, we stress-tested the controller calibration and byte-only threshold design along the parameter axis as well as the load axis: a sweep of target utilisation {0.25, 0.5, 0.75} × max-change denominator {4, 8, 16}, applied in lockstep to both controllers, ten seeds, under low, severe-congestion, launch-day, and EB-capacity-stress loads. At the tested grid points with target utilisation 0.5 or 0.75 and denominator 8 or 16, the observed comparisons were generally favourable or near-baseline. The strength of the evidence differs by load. Launch-day was paired against flat fee, with intervals that exclude zero. Severe and EB-stress were paired against the anchor calibration. Low load was unpaired and within about a point of the flat-fee aggregate. At target 0.5 the advantage holds at every contended load. At 0.75 the EB-stressing result is 31.4% urgent-class retained value against flat fee's 30.1%, without an equivalence margin. At the tested target utilisation of 0.25 the mechanism retains less value than flat fee under launch-day load, and at denominator 4 price stability degrades at every load. We also tested and rejected a cross-lane multiplier floor (a rule that holds the urgent quote at or above a fixed multiple of the standard quote). It overprices the urgent lane precisely when capacity is slack, at a cost of 9-15 percentage points of urgent-class retained value at low load. A demand-elasticity stress test (all values scaled 10×, 10-25% of arrivals at 100× values, each mix against its own flat-fee control) preserves the advantage at every mix. Under launch-day load the advantage grows with the share of high-value demand, while under severe congestion it stays roughly constant across mixes. The byte-threshold fraction and the simulated announcement age escape are direct products of these tests. The application of that fraction to the other `SDPolicy` resource dimensions is a specification completion rather than an experimentally validated result. Protocol enforcement is defined in the Endorser Block announcement threshold section.

#### D16/K10 headline rerun

<!-- PORTABILITY: the experiment report link below is relative; replace with a commit-pinned permalink (same form as the link in the next paragraph) before the CIPs-repo PR -->

We separated the simulator's fresh-demand, ranking-block-production, and retry-jitter random streams. We then reran the canonical byte-only D16/K10 simulator configuration against flat fee over the five headline loads (paired seeds 0–9, 2,000 slots each). The rerun was successful. Low load showed no statistically detectable urgent-class difference (+0.47 percentage points retained value, 95% CI [-1.16, +2.09]). Mid, severe-congestion, and EB-capacity-stress loads improved by +3.69, +7.29, and +8.57 percentage points respectively, with all ten seeds better in each case. Launch-day overall retained value improved by +8.15 percentage points (95% CI [+6.11, +10.19], ten of ten seeds). The recommendation is unchanged within the tested scope. Full results are in the [experiment report](../preliminary-experiment-report.md#d16k10-headline-rerun), and the [preserved headline record](../experiment-results/canonical-headlines.json) retains every table-driving per-seed scalar plus the raw-output, effective-input, executable, and comparison-time source hashes.

#### Thousand-seed replication at low and severe-congestion load

The ten-seed headlines bound seed-sampling error loosely, so we replicated the flat-fee versus canonical byte-only D16/K10 pairing at 1,000 paired seeds (0-999, 2,000 slots, summary-only, independent random streams) under the two extremes of the load axis: low and severe congestion. The other three loads remain ten-seed.

Under severe congestion the replication confirms and sharpens the headline. Urgent-class retained value improves from 43.56% to 50.72%, a paired difference of +7.16 percentage points (95% CI [+7.06, +7.26]), with improvement in all 1,000 seeds. Urgent-class mean latency falls from 2.97 to 2.51 blocks, again in every seed. Overall retained value rises by +0.38 percentage points (95% CI [+0.37, +0.39]).

At low load the interval narrows by an order of magnitude. The urgent-class retained-value difference is -0.03 percentage points with a 95% CI of [-0.15, +0.10], so the 95% interval bounds any effect on urgent-class retention at this load within roughly ±0.15 percentage points. Urgent-class service rate is likewise indistinguishable, and urgent-class mean latency is marginally lower (-0.29 slots, 95% CI [-0.47, -0.11]). These are demand-class metrics: they track the fastest-decaying demand across both variants, whichever lane it uses, not the set of transactions that pay the urgent quote. The mechanism records about 7.6% more urgent-class submissions than flat fee, with the retention and service-rate differences inside the reported intervals. That is consistent with a reserved lane that makes entry attractive to marginal demand, but this comparison does not isolate that channel from the rest of the mechanism.

The larger sample also resolves a cost invisible at ten seeds: overall retained value at low load sits 0.40 percentage points below flat fee (95% CI [-0.41, -0.39], flat fee better in 982 of 1,000 seeds, where ratios count value whose fate resolved within the horizon). The urgent demand class's measured retention and service-rate differences sit within the bounds above, and its mean latency is slightly lower. Attribution of the 0.40 is less clean than a per-lane split, because the mechanism also changes lane choice. Under flat fee effectively all transactions travel the standard path (a mean 5,945 per run). Under the mechanism most of that demand selects the urgent lane instead, leaving 2,139. The transactions that stay standard wait a mean of 57.96 slots against 34.25 under flat fee (2.99 against 1.78 blocks) while they pool for Endorser Blocks worth their certificate. Yet the retained ratio among them is marginally higher than flat fee's (+0.68 percentage points, higher in 910 of 1,000 seeds). The 0.40 is therefore the net of longer standard-lane waits and the shifted lane composition, not a retention loss inside either lane taken alone. It prices the low-load trade in the byte-only simulator realisation. That simulated mechanism costs 0.40 ± 0.01 percentage points of overall retained value against flat fee at this load, in exchange for the +7.16-point urgent-class improvement under severe congestion. (The pairing does not isolate the announcement threshold's own contribution from the reservation rule's.)

A dedicated hundred-seed attribution rerun (paired seeds 0-99, same configuration, per-lane value levels preserved) decomposes this cost. It reproduces the overall difference at -0.399 percentage points (95% CI [-0.443, -0.356]). Entry effects are negligible: the mechanism submits marginally more units and value than flat fee (all 100 seeds), and the per-lane value levels sum exactly to the overall totals in every seed, so the whole difference arises among submitted transactions. There are two accountings, kept separate. The -0.399 ratio counts only value whose fate resolved, and its driver is more value that decays before inclusion (+39.6M lovelace lost). In absolute terms the mechanism also carries more value still unresolved at the horizon (+38.8M), which the ratio excludes. The lane split at this load: about 71% of submitted value selects the priority lane and keeps the flat-fee latency profile (mean 35.0 slots against flat fee's 35.2), while the value that stays standard waits 59.1 slots. Both per-lane retained ratios are composition-shifted. The slower-decaying value stays standard, so its ratio rises even as its wait lengthens. The faster-decaying value migrates to the urgent lane, whose ratio sits below flat fee's overall figure despite equal latency. The [attribution record](./low-load-attribution-smoke.json) preserves the per-seed values for every metric, the derived residual check, and provenance hashes. `scripts/smoke_low_load_attribution.sh` reproduces it. The simulator reports mean announced-EB byte fill of 95.5 KiB under flat fee and 116.5 KiB under the mechanism.

The [preserved evidence record](./thousand-seed-low-severe.json) holds the per-seed values for every metric above, the paired statistics, provenance hashes, and the exact reproduction commands (`scripts/compare_thousand_seed.py` regenerates the record from the sweep outputs).

<!-- PORTABILITY: blob/main link; replace with a commit-pinned permalink before the CIPs-repo PR -->

Full details, including method, configs, per-load tables, paired seed deltas, and figures: [preliminary experiment report](https://github.com/input-output-hk/tiered-pricing/blob/main/docs/phase-2/preliminary-experiment-report.md).

### Prototype

https://github.com/user-attachments/assets/6a4ef69a-516f-4517-bfbd-d7b8a97b09cf

The two lanes live on the devnet (8 min 50 s): a walkthrough of the mechanism and live system, followed by sustained demand, a price squeeze with real evictions, certification pressure and recovery, and the return to an idle network. Captions included.

The mechanism has also been implemented end to end. The prototype patches the Linear Leios prototype node directly: the ledger rules, the consensus mempool, the node's transaction submission path, and the trace pipeline. It runs a three-node Dijkstra devnet with a live dashboard and a generated crowd of senders choosing lanes against the live quotes. Full details live in the [prototype repository](https://github.com/nhenin/dynamic-pricing): the code and a one-command launcher, the per-repository change sets, the design documents, and a section mapping the prototype's vocabulary and calibration to this CIP.

The prototype exercises the transaction lifecycle specified above on a real network rather than a simulator:

- The Ranking Block rule is a ledger rule. A transaction whose max fee does not cover the urgent quote fails with `BidBelowQuote`.
- Both quotes are repriced inside block application. A certified Endorser Block enters the price signals exactly once, at certification.
- Settlement is measured from ledger state, block by block. The min-fee component accumulates in the fee pot, the premium in the treasury, and the excess returns to a refund account named in the transaction body. The three pots sum exactly to what senders paid.
- The mempool admits one worst-case controller step ahead and re-validates under moving prices. A rising quote evicts the transactions whose max fee it overtakes.
- Endorser Block announcement is gated by the byte threshold (45,056 bytes at the default target) and the K = 10 age escape: below the threshold the standard lane pools, and a trickle can be released after ten Ranking Blocks.
- Linear Leios pool votes are cast and traced by all three nodes. Quorum produces the certificate, and the certified Endorser Block is applied by the running protocol. The certification-miss scenario withholds those votes at source rather than fabricating a certificate or ledger outcome.

The prototype runs the recommended controller and fee construction: target utilisation 0.5, max-change denominator 16, the 5-sample and 20-block signal windows (bytes and execution units, larger ratio), no cross-lane floor, the urgent lane's 2× initial coefficient, admission one worst-case controller step ahead, the announcement byte threshold, the K = 10 announcement age escape, and settlement by delivery. An urgent transaction delivered through a certified Endorser Block is charged the standard quote, the excess is refunded, and every urgent fee-cap check uses the maximum of the two current quotes. The Endorser Block is a FIFO merge of standard transactions and urgent overflow; an urgent rider contributes at most one Ranking Block reservation to each certified urgent signal sample.

The prototype deliberately diverges from the recommended construction at two integration points. First, the recommendation merges the entire urgent remainder into the Endorser Block, whereas the prototype holds one Ranking Block's worth of the newest urgent backlog out of that merge. This express reserve keeps the next Ranking Block non-empty while urgent demand is waiting. It is a node-policy experiment, not a ledger rule, and should be evaluated against the unreserved FIFO merge before production. Second, the prototype permits a certificate-carrying Ranking Block to apply its own urgent payload immediately before the certified Endorser Block cargo; the current CIP model treats that Ranking Block as payload-free. This integration choice preserves transaction dependency order in the prototype, but it must either be reconciled with the Linear Leios block-body construction or removed from a production implementation.

The implementation required coordinated changes across four boundaries: ledger validation and settlement, consensus mempool selection and revalidation, the node submission path, and the Linear Leios announcement, voting, certification, and trace pipeline. The most difficult part was maintaining one transaction lifecycle while an Endorser Block is in flight. Announced transactions must leave the selectable mempool to prevent duplicate inclusion, superseded uncertified cargo must be readmitted without violating first-come dependencies, and certified cargo must be applied in dependency order. These are consensus-safety concerns rather than dashboard concerns, and the prototype includes targeted tests and live traces for them.

Making the work production-grade would require replacing prototype constants with governed protocol parameters and era-versioned serialization, completing the formal reordering and dependency treatment (including governance actions), specifying adversarial mempool and DoS limits, integrating wallet fee-cap and refund construction, and testing restart, rollback, reannouncement, and fork convergence across heterogeneous nodes. The two prototype divergences above also need a single normative resolution, followed by conformance tests shared by the formal model, simulator, ledger, consensus, and wallet implementations. The prototype demonstrates implementability; it does not remove those integration and assurance steps.

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

The urgency signal stops discriminating once urgent demand itself exceeds Ranking Block capacity: every RB candidate already pays the urgent quote, so the flag can no longer separate them. In this case, users could use [nested transactions](https://github.com/cardano-foundation/CIPs/pull/862) to offer the block producer a tip that buys priority within the urgent lane. The nested-transactions proposal does not itself specify producer tips or selection priority.

With nested transactions `tx` implemented, any user can create an incomplete transaction whose `produced` value 
is less than its `consumed` value. The funds
that make up the difference can be directed to any user running a nested transaction aggregator. The aggregator 
can then construct a complete transaction `tx'` containing `tx` directing the funds difference to the aggregator's
address. This works regardless of which aggregator receives `tx`, but only the aggregator who successfully submits 
a block that contains `tx'` (and gets included in the chain) will receive the "tip". This way, the transaction author 
can tip whoever's completed transaction makes it on-chain, likely the same user that is running the aggregator alongside 
a node who turn to produce a block it happened to be.

## Path to Active

### Acceptance Criteria



### Implementation Plan

## Versioning

<!-- PORTABILITY: CIP-84 link becomes the repo-relative ../CIP-0084 once inside the CIPs repo -->

Transaction urgency signalling changes the rules by which transactions are admitted to Ranking Blocks under linear-Leios. Where this affects ledger validation, transaction format, fee calculation, or block validity, it requires a new major protocol version and a new ledger era, and [CIP-84](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0084) applies.

A hard-fork event enables the mechanism, either as part of the linear-Leios hard fork or in a later hard fork. Incompatible changes require a successor CIP and a subsequent protocol version.

<!-- PORTABILITY: the fee change CIP link below points at a fork branch; repoint at its CIPs-repo PR (or CIP number) once one exists -->

This CIP also depends on [the fee change CIP](https://github.com/polinavino/CIPs/tree/fee-change/CIP-%3F%3F%3F%3F).

## Copyright

This CIP is licensed under [CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/legalcode).
