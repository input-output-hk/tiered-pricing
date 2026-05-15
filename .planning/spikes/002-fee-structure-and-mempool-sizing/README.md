# Spike 002 — Fee structure and mempool sizing
Date: 2026-05-13
Verdict: NEEDS-DISCLOSURE

## Spike Question

- **Given** current Cardano mainnet protocol parameters (minFeeA,
  minFeeB, maxTxSize, maxBlockBodySize, mempool sizing) and observed
  user behavior around fee selection,
- **When** phase-2 charges `actual_fee = minFeeB + quote × bytes`
  (no `minFeeA × bytes` term — phase-2's pricing kernel reinterprets
  the per-byte fee as the dynamic `quote`), caps the mempool at
  `2 × eb_referenced_txs_max_size_bytes`, and defaults
  `max_fee_policy = 4 × quote` per actor
  (`ScaledOverLaneQuote { numerator: 4, denominator: 1 }`),
- **Then** per-tx fees, max-fee headroom, and admission-rejection
  rates should reflect plausible mainnet conditions — or, if not,
  the deviations must be acknowledged and bounded.

## Research

Mainnet protocol parameters reused from spike 001 (retrieved
2026-05-13 via the Cardano on-chain `/epoch_params` query at epoch 540, cross-checked at
tip = epoch 630, Conway era, protocol major version 10 pending
Van Rossem promotion to v11):

- `minFeeA`: **44** lovelace/byte
- `minFeeB`: **155381** lovelace
- `maxTxSize`: **16384** bytes
- `maxBlockBodySize`: **90112** bytes (RB body cap)
- `maxBlockHeaderSize`: 1100 bytes
- Slot length: 1 s; activeSlotsCoeff: 0.05

**Cardano-mainnet fee semantics (the key qualitative finding):**

- Mainnet fees are computed deterministically and **exactly** at
  transaction-creation time by the wallet from the formula
  `fee = minFeeA × bytes + minFeeB`. The transaction's `fee` field
  is the **exact** charged amount; there is no `maxFee` /
  `maxFeePerGas` analogue on mainnet. ("Cardano provides exact,
  deterministic fees with no maximum fee parameter needed,
  while Ethereum's EIP-1559 requires users to specify a maxFee as
  a protective mechanism" — Cardano Developer Portal &
  EIP-1559 comparison literature.)
- For a typical 200-byte tx, mainnet fee is therefore exactly
  `44 × 200 + 155381 = 164181 lovelace ≈ 0.164 ADA` — a value
  repeatedly quoted in the official docs and developer portal.
- Wallets (Lace, Eternl, Daedalus, Yoroi) all use
  `cardano-serialization-lib` / `cardano-cli` / `cardano-wallet`
  to compute this exact fee; the user does not enter a fee or a
  buffer. Some libraries internally compute the fee twice
  (a "round trip" — fee depends on bytes, which depend on fee
  field width) but the result that ships is the exact min-fee for
  the final tx shape.
- This is a deliberate UX property of Cardano's eUTxO model and
  is contrasted in IOG marketing material directly against
  Ethereum's `maxFeePerGas`.

**Cardano-mainnet mempool default sizing.** From
`IntersectMBO/ouroboros-consensus`,
`Ouroboros.Consensus.Mempool.Capacity::computeMempoolCapacity`:

```haskell
blockCount = case override of
  NoMempoolCapacityBytesOverride -> 2
  MempoolCapacityBytesOverride (ByteSize32 x) -> max 1 $ …
SemigroupViaMeasure capacity =
  stimes blockCount (SemigroupViaMeasure oneBlock)
```

i.e. **default mempool capacity is exactly `2 × oneBlock` measured
via the ledger's `blockCapacityTxMeasure`**, which on Praos =
`2 × maxBlockBodySize` = `2 × 90112` = **180224 bytes** (≈ 176 KB).
This is confirmed by the Cardano Forum mempool reference: "the
mempool size is currently set to 128 KB: twice the current block
size, which has been chosen based on queuing models" (the 128 KB
figure dates to before the April-2022 maxBlockBodySize bump to
90112). The mempool ratchets up automatically with each
`maxBlockBodySize` parameter change.

**Phase-2 fee formula and mapping:**

