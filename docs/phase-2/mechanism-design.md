# Phase 2 mechanism design — recommended candidates

Important note: This document was written by Claude based on an extensive back-and-forth quiz of Will Gould's perspective on the mechanism. As such, all sources of meaning in the document have an actual provenance.

This is the spec for the dynamic-pricing mechanisms that survive the phase-2 down-select. It describes the *deployment design*: the on-ledger and node-policy behaviour each candidate would have if implemented on linear-Leios. The simulator at `sim-rs/` validates the design empirically; where the simulator's implementation diverges from this spec, the spec is authoritative and the gap is documented in the *Methodology* appendix.

The full diagnostic walk and per-experiment evidence is in `experiment-journal.md`.

## Motivation

Linear-Leios needs a transaction-fee mechanism that handles congestion well, gives a useful urgency signal, and remains simple enough to implement and validate. Today's flat fees give users no way to signal urgency; under congestion, urgent transactions lose value with no recourse.

Phase-2 pivoted away from full tiering toward a narrower "dynamic pricing" scope after product and community feedback. The candidate set below is the result of that down-select: a single-lane dynamic fee mechanism and three two-lane mechanisms that pair a priority lane with a standard lane. The flat-fee status quo serves as the baseline against which these candidates are evaluated; it is not itself a candidate.

All live candidates preserve Cardano's existing additive fee structure (`minFeeB + minFeeA × bytes`). Dynamic adjustment lives entirely in the per-byte coefficient. The constant `minFeeB` is unchanged across every live mechanism.

## Design-space matrix

The phase-2 design space is two axes: how many lanes are dynamic, and whether priority is enforced via an on-ledger partition.

