This brief write-up closes out Phase 1: Requirements Gathering and Analysis.

This phase ended up producing more than was initially intended; giving us clarity on the paper's mechanism, and preliminary simulation signals for how it interacts with a variety of loads.

These preliminary signals give us guidance for concrete experimental results in the next phase.

**What we did:** Analysed the "Tiered Mechanisms for Blockchain Transaction Fees" paper in the context of linear-Leios; started to build a simulation testbed allowing for quick and easy experiment setups via config files; identified where the paper's assumptions diverge from Cardano's protocol structure; altered the Cardano ledger formal specification accordingly.

**Key finding:** The RB/EB split in linear-Leios creates an inherent two-lane latency structure (20 vs 56 slots) that the paper doesn't account for. This suggests the tier mechanism should let users choose between block types, rather than treating block space as homogeneous.

**What's unresolved:** The questions in this report (particularly around delay enforcement, overflow policy, and repricing frequency) need both formal research input and validated experimental answers.

**Phase 2 focus:** Complete the simulation machinery, then run controlled experiments against the specific questions listed here - particularly the structural comparison (RB-fast/EB-slow vs paper-like) and oscillation thresholds.

---

Optimisation target:
Our working assumption is that we must optimise for price stability while minimising utility loss through delay or rejection. Note that these two things are not the same, and can even be at odds with each other. 
The quicker price adjustments occur, the less time tiers will be mispriced when under rapidly changing loads, but the more time will be spent mispriced due to oscillation. Similarly, the slower price adjustments take,
the longer it'll take for changing demands to be caught up with, from a pricing perspective.

Another framing is that the mechanism should minimise the time spent mispriced in conjunction with the magnitude of mispricing; essentially, cumulative mispricing over time.

The paper's primary target is traffic diversity. We need to decide whether this should also be an optimisation target for us. This will impact the set of solutions we consider; for example, if we choose that this isn't something
we need to worry about, simple Ethereum-style dynamic pricing may be sufficient.

---

Where the paper's assumptions diverge from Cardano / linear-Leios:

**Block production rate is fixed and uniform.** The paper assumes a constant throughput B (txs/block) with one block per time unit. Linear-Leios has two block types (RB, EB) with different production rates (~1 per 20 slots vs ~1 per 56 slots) and neither is guaranteed per-slot.

**Block space is a single homogeneous stream.** The paper's tiers subdivide a single block's capacity. In linear-Leios, RBs and EBs are structurally different - EBs reference transactions, RBs carry endorsement certificates and potentially inline transactions. Tiers must map onto this two-lane structure.

**Transactions are homogeneous in size.** B is measured in transactions per block. Cardano transactions vary significantly in byte size (and compute cost for Plutus scripts), so tier fullness must be measured in bytes (and possibly execution units), not tx count.

**Pricing is one-dimensional (fee per transaction).** Cardano's fee model (especially with Plutus) is two-dimensional: bytes and execution units. The paper's per-tier price coefficient doesn't account for this.

**Delay enforcement is assumed, not specified.** The paper says transactions "remain in limbo" until their delay elapses (Section 4.2), but doesn't describe how this is enforced at the ledger or mempool level - when transactions are validated, how immature transaction IDs propagate through blocks, or how stale maturing transactions are cleaned up.

**Tier assignment is instantaneous and final.** The paper's model has users observe current prices and choose a tier atomically. In practice, there's a propagation delay between price observation and transaction arrival, during which prices may change - creating a potential stale-assignment problem.

**No overflow or rejection mechanics.** The paper's steady-state analysis assumes demand self-selects into tiers via pricing. It doesn't model what happens when a tier is over-subscribed before prices adjust - there's no mempool overflow, rejection, or retry.

**The "exclusion tier".** The paper's tier 0 represents transactions that choose not to participate. In practice, these transactions simply wouldn't be submitted.

**No MEV or information asymmetry.** The paper assumes all users are myopic and truthful (Definition 1). Tier choice as a public urgency signal is not modelled as an attack surface.

---

Structural direction:

Something to note - linear-Leios introduces a complication for the paper's solution; the paper assumes a fixed block rate `B` for transactions/block. In linear-Leios, `B` is variable, since we have RBs and EBs. Preliminarily, we've decided that "Delay" in this context should just be counted in RBs.