- Phase-2 charges `actual_fee = minFeeB + quote_per_byte(served_lane)
  × bytes` ([`MempoolGate::on_inclusion`](../../sim-rs/sim-core/src/sim/mempool_gate.rs)
  L251-254). At baseline `quote = 44`, this is bit-equal to the
  mainnet formula `44 × bytes + 155381` — the `minFeeA × bytes` term
  hasn't *disappeared*, it's been *renamed* to the dynamic
  `quote_per_byte × bytes` term. **`minFeeA` is still consumed
  internally**: it parameterises `Eip1559Settings` as the era-floor
  enforced inside the controller update ([config.rs](../../sim-rs/sim-core/src/config.rs)
  L1056) and is stored on `MempoolGateConfig.min_fee_a` ([config.rs](../../sim-rs/sim-core/src/config.rs)
  L1102) — the gate just doesn't *add* it to the fee at admission
  because the dynamic `quote` already plays that role.
- Phase-2 introduces `max_fee_lovelace` per tx, the
  EIP-1559-style maxFee envelope. The mechanism-design spec is
  explicit: this **reinterprets the existing transaction `fee`
  field** as the maxFee envelope rather than the exact fee
  ([mechanism-design.md](../../docs/phase-2/mechanism-design.md)
  L39-51). The actual fee is computed at inclusion-time
  from the (possibly-drifted) current quote, and the gap
  `max_fee_lovelace − actual_fee` is refunded via Polina's
  separate fee-change-return CIP.
- The default actor `max_fee_policy = ScaledOverLaneQuote{4, 1}`
  implements `max_fee_lovelace = minFeeB +
  ⌈quote × bytes × 4 / 1⌉` — i.e. 4× the dynamic-quote portion
  of the fee. At baseline `quote = 44`, this gives 4× quote-drift
  headroom over today's mainnet fee.
- The mempool cap is `2 × eb_referenced_txs_max_size_bytes` =
  `2 × 12000000` = **24000000 bytes (24 MB)** —
  derived inside `config.rs` L1048-1051 when
  `mempool-max-total-size-bytes: null` (the default in
  `protocol-base.yaml`).

**Initial-quote table** (read from `parameters/phase-2-sweep/pricing/`):

| Pricing variant | Standard initial | Priority initial |
|---|---|---|
| `baseline_flat_fee` | c = 1 (constant); fee = `minFeeB + 1 × bytes` | n/a |
| `eip1559_*` (single-lane) | 44 lovelace/byte | n/a |
| `two_lane_*_x4` | 44 | 176 (= 4 × 44) |
| `two_lane_*_x8` | 44 | 352 (= 8 × 44) |
| `two_lane_*_x16` | 44 | 704 (= 16 × 44) |

The standard-lane initial quote = 44 exactly reproduces today's
mainnet per-byte coefficient. Priority initial quote scales the
multiplier floor for that variant.

## Comparison Table