| | **RB-reserved partition** | **No partition** |
|---|---|---|
| **Single-lane (one dynamic coefficient)** | n/a — no priority lane | ✓ live: [Single-lane EIP-1559](#single-lane-eip-1559) |
| **Two-lane priority-only (static standard)** | ✓ live: [RB-reserved priority-only premium](#rb-reserved-priority-only-premium) | ✓ live: [Un-reserved priority-only premium](#un-reserved-priority-only-premium) |
| **Two-lane both-dynamic** | ✓ live: [Both-dynamic, partitioned variant](#both-dynamic) | ✓ live: [Both-dynamic, un-partitioned variant](#both-dynamic) |
| **Two-lane fully-static (both lanes static)** | preliminary discard | preliminary discard |
| **Tiered** | out of phase-2 scope | out of phase-2 scope |

The "RB-reserved partition" axis means: the Ranking Block is definitionally priority-only, an Endorser Block has a logical priority partition of one RB-worth (~90KB), and the priority partition is activated only when the EB is at capacity. Without the partition, priority is delivered through producer-side `priority_first` block-build ordering; service is preferential but not on-chain-validated.

Both-dynamic appears as one row with two cells because the partition decision is orthogonal to whether both lanes are dynamic; the spec presents both forms as live candidates.

## Common design choices

These apply to every live mechanism unless the per-mechanism section explicitly overrides.

### EIP-1559 maximum-fee semantics

Each transaction's existing `fee` field is interpreted as the maximum total lovelace amount the transaction authorises the ledger to collect. This document refers to that value as `maxFee` when discussing EIP-1559 semantics, but it is not a separate transaction field.

For validity, the current quoted fee for the transaction's posted lane must fit under that maximum:

$$\text{minFeeB} + \text{currentQuotePerByte}(\text{postedLane}) \cdot \text{bytes} \le \text{maxFee}$$

In single-lane, `postedLane` is the single dynamic lane. In two-lane mechanisms, the transaction's priority/standard intent determines which lane quote is checked.

- **Invalidation.** If the current quoted fee moves above a transaction's `maxFee` while the transaction sits in mempool, the transaction is invalid for inclusion and is evicted. This is the natural mempool-draining mechanism: users who underestimated quote drift are removed and can resubmit with a higher fee-field value.
- **Inclusion fee.** When a transaction is included, it pays the *current* quote at inclusion, not its `maxFee`.
- **Difference refund.** The difference `maxFee − actual fee` is refunded to the user via the fee-change return mechanism (Polina's separate CIP). The refund is per-transaction.

This is the standard EIP-1559 pattern: users post a maximum authorised payment, pay the current price, get refunded the gap. Invalidation under `maxFee` replaces the prior "never-stale" proposal; the perverse mempool-clog argument that motivated never-stale doesn't apply because invalidation itself drains stale entries.

### Difference-refund mechanism

The fee-change return mechanism (Polina's CIP, separate deliverable) refunds the difference between the transaction's maximum authorised fee (`maxFee`, carried in the existing `fee` field) and the actual fee at inclusion. For two-lane mechanisms it also handles the lane-mismatch refund: a priority-fee-paying transaction that lands in standard space (because the EB was below capacity, or because the priority partition was already full) is refunded down to the standard fee. Both refund paths use the same mechanism.

### Finite mempool cap

Per-node mempool capped at a finite byte budget. Default cap: `2 × max block body size`, matching today's mainnet convention. New arrivals beyond the cap are not admitted at the mempool layer. There is no protocol-level rejection message; users observe their transactions' presence in node mempools to detect non-admission.

### Era floor

Two protocol-fixed constants underlie every mechanism:

- `minFeeA = 44 lovelace/byte`: the minimum per-byte rate. The per-byte rate paid by any transaction is bounded at or above this floor.
- `minFeeB = 155,381 lovelace/transaction`: an additive constant. It is paid identically by every transaction across every mechanism, never multiplied by any dynamic coefficient.

The per-byte rate is the dynamic part. The constant is the static part.

### Notation

Every transaction's fee is expressed as

$$\text{fee} = \text{minFeeB} + c \cdot \text{minFeeA} \cdot \text{bytes}$$

where `c` is a dimensionless **fee coefficient**. `c = 1` is today's flat-fee Cardano rate (`minFeeB + minFeeA × bytes`). `c > 1` raises the per-byte rate; the era floor enforces `c ≥ 1` (so the per-byte rate never drops below `minFeeA`).

Each mechanism specifies how `c` is determined and bounded. In single-lane, there is one `c` for everyone. In two-lane, there is one `c` per lane (`c_standard`, `c_priority`), with the cross-lane invariant `c_priority ≥ multiplier_floor × c_standard`.

### Controller signal — capacity-weighted utilisation over a window

**Placeholder design — best current intuition.** This applies to controllers whose "relevant capacity" varies across priced blocks — single-lane EIP-1559 most importantly, and the un-reserved priority-only and both-dynamic candidates depending on how their open signal questions resolve. Two-lane priority controllers operating against a uniform per-block priority partition (RB-reserved variant) do not need this framework; they use a simpler per-block normalised utilisation defined in the per-mechanism section.

Linear-Leios produces priced blocks of very different sizes — RBs cap at ~90KB; EBs at ~12MB. The standard EIP-1559 single-block fill rate (`block_bytes / block_capacity`) doesn't generalise cleanly: an update fired on an RB and an update fired on an EB can produce wildly different signals from the same demand. Where capacities differ, the spec feeds the EIP-1559 controller a *capacity-weighted aggregate utilisation* over a recent window of priced blocks:

$$\text{aggregateUtil} = \frac{\sum_{b \in W} \text{relevantBytes}(b)}{\sum_{b \in W} \text{relevantCapacity}(b)}$$

where `W` is the most recent window of priced blocks. Each block contributes its relevant bytes to the numerator and its relevant capacity to the denominator. The per-mechanism section specifies what counts as "relevant"; for *Single-lane EIP-1559*, it is total committed transaction bytes against block body capacity.

**In plain English.** Add up all the transaction bytes that have been delivered across the last `N` priced blocks. Add up all the block-space capacity those same blocks offered. Divide the first by the second. The result is the fraction of recently-available block space that actually got used: `0.0` means the window's blocks were empty, `0.5` means the network has been running half-full on average across the window, `1.0` means every block in the window was saturated. The controller compares this fraction against its `target` and ratchets the price up or down accordingly.

The aggregate is a value in `[0, 1]` and is fed to the EIP-1559 update rule defined under *Single-lane EIP-1559* below in place of an immediate per-block fill rate.

**Why capacity-weighting.** A saturated 12MB EB and a saturated 90KB RB both register as "fully utilised" against their own capacities, and capacity weighting blends them proportionally — large blocks dominate the sum where they exist, but small blocks still register their contribution. Endorsement-only RBs (carrying a cert, no own txs) contribute 0 bytes to the numerator and their body capacity to the denominator: a proportional drag rather than a binary outlier.

**Alternatives considered.** Each was found to have a specific weakness with the linear-Leios block-mix:

- *Immediate per-block fill rate* (`block_bytes / block_capacity`, fired per priced block, no aggregation). Same demand produces wildly different signals depending on which block type fired: 90KB delivered on a 90KB RB reads `fillRate = 1.0` (max upward step); the same 90KB delivered on a 12MB EB reads `fillRate ≈ 0.0075` (max downward step). The controller bounces around block-type cadence rather than tracking demand.
- *Per-RB settlement-step aggregation against a fixed `targetBytesPerRb`*. Aggregates RB-own-bytes plus the certified EB's bytes per RB and compares to a single configured target. Solves the cadence problem of the previous alternative but introduces a different variance source: settlement-step size is bimodal because a tx-bearing RB delivers ≤90KB while an endorsement-only RB whose certified EB is full delivers ≤12MB. Calibrating `targetBytesPerRb` to expected mean throughput (say, ~6MB) makes a 100%-full tx-bearing RB read as `fillRate ≈ 0.015` and the controller pushes the price *down* on a fully-saturated block.
- *EMA over per-block utilisations*. Each priced block contributes its `[0, 1]` fill rate to an EMA. Doesn't address block-size leakage: a 100%-full 90KB RB and a 100%-full 12MB EB both contribute `1.0` to the EMA with equal weight, even though the EB delivered ~130× more bytes. A window of mostly-small-blocks-at-saturation and a window of mostly-large-blocks-at-saturation read the same on the EMA but represent very different absolute throughput. Adds a smoothing-factor calibration burden on top.
- *Median over per-block utilisations*. Robust to outliers — cert-only RBs at 0 are filtered as a minority. But doesn't address block-size leakage: a 100%-full 90KB RB and a 50%-full 12MB EB both contribute utilisation values to the order statistic with equal weight, so the median's response shifts based on block-mix even when bytes-delivered is constant. Also has step-wise response (small persistent shifts may not move the median across the window).

Capacity-weighted aggregation addresses each: cadence is smoothed by the window (vs immediate per-block); the settlement-step's bimodality is replaced with per-block proportional contribution (vs fixed `targetBytesPerRb`); and block-size differences are correctly weighted into the aggregate (vs EMA's and median's equal-weight averaging of utilisations). It does sacrifice some of median's outlier-robustness — a single anomalous block does perturb the aggregate, in proportion to its capacity — which is why an additional smoothing layer (EMA, median, or other estimator on top of the capacity-weighted aggregate) is left as a calibration choice.

**Open calibration choices.** Window length, update cadence (per priced block, per RB, or per epoch), the precise definition of "priced block" (whether to include endorsement-only RBs, or only RBs and EBs that delivered transactions to the canonical chain), and whether to layer additional smoothing (EMA, median, or other robust estimator over the aggregate) are all calibration questions deferred to *Open questions*.

## Live mechanisms

### Single-lane EIP-1559

A single dynamic fee coefficient `c` applied uniformly to all transactions.

**Fee:** `minFeeB + c × minFeeA × bytes`, with `c ≥ 1`.

The coefficient `c` is held in ledger state and updated deterministically.

**Controller signal.** Compute `aggregateUtil` per *Common design choices: Controller signal* with `relevantBytes(b) = block_bytes(b)` (total committed transaction bytes in block `b`) and `relevantCapacity(b) = block_capacity(b)` (block body capacity).

**Controller update rule.** Step:

$$c \leftarrow c \cdot \left(1 + \text{clamp}\left(\frac{\text{aggregateUtil} - \text{target}}{\text{target} \cdot D}, -\frac{1}{D}, \frac{1}{D}\right)\right)$$

floored at 1.

**In plain English.** Look at how far the aggregate utilisation is from the target, expressed as a fraction of the target itself: that's the *signal*. Divide the signal by `D`, the *max-change denominator*. The result is the fractional move the price would make, capped on both sides at `±1/D` so a single step can never move the price more than `1/D` of its current value (`12.5%` at `D = 8`). Multiply the current coefficient by `1 + that fractional move`. If utilisation is at target, the signal is zero and the price doesn't move; above target, it ratchets up; below target, it ratchets down.

**Why the clamp is needed.** With a target of `0.5`, an empty block sits exactly as far *below* the target as a full block sits *above* it. The controller responds with equal-magnitude moves on both sides — symmetric, no clamp needed. (This is what Ethereum picks; their clamp would never engage.)

With a target of `0.25` instead, a full block sits three times further above the target than an empty block sits below it. Without the clamp, the controller would push the price up three times harder on saturation than it pushes the price down on empty blocks — over-reactive on the way up.

The `±1/D` clamp restores symmetry: for whichever target the deployment picks, the per-step price move is bounded equally on both sides at `1/D` of the current price.

**Why allow target ≠ 0.5 at all?** A deployment's choice of target is a choice about what equilibrium load the controller aims at. `0.5` (Ethereum-style) keeps blocks half-full on average — fine for steady demand. A lower target keeps blocks emptier on average, leaving headroom to absorb demand spikes before prices climb. A higher target runs closer to saturation, accepting more frequent congestion in exchange for higher steady-state throughput. Different lanes can also reasonably want different targets: a priority lane might aim low so priority service is rarely contested, while the standard lane runs at `0.5`. The spec leaves the target choice open per *Open questions*; the clamp is unconditional in the rule so that any deployment-side choice is safe.

**Calibration parameters:** `target`, `D`, initial `c`. See *Calibration vs invariant*.

**Maximum fee, invalidation, refund:** all standard per *Common design choices*. There is no lane choice; everyone pays the same coefficient.

**Properties.**

- Aggregate welfare comparable to flat-fee under moderate demand, with prices that adapt to load.
- Single user-facing knob: wallets and explorers handle the coefficient like today's `minFeeA`.
- *Coarse* urgency signalling. At any given moment, all included transactions pay the same per-byte rate — there is no within-moment discrimination among them. But each user's choice of `maxFee` is itself a binary engage/don't-engage signal: a user posting a high `maxFee` stays in the mempool through price spikes (signalling "this is urgent enough to ride out the congestion"); a user posting a low `maxFee` gets invalidated and drops out (signalling "not urgent enough to pay through this"). Less expressive than an explicit priority lane, but not nothing — and it disappears entirely under flat fees.
- The simplest live candidate. One controller, one price, one user-facing knob — less to implement, audit, calibrate, and surface to users than any of the two-lane variants.

### RB-reserved priority-only premium

Two lanes: a static standard lane at today's Cardano fees, and a dynamic priority lane with a multiplier-floor premium. An on-ledger partition rule enforces the lane separation, which prevents producers from being bribed to drop priority transactions for standard-fee replacements. The mechanism canonically assumes `priority_first` block-build scan order; see *FIFO fallback* for what happens if real-mempool constraints force FIFO instead.

**Fee:**

- Standard transactions: `c_standard = 1`, fee = `minFeeB + minFeeA × bytes`. Unchanged from today.
- Priority transactions: `c_priority ≥ multiplier_floor`, fee = `minFeeB + c_priority × minFeeA × bytes`.

`c_priority` is dynamic; it moves with the priority controller. `c_standard` is fixed at 1. The multiplier floor (default 16, configurable) gives a price-discrimination guarantee: the priority per-byte rate is always at least `multiplier_floor` times the standard per-byte rate, never more lenient.

**On-ledger partition rule.**

- **Ranking Blocks**: definitionally priority-only. Every transaction in an RB must have posted a priority-fee. A standard-fee transaction in an RB makes the block invalid.
- **Endorser Blocks**: a logical priority partition of size `priority_reservation_bytes = max_block_size` (one RB-worth, ~90KB). The partition is *activated* only when the EB is at capacity:
  - **EB at capacity**: the priority partition holds priority-fee transactions up to one RB-worth. Priority transactions that don't fit in the partition (overflow) and standard-fee transactions go in standard space.
  - **EB below capacity**: the priority partition is not used. All transactions, including priority-fee transactions, go in standard space.

When a priority-fee transaction lands in standard space — whether from EB-below-capacity or priority-partition overflow — it is refunded down to the standard fee per *Common design choices*. The transaction collected only standard service, so it pays only standard fee.

The partition is currently described as a *logical / tag-based* grouping (placeholder pending Polina's review of representation). Whether the tag is an explicit transaction field or an implicit grouping derivable from block contents is an open question; the design is independent of representation.

**Priority controller signal.** Demand-driven, not delivery-driven. The signal is

$$\text{priorityUtil}(b) = \frac{\min(\text{priority\_paying\_bytes}(b), \text{priority\_reservation\_bytes})}{\text{priority\_reservation\_bytes}}$$

per priced block `b`, where `priority_paying_bytes(b)` includes priority-fee transactions even if they were refunded (refunded priority transactions still drove demand; the controller responds).

**In plain English.** For each priced block, count the bytes of transactions that posted a priority fee — including any that ended up getting refunded to standard because the priority partition wasn't activated for that block. Cap that count at one RB-worth (the partition's maximum capacity per block; no more priority service can be sold than that). Divide by one RB-worth. The result is in `[0, 1]`: `0.0` means no priority demand showed up on this block, `1.0` means priority demand met or exceeded the partition's capacity. The controller compares this against `target_priority` and ratchets `c_priority` up or down.

The capacity-weighting and windowing required for single-lane EIP-1559 are not needed here. Every priced block — RB or EB — offers exactly one RB-worth of priority service capacity (the entire RB body, or the EB's priority partition), so the per-block utilisation is already normalised to `[0, 1]` regardless of block type. Once priority demand exceeds `priority_reservation_bytes` in a block, the signal saturates at 1.0; below that, it tracks demand linearly.

- A tx-bearing RB contributes `RB_bytes / priority_reservation_bytes` (the RB is entirely priority-only, so all RB bytes are priority bytes; saturates at 1.0 when the RB is full).
- A certified EB contributes `min(priority_paying_bytes, priority_reservation_bytes) / priority_reservation_bytes` — the priority partition's utilisation, with overflow priority-paying bytes (which spill into standard space and are refunded) still counted up to the partition's capacity.
- Endorsement-only RBs do not fire a controller event — their certified EB does, separately, when applied.

The controller updates `c_priority` per the EIP-1559 rule from *Single-lane EIP-1559* with denominator `D_priority` and target `target_priority`, feeding `priorityUtil(b)` in place of `aggregateUtil`. The multiplier floor `c_priority ≥ multiplier_floor` is enforced after each update.

**Standard controller.** Static. `c_standard = 1` always; the standard side does not adapt to load.

**Anti-bribery property.** The RB partition rule is on-chain-enforceable. A producer cannot drop a priority-fee transaction to free space for a standard-fee one within an RB without producing an invalid block. This bounds the producer's incentive to take side-payments in exchange for transaction position. The EB partition rule extends the property to one RB-worth of EB space whenever the EB is at capacity.

**Calibration parameters:** `target_priority`, `D_priority`, initial `c_priority`, `multiplier_floor`, `priority_reservation_bytes` (set to `max_block_size` in the recommended configuration; experimentally relaxable).

**Properties.**

- Strong urgency separation under congestion: high-urgency users self-select into priority and receive faster, more reliable inclusion via the RB partition.
- Static standard preserves today's Cardano fee experience for non-priority users (UX argument).
- Anti-bribery via the RB partition rule.
- One RB-worth of guaranteed priority service per slot opportunity, via either the RB itself or an EB partition.

### Un-reserved priority-only premium

Same fee structure and refund/invalidation rules as the RB-reserved variant, but without the partition.

**On-ledger validation:** none. There is no RB-priority-only rule and no EB priority partition. Standard and priority transactions share blocks freely.

**Priority delivery:** producer-side block-build ordering. Compliant producers scan the mempool with `priority_first` semantics — priority-fee transactions are admitted to the working block before standard-fee transactions in the same scan. There is no on-chain check that they did so.

**Refund:** the EB-fullness-conditional rule does not apply. A priority-fee transaction that is included pays the current priority quote, with refund per the maximum-fee rule. There is no notion of "wrong space" — there's no partition to be in or out of.

**Priority controller signal: open question.** Without a partition, "priority utilisation" needs a definition. Three candidates the spec presents as options:

1. *Priority-fee bytes / total block capacity*: priority's share of delivered bytes.
2. *Priority-fee bytes / target priority share*: a notional priority share parameter, even without an enforced partition.
3. *Observed delay-gap signal*: `standard_delay_ema − priority_delay_ema`, with utilisation as a diagnostic only.

The decision is left to a follow-up design pass.

**Anti-bribery property:** absent. Priority protection comes only from the producer's incentive to grab the higher fee, plus `priority_first` ordering in compliant producers. Side-payments to drop priority transactions are not on-chain-detectable.

**Properties.**

- Soft priority lane: economic differentiation without on-chain enforcement.
- Less mechanism surface than the RB-reserved variant — no partition rule, no fullness-conditional refund.
- Suitable for deployments that want urgency signalling but cannot accept the partition rule's implementation cost or operational complexity.

### Both-dynamic

Two dynamic controllers — one for the priority lane, one for the standard lane — with the cross-lane multiplier-floor invariant.

**Fee:**

- Standard: `c_standard ≥ 1`, fee = `minFeeB + c_standard × minFeeA × bytes`.
- Priority: `c_priority ≥ multiplier_floor × c_standard`, fee = `minFeeB + c_priority × minFeeA × bytes`.

Both coefficients move dynamically. The cross-lane invariant is enforced after every controller update: priority's per-byte rate is always at least `multiplier_floor` times standard's. With both lanes dynamic, the standard side also responds to load — preserving today's flat-fee experience is sacrificed in exchange for fuller load adaptation.

**Maximum-fee semantics apply to both lanes.** A standard-fee transaction and a priority-fee transaction each carry one maximum authorised fee value in the existing transaction `fee` field. The transaction's posted lane determines which quote is checked while it sits in mempool. It is invalidated if that lane's current quoted fee rises above its `maxFee`, and it is refunded the difference between `maxFee` and the actual fee charged at inclusion.

**Partition is an orthogonal axis.** Both-dynamic can be deployed with the RB-reserved partition (matching the RB-reserved priority-only mechanism's partition rules) or without (matching the un-reserved priority-only mechanism's lack of partition). Both forms are live candidates. The choice between them is independent of whether the standard side is dynamic.

**Controller signal: open question for both lanes.** The priority signal source is the same open question as for the un-reserved variant when no partition is used; with a partition, the same demand-driven RB-equivalent yardstick as the RB-reserved variant applies. The standard signal source is also open: standard-fee bytes against what denominator (total block capacity, or a notional standard share, or some other measure)?

**Calibration parameters:** per-controller `target`, `D`, initial coefficient; `multiplier_floor`; partition parameters (when applicable, as in the RB-reserved variant).

**Properties.**

- Full load adaptation on both sides — standard fees move with load too, not just priority.
- Two user-facing prices and lane-aware fee-cap guidance. More wallet/explorer surface.
- Multiplier-floor invariant guarantees price discrimination across load regimes.
- The community-preference argument against a dynamic standard lane (no guaranteed fixed-fee path for non-priority users) is the main reason this is not the leading candidate. Kept as a candidate pending further evidence.

## Discarded variants

### Two-lane fully-static (preliminary discard)

Both lanes static prices, no controllers. The simplest possible two-lane mechanism: priority and standard prices fixed at genesis, with the multiplier-floor invariant maintained statically.

Discarded preliminarily because it cannot adapt to load: under any demand regime that differs from the calibration target, the mechanism either over- or under-prices. Earlier experimental data quantifying this is invalidated by the change to EIP-1559 maximum-fee semantics (the prior runs assumed never-stale validation). Updated experimental comparison is pending.

Kept documented because it remains the simplest baseline against which the dynamic variants must justify their additional surface.

### Tiered

Full tiered pricing — multiple tiers with delays scaling as `2^k` and per-tier dynamic prices — was the phase-1 candidate. After the phase-2 community pivot toward simpler dynamic pricing, the full tiered mechanism is out of phase-2 scope. See the phase-1 writeup for design details and rationale.

## FIFO fallback

The live two-lane mechanisms above all assume `priority_first` block-build scan order: when a producer assembles a block, priority-fee transactions are scanned and admitted before standard-fee transactions. This is the canonical ordering.

Real Cardano mempool implementations may force FIFO scan order for engineering reasons (admission cost, gossip-layer interactions, validator state-machine constraints). Under FIFO, priority transactions are scanned in arrival order alongside standard transactions; an early-arrived standard transaction can fill block space that a later-arrived priority transaction would otherwise have used.

If FIFO is forced, an **anti-standard cap** reappears in the design as a priority-protection mechanism: a configured upper bound on the fraction of each block that can be filled by standard-fee transactions, leaving the remainder for priority transactions even if priority transactions arrived later. The cap value is configurable; one-RB-worth (`max_block_size`) is a natural symmetric choice with the priority partition.

The anti-standard cap is *only* introduced under the FIFO fallback. Under `priority_first`, priority is protected by scan order; the cap is unnecessary and would only throttle aggregate throughput when priority is unused.

## Open questions

- **Intent vs fee.** Does a transaction have a mechanism to signal its intent to be priority *other than* the fee it offers? An explicit `lane: priority | standard` tag, or a priority-intent flag, would let the controller see refunded priority bytes without ambiguity, and would clarify what happens in the gap between standard fee and minimum priority fee. May share representation with the partition tag from Polina's design pass.
- **Resubmission semantics.** Can a user replace a standard-fee transaction with a priority-fee transaction (consuming the same UTxO inputs) before either lands on chain? How do node mempools resolve double-attempts that consume the same input? Cardano's UTxO model permits multiple conflicting transactions in flight; mempool resolution policy is not yet specified.
- **Priority controller signal source for un-reserved priority-only premium and both-dynamic mechanisms.** Three candidate signals are listed in the per-mechanism sections; choice deferred.
- **Standard controller signal source for both-dynamic.** Symmetric to the priority question above.
- **Standard side relaxation in the RB-reserved variant.** The static-standard rule is primarily a UX argument (preserve today's guaranteed fixed-fee path). If both-dynamic experiments show clear value and community sentiment shifts, relaxing the RB-reserved variant to a dynamic standard with an extended refund rule may be warranted.
- **Controller signal: capacity-weighted utilisation calibration.** The capacity-weighted aggregate over a window is a placeholder design. Open: window length; update cadence (per priced block, per RB, per epoch); whether to include endorsement-only RBs in the window or restrict to RBs and EBs that delivered transactions; whether to layer additional smoothing (EMA over the aggregate, median, or a robust estimator) on top of the capacity weighting. Validation pending against representative load.

## Calibration vs invariant

| Parameter | Status | Notes |
|---|---|---|
| `minFeeA = 44 lov/byte` | Protocol invariant | Era floor; bounds dynamic coefficients from below |
| `minFeeB = 155,381 lov/tx` | Protocol invariant | Additive constant; never multiplied; identical across mechanisms |
| `max_block_size` | Protocol invariant | Cardano network parameter; defines RB capacity |
| `priority_reservation_bytes` | Spec invariant for RB-reserved | Set to `max_block_size`; experimentally relaxable |
| `multiplier_floor` | Calibration parameter (configurable) | Default 16; enforces priority-vs-standard price discrimination |
| `target_utilisation` per controller | Calibration parameter | EIP-1559 controller target fill rate |
| `max_change_denominator` (`D`) per controller | Calibration parameter | EIP-1559 per-step price-change cap, `±1/D` |
| `initial_coefficient` per controller | Calibration parameter | Starting per-byte coefficient |
| Mempool cap | Per-node configuration | Default `2 × max_block_body_size`; mainnet convention |
| `priority_first` scan order | Spec invariant for live two-lane mechanisms | Falls back to FIFO + anti-standard cap if real-mempool constraints force it |
| Controller-signal window length | Calibration parameter (placeholder) | Length of the capacity-weighted aggregate window; pending calibration. See *Open questions*. |
| Controller update cadence | Calibration parameter (placeholder) | Per priced block / per RB / per epoch; pending calibration. See *Open questions*. |

Calibration parameters should be set per deployment by sweep against representative load. None should be hard-coded constants in the protocol spec.

## Methodology: simulator approximations

The simulator at `sim-rs/` is the primary phase-2 evidence-generation tool. The clean-room rebuild on the `dynamic-experiment` branch ([implementation-plan.md](implementation-plan.md)) brings the simulator into spec alignment for every divergence except one. The table below records the row-by-row resolution: of the 8 simulator approximations originally documented, **7 are resolved by the M1-M4 rebuild and 1 remains** (anti-standard cap under FIFO fallback). Resolved entries point at the milestone whose handoff describes the implementation.

| Spec design | Status |
|---|---|
| EIP-1559 maximum-fee semantics: invalidation if the current quoted fee rises above the transaction's fee-field value; difference refund on inclusion | **Resolved at M1** — `MempoolGate` admission/revalidation/inclusion path. See [m1-handoff.md](m1-handoff.md) §"What M1 delivered". |
| EB priority partition activated only when EB is at capacity (binary trigger) | **Resolved at M2** — saturation + capacity-bound binary trigger in `select_eb_with_partition`; M3 made it the production path with `partition_activated` carried on the EB. See [m2-handoff.md](m2-handoff.md) §"What M2 delivered" and [m3-handoff.md](m3-handoff.md) §"Decisions M3 made" (consolidation row). |
| Per-tx refund on lane mismatch and fee-field value minus actual-fee difference | **Resolved at M1+M2** — `actual_fee_lovelace` and `refund_lovelace` are emitted on `Event::TXIncluded` directly, no metrics-layer relabelling. See [m1-handoff.md](m1-handoff.md) and [m2-handoff.md](m2-handoff.md). |
| RB partition definitionally priority-only | **Resolved at M2** — `LaneValidityRule::PriorityOnly` for RB-reserved variants; standard-fee txs in such an RB make the block invalid. See [m2-handoff.md](m2-handoff.md). |
| Priority partition size = one RB-worth | **Resolved at M2** — `PricingBackend::samples_for_block` per-variant override caps the EB priority numerator at `min(priority_bytes, max_block_size)` against `relevant_capacity = max_block_size`. See [m2-handoff.md](m2-handoff.md). |
| Anti-standard cap absent under `priority_first` | **Partially resolved**: simulator carries no `max_standard_block_fraction` knob; under `priority_first` selection no cap exists, matching the spec. **The remaining gap is the FIFO branch** — the spec mandates an anti-standard cap under `LaneSelectionOrder::Fifo`, but no FIFO suite is authored on `dynamic-experiment` and the simulator has no cap implementation. This is the single residual divergence; it would only need to be implemented before any FIFO experiment is run. See [implementation-plan.md §"Spec gaps deferred"](implementation-plan.md#L20). |
| Logical / tag-based partition (placeholder) | **Resolved at M1+M2** — per-tx `posted_lane: Lane` field on `Transaction`, with no separate tier vocabulary; the spec at line 166 says the design is independent of representation. |
| Capacity-weighted aggregate utilisation over a window for single-lane EIP-1559 | **Resolved at M1** — `CapacityWeightedWindow` (default length 32 for capacity-varying signals, length 1 for RB-reserved priority controllers, which mathematically reduces to per-block fill rate). See [m1-handoff.md](m1-handoff.md). |

The discarded fully-static variant's invalidation/refund model is now consistent with the spec via the same `MempoolGate` path that resolved row 1; re-running the static variant against the rebuilt simulator is welfare-comparable.

## Calibration choices

The spec leaves several calibration parameters open ([§Open questions](#open-questions)). The simulator picks concrete defaults so the suites have something to run; these are *calibration choices*, not spec divergences. Each is documented here with the value the simulator picked, the spec section it answers, and a forward-pointer to the cost of re-calibrating.

| Choice | Value | Spec section | Re-calibration cost |
|---|---|---|---|
| Window length (capacity-varying signals) | 32 | [§"Open calibration choices"](#open-questions) (window length) | Edit `window-length` in the relevant pricing YAML; M5 suite goldens flip; regenerate via `UPDATE_GOLDENS=1` and re-tag. |
| Window length (RB-reserved priority controller) | 1 (reduces to per-block fill rate) | [§"Open calibration choices"](#open-questions) and the RB-reserved priority signal at line 170 | Same as above; raising it above 1 introduces unnecessary smoothing because RB-reserved priority capacity is uniform per block. |
| Update cadence | Per priced block | [§"Open calibration choices"](#open-questions) (cadence) | Per-RB or per-epoch alternatives require a rewrite of the simulator's `apply_priced_block`/`apply_eb_priced_block` paths; intrusive. |
| Un-reserved priority signal source | Option 1: `priority_paying_bytes / total_block_capacity` | [§"Un-reserved priority-only premium"](#un-reserved-priority-only-premium), lines 207-211 (open-question framing + the three options) | Option 2 (notional priority share) needs a new config knob. Option 3 (delay-gap) is a controller-rewrite. |
| Both-dynamic standard signal source | Capacity-weighted aggregate of `standard_paying_bytes` against `eb_referenced_txs_max_size_bytes` for EBs; **no standard sample fires on RB-reserved RBs** (RB capacity is dedicated to priority, so RB traffic must not move standard pricing) | [§"Both-dynamic"](#both-dynamic), line 238 | Change `relevant_capacity` formula in `PricingBackend::samples_for_block`. |
| Default actor `max_fee_policy` | `ScaledOverLaneQuote { numerator: 4, denominator: 1 }` — i.e. 4× quote-drift headroom on `max_fee_lovelace` | [§"Open questions"](#open-questions) (Intent vs fee, partial) | Per-component override in a demand YAML. M4's `paper_like_mispriced.yaml` already does this for the high-urgency component (`{1, 1}`, zero headroom). |
| `multiplier_floor` in `phase-2-rb-scarcity` and `phase-2-urgency-inversion` | 4 (rather than the default 16 used by other suites' `*_x16` jobs) | [§"Calibration vs invariant"](#calibration-vs-invariant) | Raise the floor in those suites' pricing YAMLs; signals will weaken because fewer urgency components self-select into priority. At 16, priority demand stays too low to drift the controller within the run. Full rationale in [m4-handoff.md §Decisions M4 made](m4-handoff.md) and [calibration-fix-postmortem.md](calibration-fix-postmortem.md). |
| `rb-generation-probability` and `default-slots` | 0.05 (Cardano-realistic) and 1000 respectively | linear-Leios endorsement window: spec is silent, but `try_generate_rb`'s `earliest_endorse_time` check requires `slot ≥ parent_rb.slot + header_diffusion × 3 + linear_vote_stage_length + linear_diffuse_stage_length` (≈ 13 slots). Probability 0.05 gives ~20-slot expected RB cadence, clearing the window. | An earlier revision pinned `rb-prob = 1.0` for uniform tx-bearing-block-per-slot output; that combination prevented EB endorsement entirely. See [calibration-fix-postmortem.md](calibration-fix-postmortem.md) for the full explanation. *Re-calibrating*: any rb-prob ≥ ~0.077 reintroduces the bug (median RB gap drops below 13 slots); paired with `topology-single-producer.yaml`'s `stake: 100000` to preserve probability under VRF stake quantization. |
| Default actor `target_inclusion_blocks` | priority = 1, standard = 4 (seeds the per-(component, lane) `LatencyEstimator`; observed latencies overwrite the seed once inclusion events arrive) | [§"Open questions"](#open-questions) | Per-component override in the demand YAML. |
| Mempool cap default | `2 × eb_referenced_txs_max_size_bytes` — the simulator's interpretation of the spec's "max block body size" in linear-Leios | Line 59 ("Finite mempool cap") | Set `mempool-max-total-size-bytes` in `protocol-base.yaml`. |
