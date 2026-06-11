Objective: Gain clear data about the impacts of different dynamic pricing implementations on linear-Leios without the complexity overhead of network simulation.

Why?: The Leios `sim-rs` project, while useful in directing us towards or against certain design spaces, is still under active upstream development, with bugs being fixed and components being altered. As such, referencing precise data derived from any simulation based on it could be seen as, at least to some degree, disingenuous. We do want to include precise data in our CIP for urgency signalling, and as such, looking at urgency signalling in the abstract is more representative.

What?: A simulator of `n` (configurable) nodes, running linear-Leios, with the network abstracted. Our candidate designs:

None (as control)
EIP-1559 (as control)
Two lane: Both dynamically priced
Two lane: Priority dynamic only

We should have reservation/no-reservation as an axis, at least for priority, and possibly for standard transactions.

Within those, we may have variations. For example, Giorgos' recommended design is as follows:

"Priority and standard always pay standard price in EBs. Only priority transactions are allowed in RBs. Priority transactions in EBs count towards priority price: if 1RB's worth of space is taken up, max fill.

Both dynamic.

If we must have fixed price, EB included txs always just pay fixed price. No space reserved."

EB generation and inclusion should be a random number generator based on figures from [the CIP](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0164). All non-CIP-specified factors should be assumed to be based on current Praos. Transactions should also model Plutus scripts. As such, fee price is based on a multiple (according to the current dynamic pricing coefficient) of the entire price, not just size bytes `(tier .TxTier.tierCoeff) * (minfee pp utxo tx) ≤ txFee`. Block fullness should be determined as either the CIP (if specified and different to current Cardano) or current Cardano.

Configurable actors, who can submit transactions. The actors should act rationally, based on fee pricing, transaction urgency, and transaction value. Actors should have different behavioural categories, something like:
* Honest
* Dishonest
* Arbitrage/MEV/front-running attempts

The distribution of actors' transaction size should be configurable by way of a curve, for ease.
Plutus script size should, separately, also be configurable by way of a curve.
Ex-units should, separately, also be configurable by way of a curve.

Transaction dependency chains should be modelled.

Staleness behaviour:

In all dynamic cases, the transaction is expected to post a fee that the node will consider "stable"; the true fee at the time of inclusion should not be likely to rise to above the posted fee, lest the containing EB becomes invalid.

Metrics:

Transaction inclusion, by urgency
Retained value/lost value, by urgency
Inclusion latency, by urgency
Price shock
Revenue/fees + refunds
Aggregate throughput / EB utilization
Price convergence/oscillation 
Invariant breaches

Mempool:

Transactions are admitted if the posted fee is considered appropriate (unlikely to be out-priced by the dynamic mechanism before inclusion), and they pass the current Cardano mempool checks.

The cap is as described in the Leios CIP (2x EB size).

Transactions that have become stale for any reason are evicted from the mempool at the time of block construction.

EBs that happen to contain stale transactions will fail the "validation" (RNG determines if an EB is eligible for inclusion, then its txs are checked for staleness).

EB certification and inclusion:

This pins the inclusion RNG above. The CIP rule: an EB announced by an RB is certified and included only if the next RB on the chain lands at least `D = 3·L_hdr + L_vote + L_diff` slots later; if the next RB comes sooner, that EB is discarded and the next RB simply announces its own fresh EB. Those three terms are the certification phases in order: `3·L_hdr` rules out an equivocating header before voting opens, `L_vote` is the committee's voting window, and `L_diff` lets the certificate diffuse network-wide - only after all three is the cert safe to reference. The discarded EB's txs were never removed from the mempool in the first place (a tx is only removed once it's on-chain - in an RB body or via a certified, endorsed EB), so they stay available with no re-queuing needed. The CIP gives this formula and the constraint `3·L_hdr + L_vote > Δ_EB` (a voter must receive *and fully validate* the EB inside the `3·L_hdr + L_vote` window); it does not pin `L_hdr`, `L_vote`, `L_diff`, or `f`. `f` comes from current Praos, the stage lengths come from `sim-rs`'s config'.

So the chance an EB is included (`P(included)`) is a function of Praos timing only: `f` and `D`, with no EB-size term. Each slot independently produces an RB with probability `f` (the active-slot coefficient) - a per-slot dice roll. So the next RB landing at least `D` slots away means `D-1` empty slots in a row, giving `P(included) = (1 - f)^(D-1)`. At mainnet `f = 0.05` and our `D = 13` slots (header-diffusion 1s × 3 + `L_vote` 5 + `L_diff` 5), that is `0.95^12 ≈ 0.54`. More frequent RBs (higher `f`) *lower* per-EB inclusion; a smaller `D` raises it. `D` is the full inclusion delay (`3·L_hdr + L_vote + L_diff`), not just the voting stage `L_vote`, so shortening any of its three parts (header diffusion, vote stage, or diffuse stage) shrinks `D`.

Inclusion needs two conditions to hold, but only the first can fail in this model. `P(included) = (1 - f)^(D-1) × q`:
* RB timing, `(1 - f)^(D-1)`: the next RB must land at least `D` slots later. Size-free, set by `f` and `D`. This is the only condition that fails here.
* Quorum, `q`: enough committee votes must be gathered. Set it to 1. The CIP's committee is deterministic stake-based truncation, so quorum is reached unless the votes can't spread across the network fast enough to be counted within `L_vote` - i.e. the network is too congested. We abstract the network, so votes always arrive and quorum always forms. That congestion failure is out of scope here; disclose it as an omission and revisit only if network effects are reintroduced.

So `P(included) = (1 - f)^(D-1)`, full stop.

Size re-enters only through the floor on `D`. A larger or more ex-unit-heavy EB takes longer to receive and validate, so the CIP constraint forces a larger `L_vote`, hence a larger `D`, hence a lower `(1 - f)^(D-1)`. It is a parameter coupling - size sets the minimum viable `D` - not a per-EB random failure. So derive `D` from the receive+validate time of the largest EB we allow (validate = bytes + ex-units), and hold `f` and `D` fixed at those justified values: the experiment sweep is over candidate designs and seeds, not protocol timing parameters.

Don't quote `D = 13` or `0.54` as CIP figures - the CIP gives the rule; the numbers are ours.