| Knob / concept | Phase-2 value | Mainnet value | Δ | Impact on phase-2 conclusions |
|---|---|---|---|---|
| `min-fee-a` (per-byte coefficient at baseline) | 44 (consumed as era-floor + initial quote, not added directly to fee at admission) | minFeeA = 44 lovelace/byte | 0 numerically | Semantic re-interpretation: the static `minFeeA × bytes` term becomes a *dynamic* `quote × bytes` term whose **baseline value is 44**. At controller equilibrium with no drift, phase-2 charges *exactly* mainnet fees. NEEDS-DISCLOSURE in any write-up because the equivalence is non-obvious. |
| `min-fee-b` | 155381 | minFeeB = 155381 lovelace | 0 (exact match) | None — fee floor is mainnet-current. |
| Standard-lane `initial-quote-per-byte` | 44 | minFeeA = 44 (the static analogue) | 0 (exact match) | None — standard lane initialises at mainnet's static fee. |
| Priority-lane `initial-quote-per-byte` (x4/x8/x16 floors) | 176 / 352 / 704 | No mainnet analogue (no priority lane today) | n/a | Counter-factual by design; this is the phase-2 priority-premium probe. The x4 floor (44 × 4 = 176) makes priority cost 4× a baseline-mainnet fee per byte. |
| `maxTxSize` | 16384 (`tx-max-size-bytes`, inherited from `config.default.yaml`) | maxTxSize = 16384 | 0 (exact match) | None. |
| `mempool-max-total-size-bytes` (derived default) | 24,000,000 bytes = `2 × 12,000,000` (= 2 × eb-referenced-txs-max-size-bytes; **Leios EB cap, not Praos block cap**) | cardano-node default = `2 × oneBlock` = `2 × 90112` = 180,224 bytes | **+133×** larger | Phase-2's mempool is two orders of magnitude larger than mainnet's, but this is a *consequence of Leios's 12 MB EB cap*, not of a different cap *rule*. Both follow `2 × max-bearer-block-bytes`. Under Leios, an EB carries up to 12 MB of tx bodies; under Praos, an RB carries up to 88 KB. **Both networks set the mempool to two block-bearers' worth.** The 133× ratio reflects the 12 MB / 88 KB block-cap ratio, not a different mempool-sizing philosophy. NEEDS-DISCLOSURE in any cross-system comparison. |
| Mempool cap rule | `2 × eb_referenced_txs_max_size_bytes` (auto-derived when null in YAML; explicit override possible) | `2 × oneBlock` via `computeMempoolCapacity` when `NoMempoolCapacityBytesOverride` (operators can override) | Same rule shape (2× the largest tx-bearer) | None at the rule level. Both networks use "two-blocks-worth of mempool" with operator override. |
| Mempool overflow policy | Reject-only on full mempool; no eviction of valid txs to make room ([`mempool_gate.rs:170-179`](../../sim-rs/sim-core/src/sim/mempool_gate.rs)) | Reject incoming when full; existing txs persist (the cardano-node mempool is a queue with full-rejection, not a priority eviction queue) | 0 (matches) | None — same policy shape. |
| **Fee semantics** | `fee_field = max_fee_lovelace` (maxFee envelope; actual charged fee may be lower, gap refunded) | `fee_field = exact_fee` (deterministic at tx-creation; no maxFee concept) | **Qualitative break** | This is a deliberate, spec-level change phase-2 introduces. Mechanism-design.md L39-51 frames it explicitly: "Each transaction's existing `fee` field is interpreted as the maximum total lovelace amount the transaction authorises… This document refers to that value as `maxFee`." On mainnet today, wallets ship exact fees and there is no refund mechanism. **NEEDS-DISCLOSURE in the strongest sense**: phase-2's user-experience model is not what Cardano users have today. The refund path also depends on Polina's separate fee-change-return CIP, which is out-of-scope for phase-2 itself. |
| Default `max_fee_policy` | `ScaledOverLaneQuote { 4, 1 }` ⇒ `max_fee = minFeeB + ⌈4 × quote × bytes⌉` (4× headroom on the dynamic portion) | No analogue — wallets compute exact fee, no buffer. Closest mental model: a wallet would presumably ship at `1×` (the exact min-fee), giving zero drift headroom (analogous to phase-2's `paper_like_mispriced.yaml` `{1, 1}` knob). | n/a (no mainnet analogue exists today; a hypothetical EIP-1559-style Cardano wallet would have to pick *some* policy) | NEEDS-DISCLOSURE. Phase-2's `{4, 1}` is not "wrong" — it's a sensible default for a post-deployment wallet under quote-drift, comparable to Ethereum wallets defaulting to ~2× `maxFeePerGas` for safety. But it's a **modelled assumption about future wallet behavior**, not a parameter calibrated from observed mainnet user behavior (which doesn't exist for this knob). The `paper_like_mispriced.yaml` `{1, 1}` knob deliberately models the worst case — users who treat phase-2 like mainnet and ship at exact min-fee — and the suite design uses this contrast to bound the welfare impact of mis-calibration. |
| Empirical baseline per-tx fee (200-byte tx, no congestion) | `minFeeB + 1 × bytes = 155581 lovelace` (≈ 0.156 ADA) under `baseline_flat_fee` (c=1); `minFeeB + 44 × bytes = 164181 lovelace` (≈ 0.164 ADA) under EIP-1559 baseline (quote=44) | `44 × 200 + 155381 = 164181 lovelace` (≈ 0.164 ADA) | EIP-1559 baseline matches mainnet exactly; `baseline_flat_fee` understates by `(44-1) × 200 = 8600 lovelace` per tx (because c=1 means 1 lovelace/byte, not 44) | The `baseline_flat_fee` suite is the deliberate counter-factual ("what if Cardano really did charge 1 lovelace/byte for all bytes?"), used as the null comparison against dynamic suites. Not a misconfiguration; documented as such. The EIP-1559 baseline (`initial-quote-per-byte: 44`) reproduces mainnet fee exactly at t=0 before any controller drift. |
| Admission rejection on `max_fee_lovelace` | Tx rejected at admission if `posted_fee = minFeeB + quote × bytes > max_fee_lovelace` ([`mempool_gate.rs:160-168`](../../sim-rs/sim-core/src/sim/mempool_gate.rs)) | n/a (no max-fee envelope today; "rejection" path on mainnet is "wallet computes exact fee, signs, submits — node accepts iff sigs/UTxOs/scripts are valid") | Phase-2-only mechanism | Documented in spec. The rejection rate observed in phase-2 suites is a **first-order finding of the experiment**, not a calibration knob. Suite metrics (`admission_rejections_max_fee`, `evictions_quote_drift`) surface this directly. |

## Findings

- **Phase-2's static fee floor matches mainnet exactly.** minFeeA
  (44 lovelace/byte) and minFeeB (155381 lovelace) are bit-equal to
  the current Conway-era mainnet values, and the baseline EIP-1559
  initial quote (44 lovelace/byte) reproduces the mainnet
  `minFeeA × bytes` term exactly at controller equilibrium. **At
  t=0 with no controller drift, phase-2 charges mainnet fees to
  the lovelace.** This is the strong-equivalence anchor that any
  paper / CIP write-up should lead with.

- **The fee-formula simplification (`minFeeB + quote × bytes`
  instead of `minFeeA × bytes + minFeeB`) is a rename, not a
  semantic loss.** The dynamic `quote_per_byte` *is* the
  generalisation of the static `minFeeA`; reading the code,
  `min_fee_a` is still carried in `Eip1559Settings` and
  `MempoolGateConfig` as the era floor enforced inside the
  controller update path. The gate just doesn't *add* a separate
  `minFeeA × bytes` term to the fee because the controller's
  dynamic quote already plays that role. **The equivalence is
  non-obvious from skim-reading and needs disclosure.**

- **The mempool-cap rule shape matches mainnet exactly (2× one
  bearer block); the absolute byte cap diverges by 133× because
  Leios's EB is 133× larger than Praos's RB.** Both networks
  use `2 × max-bearer-block-bytes` and a reject-on-full overflow
  policy. The 24 MB phase-2 cap vs 176 KB mainnet cap is *not* a
  philosophical difference; it's a downstream consequence of the
  CIP-0164 12 MB EB target. **NEEDS-DISCLOSURE** when discussing
  cross-system comparisons: a reader who knows Cardano mainnet's
  mempool is ~180 KB might be surprised by 24 MB.

- **The biggest qualitative deviation is fee-field semantics.**
  Mainnet today: `tx.fee` is the deterministic exact fee a wallet
  computed at sign-time; there is no maxFee envelope and no refund
  path. Phase-2: `tx.fee` is the maxFee envelope; the actual
  charged amount comes from the current quote at inclusion, and
  the difference is refunded via Polina's separate
  fee-change-return CIP. This is not a calibration issue but a
  **mechanism-level change phase-2 explicitly proposes**
  (mechanism-design.md L39-51). It is the single most
  user-visible difference from the world Cardano users have today
  and **must be the headline disclosure** in any external write-up.
  The dependency on Polina's CIP for the refund path is itself an
  external-coupling risk: phase-2's welfare conclusions assume
  the refund mechanism exists.

- **The default actor `max_fee_policy = {4, 1}` is a modelling
  assumption, not a calibration from observed mainnet behavior.**
  Mainnet has no equivalent today; the closest analogue is the
  Ethereum wallet convention of defaulting to ~2× `maxFeePerGas`
  for safety. Phase-2's 4× headroom is a defensible round number
  but the spike research found no Cardano-side calibration data
  for it (because the knob doesn't exist on Cardano). The suite
  design partially compensates: `paper_like_mispriced.yaml`
  uses `{1, 1}` (zero headroom) for the hard-deadline component
  to bound the worst case. NEEDS-DISCLOSURE: phase-2's quantitative
  welfare numbers depend on the assumed wallet-policy distribution,
  not on observed user behavior.

- **`maxTxSize` matches mainnet exactly** (16384 bytes inherited
  from `config.default.yaml`). No deviation here.

- **No CIP-1694 governance action in flight that would change
  minFeeA/minFeeB.** Web searches surfaced no in-flight parameter
  proposals targeting the fee coefficients; the Apr-2026 Parameter
  Committee meeting notes focus on stake-pool fixed-cost reduction
  and Van Rossem hard-fork prep, not fee changes. Spike 003 covers
  controller-side calibration; spike 002's fee-floor numbers are
  expected to remain stable through the phase-2 paper-window.

## Investigation Trail

- The mainnet fee-formula values (44 / 155381) were re-verified
  against `developers.cardano.org/docs/learn/core-concepts/fees/`
  and `docs.cardano.org/about-cardano/explore-more/fee-structure/`,
  consistent with spike 001's `/epoch_params` retrieval.
  Both sources cite the worked example `44 × 200 + 155381 =
  164181 lovelace` for a 200-byte tx, which serves as the
  empirical anchor against phase-2's baseline EIP-1559 quote.

- The cardano-node default mempool sizing was resolved via
  WebFetch of
  `raw.githubusercontent.com/IntersectMBO/ouroboros-consensus/main/ouroboros-consensus/src/ouroboros-consensus/Ouroboros/Consensus/Mempool/Capacity.hs`,
  which shows the `computeMempoolCapacity` formula directly:
  `blockCount = case override of NoMempoolCapacityBytesOverride
  -> 2`. Cross-checked against the Cardano Forum thread
  "Understanding the Cardano Mem-Pool" and the cardano-node
  PR #3343 ("Document MempoolCapacityBytesOverride") which both
  describe the default as "two blocks". The 128 KB figure from
  the forum thread predates the 90 KB block-body-size bump; the
  current default is 180,224 bytes.

- The phase-2 mempool-cap derivation was traced through
  `protocol-base.yaml` (`mempool-max-total-size-bytes: null`) →
  `sim-core/src/config.rs:1048-1051` (the `or_else` that
  computes `2 × eb_referenced_txs_max_size_bytes`) →
  `eb-referenced-txs-max-size-bytes: 12000000` in `protocol-base.yaml`.
  Final value: 24 MB.

- The fee-formula equivalence (phase-2's `minFeeB + quote × bytes`
  at `quote = 44` = mainnet's `minFeeA × bytes + minFeeB`) was
  verified by direct reading of
  `MempoolGate::on_inclusion` and `MempoolGate::fee_at` in
  `sim-core/src/sim/mempool_gate.rs`. The non-obvious part is
  that `min_fee_a` is still consumed (as the era-floor inside the
  controller update) — confirmed by tracing the
  `Eip1559Settings.min_fee_a` field through
  `config.rs:1056, 1068, 1077` and the `MempoolGateConfig.min_fee_a`
  field at L1102. Phase-2 has not deleted `minFeeA`; it has only
  changed *which formula it appears in*.

- The fee-field-semantic re-interpretation was confirmed in
  `docs/phase-2/mechanism-design.md` L39-51 ("Each transaction's
  existing `fee` field is interpreted as the maximum total lovelace
  amount the transaction authorises…"). Cardano's deterministic-
  fee story was cross-referenced via the IOG blog
  "Cardano's EUTXO model" and the CIP-related material on
  exact-fee vs Ethereum-style `maxFeePerGas`. No mainnet
  CIP-in-flight changes the fee-field semantics; phase-2 is
  proposing this change.

- Wallet-side max-fee behavior could not be quantitatively
  characterised because the concept does not exist on Cardano.
  Lace / Eternl / Daedalus all ship at the deterministic
  computed min-fee via `cardano-serialization-lib` / `cardano-cli`.
  This makes the phase-2 default `{4, 1}` a future-prediction
  assumption rather than a calibration; the suite's
  `paper_like_mispriced.yaml` `{1, 1}` knob deliberately bounds
  the downside.

## Verdict

**NEEDS-DISCLOSURE.** No calibration knob is *numerically* wrong:
minFeeA, minFeeB, maxTxSize match mainnet exactly; the mempool-cap
rule matches in shape (2× one bearer block); the baseline EIP-1559
initial quote (44) reproduces mainnet fees to the lovelace at
controller equilibrium. But three mechanism-level changes need
explicit, prominent disclosure in any paper / CIP write-up:

1. **Fee-field semantic re-interpretation.** Mainnet `fee` = exact
   fee; phase-2 `fee` = maxFee envelope. This is a user-experience
   break and depends on Polina's separate fee-change-return CIP
   for the refund path.

2. **Mempool absolute byte cap is 133× larger than mainnet.** Same
   rule (2× bearer-block bytes), but Leios's 12 MB EB drives the
   absolute number to 24 MB vs mainnet's 180 KB. Worth flagging
   when comparing absorption / drift dynamics across systems.

3. **`max_fee_policy` is a modelled assumption, not a calibration
   from observed mainnet behavior**, because the underlying user
   choice doesn't exist on Cardano today. The suite design's
   `paper_like_mispriced` worst-case bound partially mitigates,
   but the default `{4, 1}` is a forecast about post-deployment
   wallet conventions, not a measurement.

None of these invalidate phase-2's conclusions; together they
constitute the "Cardano-side asterisks" the write-up should carry.

## Sources

- [Cardano Developer Portal — Transaction Fees](https://developers.cardano.org/docs/learn/core-concepts/fees/) — retrieved 2026-05-13 (200-byte tx ⇒ 164181 lovelace fee worked example)
- [Cardano Docs — Fee Structure](https://docs.cardano.org/about-cardano/explore-more/fee-structure/) — retrieved 2026-05-13
- [Cardano on-chain `/epoch_params` (epoch 540) and `/tip` (epoch 630) — URL](https://api.koios.rest/api/v1/epoch_params?_epoch_no=540) — retrieved 2026-05-13 (minFeeA = 44, minFeeB = 155381, maxTxSize = 16384)
- [IntersectMBO/ouroboros-consensus — Mempool.Capacity.computeMempoolCapacity](https://raw.githubusercontent.com/IntersectMBO/ouroboros-consensus/main/ouroboros-consensus/src/ouroboros-consensus/Ouroboros/Consensus/Mempool/Capacity.hs) — retrieved 2026-05-13 (`NoMempoolCapacityBytesOverride -> 2` default)
- [cardano-node PR #3343 — Document MempoolCapacityBytesOverride](https://github.com/input-output-hk/cardano-node/pull/3343) — retrieved 2026-05-13 ("default is only 2 blocks")
- [Cardano Forum — Understanding the Cardano Mem-Pool](https://forum.cardano.org/t/understanding-the-cardano-mem-pool/123417) — retrieved 2026-05-13
- [Cardano Forum — Mempool Bytes full since mid 1/14/2022](https://forum.cardano.org/t/mempool-bytes-full-since-mid-1-14-2022/92328) — retrieved 2026-05-13 ("128 KB: twice the current block size, chosen based on queuing models")
- [Cardano Node Configuration Docs (GitBook)](https://cardano-course.gitbook.io/cardano-course/handbook/protocol-parameters-and-configuration-files/node-configuration-file) — retrieved 2026-05-13 (MempoolCapacityBytesOverride NoOverride default)
- [IOG Blog — Cardano's EUTXO Model: Deterministic Predictability](https://www.iog.io/blog/posts/2025/05/02/cardano-s-eutxo-model-bringing-deterministic-predictability-to-blockchain/) — retrieved 2026-05-13 (deterministic exact fees vs Ethereum's maxFeePerGas)
- [Emurgo cardano-serialization-lib — Fee Calculation](https://github.com/Emurgo/cardano-serialization-lib/blob/master/doc/getting-started/generating-transactions.md) — retrieved 2026-05-13 (LinearFee 44 / 155381; exact-fee calculation)
- [EIP-1559 spec — maxFeePerGas / maxPriorityFeePerGas semantics](https://eips.ethereum.org/EIPS/eip-1559) — retrieved 2026-05-13 (reference for Ethereum's max-fee model that phase-2 imports)
- [Cardano Forum — Apr 30 2026 Parameter Committee Meeting](https://forum.cardano.org/t/apr-30-2026-parameter-committee-triweekly-meeting-notes/154366) — retrieved 2026-05-13 (no fee-parameter changes in flight)
- In-repo provenance: `sim-rs/parameters/phase-2-sweep/protocol-base.yaml` (minFeeA = 44, minFeeB = 155381, mempool-cap rule), `sim-rs/sim-core/src/sim/mempool_gate.rs` (`try_admit`, `on_inclusion`, `fee_at`), `sim-rs/sim-core/src/tx_actors.rs` (`MaxFeePolicy::ScaledOverLaneQuote`), `sim-rs/sim-core/src/config.rs:1048-1051, 1100-1104` (mempool-cap derivation and `MempoolGateConfig`), `docs/phase-2/mechanism-design.md` L39-51 (maxFee semantic re-interpretation), `sim-rs/parameters/phase-2-sweep/demand/paper_like_mispriced.yaml` ({1,1} worst-case knob), `sim-rs/parameters/phase-2-sweep/pricing/two_lane_priority_only_static_x4.yaml` (priority initial quote 176 = 4 × 44).
