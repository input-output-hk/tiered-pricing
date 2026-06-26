---
CIP: ?
Title: Transaction Urgency Signalling On Linear-Leios
Category: Consensus
Status: Proposed
Authors:
  - Will Gould <will.gould@iohk.io>
Implementors: []
Discussions: []
Solution-To:
  - CPS-0031
Created: 2026-06-24
License: CC-BY-4.0
---

## Abstract

We propose a solution with two pathways a transaction can submit to a node with: urgent and standard. Only urgent transactions can enter Praos blocks, while both urgent and standard transactions can enter endorser blocks. Since Praos blocks will be produced more frequently than Leios blocks, and are included on-chain immediately, this offers users who submit urgent transactions a route to quicker inclusion.

We show, using a simulation, that we can increase retention of value for urgent transactions by a significant amount.

## Motivation: why is this CIP necessary?

With the introduction of linear-Leios, transaction inclusion latency increases slightly, and the variance of latency increases also. To off-set this, it'd be helpful to be able to signal urgency, to allow nodes to better allocate block-space to serve users' intents.

See CPS-0031 for more information.

### Why not full tiered pricing?

A mechanism based on the paper [Tiered Mechanisms for Blockchain Transaction Fees by Kiayias et al](https://arxiv.org/pdf/2304.06014) was initially planned to be the subject of this CIP. After discussion with stakeholders and investigation into the technical requirements of such an implementation, it was decided that a reduced-complexity version would be adequate for community needs. A simpler version would also be easier to prove, would be less likely to cause regression, and would be implemented sooner, potentially offsetting any value-retention differential anyway.

## Specification

This CIP introduces a transaction-level urgency signal with two lanes: standard and urgent. Urgent transactions pay a different, dynamic fee quote and are eligible for inclusion in both Ranking Blocks and Endorser Blocks. Standard transactions are eligible only for Endorser Blocks. The ledger enforces that Ranking Blocks contain only urgent-paying transactions. The dynamic fee is controlled by the EIP-1559 algorithm.

We specify that Ranking Blocks can only contain urgent transactions to prevent bribery of block producers. If bribery is allowed, a block producer has the incentive to accept a bribe over a legitimate urgent transaction, because it means they'll get to keep the entire bribe. If, instead, they chose to include a legitimate urgent transaction, the excess fees would go to rewards or the treasury, depending on the policy we decide. As such, preventing that incentive is necessary.

<details>
<summary>Show glossary of terms</summary>

<br>

**Standard transaction**: A transaction which is not attempting to pay to enter the urgent lane. Cardano's current transactions.

**Urgent transaction**: A transaction which has attempting to pay to enter the urgent lane. This signals that the transaction should be included before standard transactions, where possible.

**Reserved**: A urgent lane mechanism under which RB block space is reserved for urgent transactions, enforced on-chain.

**Reservation policy - ceiling**: A reservation policy that enforces a maximum on the share of block space occupied by urgent transactions. For example, a concrete version of this policy could be that no more than 85% of a block's bytes may carry urgent transactions. If urgent demand exceeds the cap and standard demand is below the remainder, the cap leaves block space unused.

**Reservation policy - floor**: A reservation policy that reserves a minimum portion of block space exclusively for urgent transactions; standard transactions cannot occupy it. For example, with a 15% floor, if no urgent transactions exist and standard demand is 100% of block size, only 85% of block space will be occupied.


#### Lanes and routing

**Standard lane**: A pathway for transactions that do not pay the urgent fee.

**Urgent lane**: A pathway for transactions that do pay urgent fee.

**Lane selection (the user-side decision)**: The choice of lane, made by the constructor of a transaction.


#### Pricing primitives

**Pricing coefficient**: The value by which the base fee is multiplied (which results in the quote).

**Quote**: The result of multiplying the pricing coefficient by the base fee; in effect, a snapshot of the dynamic fee for a given transaction.

**Urgent premium**: The difference between the urgent lane quote and the standard lane quote.

**Absolute coefficient floor**: The minimum allowed lane pricing coefficient, expressed as a multiple of the transaction's ordinary Cardano minimum fee. In the current simulator configurations this is `1.0`, meaning the lowest possible quote is `1.0 × minFee(tx)`, i.e. the ordinary Cardano minimum fee.

**Static (pricing)**: Basic Cardano fee, as today.

**Dynamic (pricing)**: EIP-1559 style dynamic fee.

**EIP-1559 (controller)**: todo

**Smoothing window**: todo

**Target utilisation**: todo

**Quote drift**: Potential or true delta between a quote at the time of transaction submission vs the time of inclusion.


#### User-side fee fields

**Posted fee vs actual fee**: todo

**Refund**: The process of returning the unnecessary excess of a fee to a specified address.

**Max fee (max_fee_lovelace / fee ceiling on the user side)**: todo


#### Welfare / actors

**Urgency**: The rate at which the value of a transaction decays.

**Retained value / welfare**: The sum of transaction value that did not decay prior to inclusion.

**Mispriced actor**: todo
</details>

<br>

The specification touches a few different areas:

### Mempool

We have proven <put link here> that causally independent transactions can be re-ordered, with the exception of governance actions. This means that no significant changes are required to the mempool, although we do need to adjust the validation performed when a transaction with the urgent flag enters the mempool. This is because we need to ensure that the transaction is valid both at the end of the entire mempool, _and_ at the end of the urgent queue. This comes with a (slightly less than) doubling of the phase-1 validation check costs for urgent transactions. Since phase-1 check costs are very low, we consider this to be an acceptable trade-off.

<add a description of changes to the mempool algorithm if any>

#### Queue structure

TODO: describe whether nodes maintain separate standard/urgent views, whether urgent
transactions are selected from a distinct queue, and how this interacts with the existing
mempool order.

#### Admission validation

TODO: urgent transactions must be valid both against the full mempool state and against
the urgent-only selection state. Link to reordering proof and describe governance-action
exception.

#### Revalidation and stale fees

TODO: describe what happens when the urgent coefficient changes after admission. A tx
whose max fee no longer covers the required urgent fee may be retained, evicted, or
downgraded by node policy, but must not be selected into an invalid RB.

#### Dependencies and conflicts

TODO: describe how dependent transactions are handled when one transaction is urgent
and another is standard, especially UTxO dependencies and causally dependent chains.

#### Capacity, eviction, and DoS

TODO: describe whether urgent transactions get reserved mempool capacity, whether stale
or underpriced urgent transactions are evicted preferentially, and the resource impact of
extra phase-1 validation.

#### Governance actions

TODO: describe why governance actions are excluded from the general reordering result
and what policy applies to them.

### Ledger

Since we want to enforce the rule that only transactions paying a sufficient fee to enter the urgent lane may be admitted to Praos blocks, we must make ledger changes <put link here>.

#### Transaction representation
#### Fee validity
#### Block validity

### Block production and node policy

Block producers need to be cognisant of fee change over time, with respect to dynamic fees. Consider the case:

* A transaction is submitted to the dynamically priced urgent lane during a time of congestion, with more urgent transactions than Praos block space. The transaction's posted fee covers the necessary fee _at that time_ but no more.
* A Praos block is produced, but the submitted transaction misses it due to the congestion.
* The price increases, and the submitted transaction thus becomes stale, wasting mempool space during the time it was queued.

As such, in order for the system to operate, transactions must be submitted with a suitable buffer. In order for adding a buffer to be palatable, a mechanism must be present to refund the difference between the posted fee and the actual price a transaction is charged for admission to the block. This mechanism is described in <fee change CIP link>.

### Incentives



## Rationale: how does this CIP achieve its goals?

This CIP specifies a design, reinforces the design choice with experimental evidence, validates the design with formal specifications and proofs, and proves implementability with a prototype.

### Experimental evidence

Our experimental setup was as follows:

| Family | Reservation policy | Standard lane | Urgent lane | Signal variants |
|---|---|---|---|---|
| flat-fee | none | fixed | n/a | n/a |
| single-lane-eip1559 | none | dynamic | n/a | n/a |
| priority-only-open | open priority-first | fixed | dynamic | instant, windowed 3-20 |
| priority-only-reserved | RB reserved | fixed | dynamic | instant, windowed 3-20 |
| both-dynamic-open | open priority-first | dynamic | dynamic | instant, windowed 3-20 |
| both-dynamic-reserved | RB reserved | dynamic | dynamic | instant, windowed 3-20 |

We ran 10 seeds of a 2000 slot simulation with a mean load of 40 tx/slot between slots 0-249 and slots 1750-1999, and at a mean load of 160 tx/slot between slots 250-1749.

<link to experiment report>

## Path to Active

### Acceptance Criteria



### Implementation Plan

## Versioning

Transaction urgency signalling changes the rules by which transactions are admitted to Praos blocks under linear-Leios. Where this affects ledger validation, transaction format, fee calculation, or block validity, it requires a new major protocol version and a new ledger era, and [CIP-84](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0084) applies.

The mechanism is enabled by a hard-fork event, either as part of the linear-Leios hard fork or in a later hard fork. Incompatible changes require a successor CIP and a subsequent protocol version.

Additionally, this CIP is dependent on the fee refund CIP <link the fee refund CIP>.

## Copyright