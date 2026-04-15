---
CPS: 23
Title: Urgency Signaling
Category: ???
Status: Open
Authors:
  - Will Gould <will.gould@iohk.io>
Proposed Solutions: []
Discussions: []
Created: 2026-04-07
License: CC-BY-4.0
---

## Abstract

During periods of congestion, high-urgency transactions lose value because they experience delay they cannot avoid. The capability for transactions to signal protocol-recognised urgency to better serve transactions most sensitive to delay could improve overall retained transaction value during congestion.

Candidate solutions should be evaluated by how they handle prioritising high-urgency transactions, and by how they affect ordinary and low-urgency users during sustained congestion.


## Problem

Cardano does not currently provide a protocol-enforced way for a user or application to signal transaction priority.

Many transactions are time-sensitive: their value to the submitter depends on timely inclusion. A liquidation that lands several slots late may fail to recover the full loan value. An oracle update delayed behind unrelated traffic leaves a stale price on-chain. A loan collateral top-up submitted before a margin call but confirmed after it is worthless. In each case, delay destroys value that would have been captured had the transaction been included promptly.

During congestion, these transactions compete for block space on equal terms with traffic that has no particular time sensitivity. The protocol treats all transactions identically (with respect to protocol-enforced urgency or priority ordering): there is no way for a transaction to express that it is urgent, and no mechanism for block producers to commit to honouring such a signal. Urgent and non-urgent transactions queue together, and inclusion order is determined by factors opaque to the submitter.

The cost is borne across the ecosystem. Users and protocols lose value to avoidable delay. Block producers leave welfare on the table by not capturing the urgency requirements users would express if they could. And the absence of a legitimate priority channel creates pressure toward off-chain arrangements that undermine the permissionless properties of the network.

## Use Cases

1. **Liquidations and collateral auctions**

   **Scenario:** A lending protocol needs to liquidate an unsafe position before collateral value moves further.

   **Example:** A liquidation transaction lands several slots late during unrelated minting congestion.

   **Who loses today:** Depositors, LPs, protocol backstops, and sometimes borrowers if auctions become disorderly.

2. **Oracle updates**

   **Scenario:** An oracle publisher needs to update a price feed during market volatility.

   **Example:** A stale price remains on-chain because the update competes with non-urgent traffic.

   **Who loses today:** Protocols consuming the stale feed, users trading against incorrect prices, and systems relying on time-dependent parameters.

3. **Collateral top-ups and position protection**

   **Scenario:** A borrower tries to add collateral or repay debt to avoid liquidation.

   **Example:** The user submits a corrective transaction in time, but it is delayed behind unrelated congestion.

   **Who loses today:** Borrowers who attempted to act, and protocols that want orderly risk management rather than avoidable liquidations.

4. **Deadline-sensitive user transactions**

   **Scenario:** A user needs inclusion before a known deadline, such as an auction close, mint window, claim period, or liquidation threshold.

   **Example:** The transaction is valid and submitted before the deadline but confirms too late.

   **Who loses today:** Users who cannot express that deadline sensitivity in a protocol-recognized way.


## Goals

1. **Reduce value destroyed by avoidable delay.** A mechanism should exist by which urgent transactions, which would have lost value if queued behind traffic with no time sensitivity, can significantly improve retained value.

From stakeholder interviews during Buidler Fest #3, hosted by Carlos Lopez De Lara:

2. **Permissionless access.** Priority must be available to anyone willing to fulfil the necessary prerequisites, not negotiated through relationships or in private arrangements.

3. **Predictability over raw speed.** The signal predictably improves access to timely inclusion, rather than only modestly improving odds. This includes a reduction in wait time variance for high-urgency transactions.

## Constraints

Candidate solutions must also satisfy:

1. **Multi-input awareness.** Complex DeFi transactions spend multiple UTxOs atomically. A contested UTxO mechanism scoped to single-UTxO contention covers only ~30–40% (estimated by stakeholders during interview) of real lending liquidation scenarios.

2. **Bot-composable semantics.** The priority signal must be encodable in smart contract logic and readable by automated systems, not just manually configured.

## Non-Goals

Guaranteed inclusion of every urgent transaction

Guaranteed retention of value for urgent transactions

Any specific pricing mechanism

## Explored Alternatives

From stakeholder interviews at Buidler Fest #3:

* Fee pre-escalation: Transactions can overpay fees, but with no protocol-enforced prioritisation for overpaying transactions

Tried. Produced ~15–20% (estimated by stakeholders during interview) improvement in moderate congestion. Fails under systemic congestion because SPOs are not committed to sort by fee. Bidding is also calibrated blind; there is no standardized mempool signal to know where you stand.

* Multi-relay submission: Where the node is connected to multiple SPO relays to increase the likelihood that the transaction reaches the next block producer quickly

Deployed as standard infrastructure. Improves latency-to-mempool, not confirmation ordering. Once in the queue, the transaction competes equally with everything else.

* Private SPO arrangements: 

Explored and rejected. Even agreements with major SPOs yield only ~30% (estimated by stakeholders during interview) next-block probability; insufficient for liquidations. More importantly, this produces a worse outcome than a formal mechanism: an opaque, permissioned, off-chain priority market accessible only to well-capitalized incumbents.


## Open Questions

How can whatever protocol-level commitments are decided upon be enforced or incentivised?

How should changing fees be effectively transmitted?

How does priority interact with the Leios block structure?

Can we achieve our goals without starving low-urgency users of block space (especially in the context of Leios)?

How can we retain fee quote validity across repricing intervals?

Is it problematic to leak urgency information?

Is there an MEV implication here? If so, how significant?

## Copyright

This CPS is licensed under CC-BY-4.0.