Preliminary simulation outputs suggest certain avenues we should explore; one candidate direction we're considering assumes that linear-Leios is implemented with its Ranking Blocks being capable of holding both an Endorser Block certificate _and_ inline transactions.

This would allow us to offer transactions a choice of two separate sets of tiers:
* Tiers targeting a Ranking Block
* Tiers targeting an Endorser Block

There's an important reason that this choice should be left to the user: a Ranking Block is produced, on average, every 20 slots, while an Endorser block is included, on average, every 56 slots.

This is an inherent protocol-level statistical latency that is _separate to the tiered pricing mechanism_. As such, we're tentatively leaning towards a regime under which a transaction makes the selection between block types itself.

In addition to this finding, we've spotted some potential implementation considerations:

* When should a node stop requesting additional transactions for a given tier? (Assuming TxIds arrive with unverified tier metadata, this likely depends on tier fullness and/or overfullness.)
* When a transaction is rejected due to tier overfullness, should the node retain any data about the rejection?
* If so, can and should aggregated rejection/resubmission data influence tier calculations? What form would that aggregate take in a block?

---

Plutus impact:
Depending on whether or not the Plutus/Ledger/Product teams think it is relevant to include the transaction tier in the TxInfo data passed to Plutus scripts, a new version of Plutus may or may not be needed. If the TxInfo remains the same (i.e. unaware of the tx's tier), it appears that no changes are needed. If it is decided that scripts should see the tier, a new version of Plutus, with the relevant change to TxInfo is needed. Only scripts written in this new version will be able to view the tier. Old-version scripts would not be shown tier information.

---

Ledger impact:
Let us define TxTier, which includes:
(1) the tierCoeff, which is the coefficient by which the minfee of the transaction must be multiplied to determine the actual minimum fee the transaction must pay to be processed
(2) the timeToWait, which is the delay that must be imposed on the transaction (looked up at the time of transaction construction) corresponding to the tierCoeff

See attached images (slides 7-9 here)

New protocol parameters:

| Parameter | Function |
|---|---|
| `k` | Maximum number of tiers |
| `maxDelay` | Max artificial delay |
| `dFreq` | How often to update delays |
| `tFreq` | How often to update tier count |
| `targetLoad` | How full should tiers be |
| `addTierPrice` | Threshold to add new tier |
| `removeTierPrice` | Threshold to remove new tier |

This change will not be backwards compatible (with transactions not specifying the tier), since not specifying a tier will make it impossible for the mempool to determine the appropriate delay.

| Component | Field name | Function |
|---|---|---|
| Block header | `incomingIds` | Ids of txs that just arrived in the mempool and are starting their artificial delay this block/slot |
| Tx body | `txTier` | Tier coefficient and artificial delay of the tx |
| Ledger state | `diversityPolicy` | Diversity policies cor. to the slots in which they were implemented |
| Ledger state | `pending` | Incoming tx IDs (recorded from block headers) alongside the amount of time they have been delayed so far |
| Ledger state | `totalSize` | Keeps track of total memory occupied by txs in each tier processed so far in the current block (updated after each tx is processed) |
| Ledger state | `totalExUnits` | Keeps track of total `ExUnits` required in current block by txs processed so far (updated after each tx is processed) |

---

Definitions:

| Term | Definition |
|---|---|
| Tier | A pair of: delay and price coefficient; or, under lane-partitioned policies, a triple of: delay, price coefficient, and block type |
| Tier Status | Kinds of tiers (active + accepting submission, active but not accepting submission, inactive, etc.) |
| Delay | In unit blocks; the number of events (RB productions in this case) that must occur before a transaction can be included |
| Delay Period | The period of time between a transaction's entry to the mempool and the time at which the transaction becomes mature |
| Demand | The set of all transactions that would have non-negative value if included without fee |
| Load | The demand with non-negative utility given the mechanism's current prices |
| Inclusion | The percentage of load which has been included in a block |
| Value | Monetary value |
| Mature transaction | A transaction whose delay period has ended, but which hasn't yet been included in the ledger |
| Pending transaction | A transaction whose delay period has not yet ended |
| Base fee | The underlying fee per byte |
| Price coefficient | A value by which a transaction's base fee is multiplied in order to calculate its final fee |
| Target utilisation | The fullness threshold above which a tier's price is increased, and below which a tier's price is decreased |

It should be noted that the distinction between demand and load is a new introduction. In the model demonstrated in the paper, transactions which choose not to participate because their utility would be negative under the current tiers target tier 0, which is essentially a sink tier representing exclusion.

---

Questions for Research

Tier structure and dynamics:

1. Is it ok to populate a blocks tiers not based on exact delay specified by the transaction, but rather in terms of delay buckets : 0-1 block delay, 1-5 block delay, etc.
2. Would having a fixed number of tiers negatively impact the goals of the paper's solution?
3. In the paper, the calculation to add or remove tiers occurs regularly, after a specific number of blocks: tFreq. This is counter-intuitive to me, since it seems that this interval should depend on certain factors, such as transaction volume.
4. The paper treats txs as homogenous in size and fee. On Cardano, transactions have 3 parameters by which block fullness is determined, and from which fee is calculated: size, and 2 parameters of ExUnits. How do we best use these parameters to specify tier sizes?
5. A tier can always be making more money even when it is filled (in terms of memory amount/size) to the "target load" - this would be possible by adding transactions that have the same size, but run more expensive scripts. Can we reflect this in our design somehow?

Delay, maturity, and validation:

6. If B is txs/block, and n is incoming txs/slot, do we just convert txs per slot into txs per block by multiplying by slots per block? In Praos, there isn't a guarantee of either 1 block per slot OR 1 slot per block. Blocks are not a measure of time - how are we supposed to treat it as a rate?

7. We were thinking of guaranteeing transactions mature for the right amount of blocks in the following way (slightly different than what the paper says):
   * When a block producer makes a block, it lists transaction IDs of transactions that arrived in the pool right before producing the block, and are not currently mature (call this list immatureTxIDs), along with how many blocks ago it arrived
   * The transaction bodies of immature transactions are not included in blocks
   * Immature transactions are not validated until they reach maturity
   * When transactions have matured (or are top-tier), and have been validated, they can be included in blocks. During block validation, nodes check that the block contains only transactions that have been previously announced as "pending" and have waited the required delay (or they are top-tier)

   The questions are (these might be more for mempool engineers):
   * When are transactions validated? At the time of arrival in the mempool (and if yes, is there enough time to do this)? At the time of being placed into a block? Or both?
   * DDoS seems like a big danger here regardless of whether we validate transactions at the time of arrival OR at the time of inclusion in a block for full processing. If transactions aren't validated at arrival time, many bad immature TxIDs can end up in blocks and in the ChainState (potentially forever). If transactions aren't (re-)validated at the time when they will be processed in full, many of them might become invalid by then, invalidating an entire block.

8. How can we clean up "maturing transactions" in the chain state? Can we clean based on maximum delay? Can the max delay be a protocol parameter? Will discarding them cause problems?

Pricing and UX:
9. We want to address a potential UX weakpoint: without mitigation, if a tx is submitted targeting a given tier, and that tier's price is increased, the tx may not be able to cover the fees. We'd like to introduce a mechanism to honour the fee price at the time of submission. As such, we'd like for a tier's fees to be honoured for D units of delay after any given tier change that would result in a transaction being deemed invalid. Would this cause any problems?
10. Perhaps a contrived case, but if a large spike of transactions targeted a specific tier in great excess of its capacity, those transactions would sit in the mempool until they can be validated, only to be rejected due to tier price increases. Is there a way around this other than keeping track of tier fullness and rejecting from the mempool?
11. What kind of loss of inclusion would be considered acceptable?

Linear-Leios specific:
12. Assuming that nodes get the same sets of transactions in their mempools coming in across the network, there would never be a valid EB floating around that was "certified too late to be included in an RB". This is because that EB would likely contain most of the same transactions that the RB that was released for which that EB was certified too late, right?

---

Questions for Experiments:

1. Does the paper's solution applied to linear-Leios result in performance greater than or equal to its application to a Praos-like structure?
2. How does the EB = slow lane, RB = fast lane shaped solution compare to the paper-like solution's performance?
3. How much does it affect each solution candidate if RBs can include both an EB certificate and transactions?
4. Which solutions are most versatile in terms of load distribution variations?
5. At what repricing frequency does oscillation emerge, and how does this vary by load profile?
6. Does the mechanism achieve price discrimination (different urgency classes paying different fees) under each candidate design?

---

Discussion of engineering techniques for simulations:

* **Reproducibility:** all randomness (transaction arrivals, mechanism internals) should be seeded, so that the same seed and parameter set always produces an identical run.
* **A/B comparison as primary unit:** mechanism design questions are inherently comparative. Each experiment should measure against a well-understood baseline (fixed-fee or single-tier EIP-1559), not in isolation.
* **Sensitivity over point estimates:** single runs tell us little. We should sweep parameters, run multiple seeds, and look for phase transitions (e.g., the load at which rejection begins, or the repricing frequency at which oscillation emerges).
* **Vary load profiles:** test steady-state overload (the paper's setting), transient congestion bursts, and mixed-urgency moderate load.
* **Welfare metrics over throughput:** inclusion rate is trivially maximised by setting prices to zero. The metrics that matter are retained value ratio, per-urgency-class welfare (does the mechanism achieve price discrimination?), and per-tier price stability over time. Revenue should be tracked as a secondary metric.
* **Know what simulation can't answer:** incentive compatibility (game-theoretic, needs formal analysis), network-level attacks like MEV (require modelling information asymmetry), and ledger validation costs (implementation-dependent). These are research questions, not simulation questions.

The way we'll achieve these objectives is by building on top of the existing `sim-rs`, designed to run experiments for Leios.

Each design choice (whether we reject transactions for tier fullness, whether we have tiers at all, whether we have dynamic pricing) should all be simple configuration options. Each experiment will essentially _be_ a config file. This will allow us to build out many comparative experiments rapidly.

We should also have clear, readable reporting of key metrics, such as inclusion rate, average inclusion latency, rejections (where applicable), retries (where applicable), tiers (their price, delay) over time.

---

Good UX:

* Submitters of a transaction can know (by some mechanism) if the tier they've subscribed to is likely to be full in most nodes, thus giving them the opportunity to retry
* Submitters of a transaction should be able to reliably know, with a fair degree of accuracy, the true expected latency of inclusion for their transaction before submitting
* At a wallet level, a recommendation for the optimal tier based on a tx's delay-value map

---

Preliminary security discussion:

Note - the paper doesn't discuss the topic of security.

It appears that the higher the resolution of higher tier selection, the more we enable MEV behaviour. This isn't necessarily a problem, but it's worth bearing in mind.

To elaborate: the more tiers available, the more precisely a transaction's urgency is revealed by its tier choice. This legibility of urgency preferences may increase the surface area for MEV extraction, since observers can identify high-urgency transactions and front-run accordingly. In a tiered system, tier choice is a public signal of urgency.

---

Design space options:

| Category | Knob | Options |
|---|---|---|
| **Structure** | Mechanism type | Fixed-fee / single-tier EIP-1559 / multi-tier dynamic |
| | Number of tiers | Fixed vs dynamic (paper's tFreq add/remove) |
| | Lane partitioning | Single block stream (paper-like) / RB = fast + EB = slow / tiers spanning both RB and EB continuously |
| | RB inline transactions | Can RBs carry transactions alongside EB endorsements? |
| **Pricing** | Repricing cadence | base_fee_change_denominator (how aggressive per-block updates are) |
| | Delay update frequency | How often tier delays are reassessed |
| | Tier add/remove frequency | How often tier count changes |
| | Tier add/remove signal | Price-driven (paper: last-tier price thresholds) vs demand-driven (fill-rate thresholds) |
| | Price boundary enforcement | Whether p_{i+1} ≤ μ_i × p_i (slower tiers must be cheaper) is actively enforced |
| **Admission & overflow** | Overflow policy | Queue in mempool / reject at submission / reject with retry |
| | Rejection data visibility | Does aggregated rejection/overflow data influence repricing? |
| | Overflow pricing mode | Overflow counted as fill-rate vs linear additive price increment |
| | Retry policy | Enabled/disabled, backoff curve, per-urgency-band retry limits |
| **Capacity** | Tier size allocation | Fixed fractions vs dynamic rebalancing based on observed demand |
| | Tier 0 reservation | None / partial / full RB reservation for fastest tier |
| | Target utilisation | Fill-rate threshold separating price-increasing from price-decreasing |
| **Validity** | Assignment semantics | Never-stale (honoured forever) vs revalidate at inclusion time |
| | Delay unit | Slots vs blocks |