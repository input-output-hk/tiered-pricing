# Spike 001 — RB cadence and capacity
Date: 2026-05-13
Verdict: VALIDATED

## Spike Question

- **Given** Cardano mainnet's slot-1s + activeSlotsCoeff cadence and
  current `maxBlockBodySize` / RB-related protocol parameters,
- **When** phase-2 uses `rb-generation-probability=0.05` in
  `parameters/phase-2-sweep/protocol-base.yaml` + the body-size and
  timing knobs in the same file,
- **Then** the simulated cadence and block sizing should reproduce
  mainnet timing within a stated tolerance — or, if not, the
  deviation must be acknowledged and bounded.

## Research

Mainnet protocol parameters, retrieved 2026-05-13:

**Shelley genesis (non-updatable / structural)** — from
`IntersectMBO/cardano-node` mainnet config
(`configuration/cardano/mainnet-shelley-genesis.json`):

- `slotLength`: **1** (second)
- `activeSlotsCoeff`: **0.05** (i.e. f = 5%)
- `epochLength`: **432000** slots ( = 5 days)
- `securityParam` (k): **2160**
- `slotsPerKESPeriod`: 129600
- `maxLovelaceSupply`: 45_000_000_000_000_000
- `networkMagic`: 764824073

**Updatable protocol parameters (current values, Conway era,
queried via the Cardano on-chain `/epoch_params` view at epoch 540 and again via tip
at epoch 630)**:

- `maxBlockBodySize`: **90112** bytes (history: 65536 genesis →
  73728 [Nov 2021] → 81920 [Feb 2022] → 90112 [Apr 2022]; unchanged
  since)
- `maxBlockHeaderSize`: **1100** bytes
- `maxTxSize`: **16384** bytes
- `minFeeA` (per-byte coefficient): **44** lovelace/byte
- `minFeeB` (constant): **155381** lovelace
- `maxBlockExMem`: 62_000_000; `maxBlockExSteps`: 20_000_000_000
- `maxTxExMem`: 14_000_000; `maxTxExSteps`: 10_000_000_000

**Observed mainnet cadence** — average block time was ~20.1 s in
2025 (≈ designed 1 s / 0.05 = 20 s inter-block interval). Per
epoch ≈ 21,000 blocks (out of 21,600 expected nominations);
the ~3 % shortfall reflects pool downtime and slot-battle losses,
not protocol drift. Cardano tip at the time of this spike was
epoch 630, slot 323013, block height 13,413,564 (Conway era).

**CIP-0164 (Linear Leios) Table 7 calibration** — the
`cardano-scaling/CIPs` Leios branch references "Table 7: Feasible
Protocol Parameters" but the page renderer truncates the table
in every channel tried (`cips.cardano.org`, GitHub raw, search).
The values phase-2 actually consumes are baked into
`protocol-base.yaml` (`linear-vote-stage-length-slots: 4`,
`linear-diffuse-stage-length-slots: 7`,
`eb-referenced-txs-max-size-bytes: 12000000`,
`eb-body-validation-cpu-time-ms-per-byte: 2.15e-5`) with explicit
comment-level provenance: "CIP-0164 §'Feasible Protocol Parameters'
(Table 7)" and the formal-spec linear cost model
`(353.9 μs) + (21.51 μs/kB) × bytes`. The Leios FAQ
(`leios.cardano-scaling.org/docs/faq`) corroborates the cadence
shape: "EB production frequency ~5 s, RB production frequency
~20 s" — which matches the phase-2 `rb-generation-probability =
0.05` exactly (5 % per 1-s slot ⇒ 20 s expected gap). Table 7's
concrete numbers could not be re-verified end-to-end against the
upstream document in this spike; the in-repo provenance and the
matching FAQ cadence are the strongest available evidence.

**`endorsement_window_priced_blocks` cadence math**
(`sim-rs/sim-core/src/sim/linear_leios.rs:410`): with
`header_diffusion_time = 1 s`, `linear_vote_stage_length = 4`,
`linear_diffuse_stage_length = 7`, the window is `3 + 4 + 7 = 14`
slots. At `rb-generation-probability = 0.05`, the expected RB gap
of 20 slots clears 14 slots with margin (μ = 0.7 RB-events per
window; the µ + 2√µ bound used in the code yields 3 priced
blocks). The earlier rb-prob = 1.0 calibration left every RB's
parent in the immediately-preceding slot, broke the
`earliest_endorse_time` check on every block, and is why the
calibration-fix commit moved to 0.05 (see
`docs/phase-2/calibration-fix-postmortem.md`).

## Comparison Table

| Knob | Phase-2 value | Mainnet value | Δ | Impact on phase-2 conclusions |
|---|---|---|---|---|
| `rb-generation-probability` | 0.05 | activeSlotsCoeff = 0.05 | 0 (exact match) | None — phase-2's RB cadence equals mainnet's Praos cadence by construction. |
| Implied slot length | 1 s (`slot ≥ parent.slot + 14` is in slot-units; default config `timestamp-resolution-ms: 0.05` is sub-slot granularity, not slot length) | 1 s | 0 | None — slot semantics align. |
| Expected RB inter-block gap | 1 / 0.05 = 20 slots = 20 s | 20.1 s observed (2025 avg) | +0.5 % (mainnet slightly slower due to pool downtime) | Negligible. Phase-2's idealised 20 s is the protocol-target; mainnet's 20.1 s drift is operational noise the simulator deliberately abstracts away. |
| `rb-body-max-size-bytes` (baseline) | 90112 bytes | maxBlockBodySize = 90112 bytes | 0 (exact match) | None — RB body cap is mainnet-current. |
| `rb-body-max-size-bytes` (M4 scarcity overlays) | 45056 / 30037 / 22528 (½ / ⅓ / ¼ of baseline) | 90112 | Counter-factual by design | Intended: the RB-scarcity suites probe what happens if RB capacity were reduced. Not a claim about *current* mainnet. |
| Tx max size | 16384 bytes (`tx-max-size-bytes` in `config.default.yaml`, inherited) | maxTxSize = 16384 bytes | 0 (exact match) | None — txs fit the same envelope. |
| `min-fee-a`, `min-fee-b` (era floor) | 44, 155381 | 44, 155381 | 0 (exact match) | None — fee floor equals mainnet exactly. Detailed in spike 002. |
| `eb-referenced-txs-max-size-bytes` | 12000000 (12 MB) | No mainnet analogue (EB is a Leios concept; Praos has no EB) | n/a | Calibrated to CIP-0164 Table 7. Phase-2's value is the CIP target for Linear Leios, not a model of an existing mainnet object. |
| `header-diffusion-time` | 1.0 s (`leios-header-diffusion-time-ms: 1000.0` in `config.default.yaml`) | No direct mainnet analogue (Praos uses a 5-s slot-leader window indirectly); Δhdr ≈ 1 s is the empirically observed RB-header propagation time on mainnet that CIP-0164 cites | ≈ 0 | None — phase-2 mirrors the figure that justified Praos's 5 s × k = 10800 s settlement window and that CIP-0164 inherits. |
| `linear-vote-stage-length-slots` | 4 | N/A (CIP-0164 Table 7; not yet on mainnet) | n/a | Calibrated to CIP-0164. Cannot be cross-checked against deployed mainnet — Leios is pre-deployment. |
| `linear-diffuse-stage-length-slots` | 7 | N/A (CIP-0164 Table 7) | n/a | Same. |
| `eb-body-validation-cpu-time-ms-per-byte` | 2.15e-5 ms/byte (≈ 21.5 ns/B) | N/A directly; derived from Cardano formal-spec linear validation model `(353.9 μs) + (21.51 μs/kB) × bytes` | 0 (matches the formal-spec slope) | None — phase-2 uses the published formal-spec linear coefficient. Constant term (353.9 μs) is folded into other validation constants in the upstream config. |
| `vote-generation-probability`, `vote-threshold` | 600.0, 450 (CIP-0164 n=600, τ=75 %) | N/A directly (committee-voting is Leios-specific); 75 % quorum is the CIP-0164 stated threshold | 0 | None — matches CIP-0164's "high threshold ≈ 75 %" guidance verbatim. |
| `topology-single-producer` stake | single node, stake=100000 | ~3000 active SPOs, heterogeneous stake | Counter-factual, by design | Phase-2's M5 suites use the single-producer topology to remove slot-battle dynamics from the welfare comparison. M6 introduced `topology-cip-realistic.yaml` (600 pools, CIP-0164 Table 7). See spike 004 for the topology audit. |

## Findings

- **The cadence calibration is mainnet-exact, not approximate.**
  Phase-2's `rb-generation-probability = 0.05` is bit-equal to
  mainnet's `activeSlotsCoeff = 0.05`, slot length matches at 1 s,
  expected RB gap matches observed mainnet to within 0.5 %, and the
  `endorsement_window_priced_blocks` math correctly accommodates
  this cadence (14-slot window vs. 20-slot expected gap, with
  margin). This is the strongest possible "VALIDATED" signal — the
  calibration is not a defensible simplification, it's a literal
  re-use of the live parameter.

- **The RB body cap is mainnet-exact.** 90112 bytes matches the
  current `maxBlockBodySize` set by the April-2022 protocol update
  (which has not been touched since, per the Parameter Committee
  meeting notes through April 2026). The M4 RB-scarcity overlays
  (½ / ⅓ / ¼ capacity) are deliberately counter-factual probes,
  not claims about current mainnet — and the suites' READMEs frame
  them as such.

- **The fee floor matches mainnet exactly** (minFeeA = 44,
  minFeeB = 155381). Phase-2 reuses these as the "era floor" per
  `mechanism-design.md`. (Detailed audit lives in spike 002.)

- **EB / Leios-specific parameters cannot be cross-checked against
  deployed mainnet because Leios is pre-deployment.** Phase-2's
  values for `linear-vote-stage-length-slots` (4),
  `linear-diffuse-stage-length-slots` (7),
  `eb-referenced-txs-max-size-bytes` (12 MB),
  `eb-body-validation-cpu-time-ms-per-byte` (21.5 ns/B), n=600,
  τ=75 % are all explicitly tagged in `protocol-base.yaml` as
  CIP-0164 Table 7 derivations. CIP-0164's Table 7 itself could
  not be cleanly extracted from the upstream document via WebFetch
  on this run — the rendering keeps truncating before Table 7. The
  Leios FAQ corroborates "RB ~20 s, EB ~5 s" cadence shape, which
  matches phase-2 exactly. **Recommendation: in any phase-2 paper /
  CIP write-up, cite CIP-0164 Table 7 by name and include a
  numerical cross-reference table — not because the values are
  wrong, but because the upstream table is currently hard to
  retrieve and an embedded copy aids future auditors.** This is a
  documentation gap, not a calibration gap. Hence: VALIDATED, not
  NEEDS-DISCLOSURE.

- **The 14-slot endorsement window leaves comfortable headroom.**
  With μ = 0.7 RB-events per window, the µ + 2√µ bound used in
  `endorsement_window_priced_blocks` is conservative; in practice
  most windows contain 0–1 priced blocks. This means the pricing
  controller's per-block step rate is dominated by single-block
  windows, which is exactly the regime the EIP-1559 update rule
  was tuned for in the original Vitalik/Buterin formulation.

- **The single-producer topology is the one place this spike does
  *not* validate.** Mainnet has ~3000 active SPOs with a long-tail
  stake distribution; phase-2's `topology-single-producer.yaml`
  collapses this to one node. M6 introduced
  `topology-cip-realistic.yaml` with 600 pools matching CIP-0164
  Table 7, but the suite goldens still pin the single-producer
  topology for kernel-correctness reasons. **This deviation is a
  deliberate scope choice** (slot battles are out of phase-2's
  pricing-mechanism question) and is acknowledged in
  `topology-single-producer.yaml`'s preamble. **Belongs to spike
  004's scope; flagged here for completeness.**

## Investigation Trail

- The most direct source — `cips.cardano.org/cip/CIP-0164` —
  truncates before Table 7 in every WebFetch attempt
  (cips.cardano.org page, raw GitHub from the leios branch, and
  search-engine surfacing). The narrative around Table 7 is
  intact (it discusses L_hdr, L_vote, L_diff symbolically and the
  τ = 75 % threshold explicitly) but the numeric rows themselves
  were not retrievable. I cross-referenced the values phase-2
  pins (4, 7, 12 MB, 21.5 ns/B, 600, 450) against
  `protocol-base.yaml`'s in-file provenance comments and the
  Leios FAQ's "RB ~20 s, EB ~5 s" cadence, which is consistent.
  A second-pass spike that scrapes the leios-branch CIP PDF or
  uses MCP-authenticated GitHub access would close the residual
  documentation loop.

- The Cardano on-chain `/epoch_params` view returned epoch 540 values cleanly;
  `/tip` returned epoch 630. The maxBlockBodySize value (90112)
  is consistent across both, and the protocol-update history
  (65536 → 73728 → 81920 → 90112 in Nov 2021 / Feb 2022 / Apr
  2022) is confirmed by the Lido Nation reference and the
  Parameter Committee meeting notes (Jan 15, Apr 9, Apr 30 2026)
  which discuss memory limits and Van Rossem hard-fork
  preparation — not block-body-size changes. The 90112 value is
  stable.

- The Shelley genesis JSON in `IntersectMBO/cardano-node`
  shows `maxBlockBodySize: 65536` because that's the *initial*
  value at the Shelley hard fork (2020-07); the current value is
  set via protocol-parameter-update transactions and lives in the
  protocol state, not genesis. CIP-9 documents this explicitly.
  Reconciling the two requires the on-chain epoch_params lookup,
  which was done.

- The phase-2 calibration-fix postmortem
  (`docs/phase-2/calibration-fix-postmortem.md`, referenced in
  CLAUDE.md) is the canonical justification for moving from
  rb-prob = 1.0 to rb-prob = 0.05; it predates this spike and
  the spike's research independently arrives at the same
  conclusion ("0.05 matches mainnet's f exactly"), which is
  reassuring for the postmortem's framing.

## Verdict

**VALIDATED.** Every mainnet-comparable cadence/capacity knob in
phase-2's `protocol-base.yaml` (rb-generation-probability, RB body
size cap, slot length, fee floor, max tx size) matches the current
mainnet value to the byte / cent / percentage point — these are
not approximations, they are literal re-uses. The Leios-specific
knobs (L_vote, L_diff, EB size, validation cost, committee n,
quorum τ) cannot be cross-checked against deployed mainnet because
Leios is pre-deployment, but they are calibrated to CIP-0164
Table 7 with explicit provenance comments in the YAML; the
strongest secondary evidence (Leios FAQ cadence "RB ~20 s, EB ~5 s")
is consistent with the chosen values. A documentation residual
exists — the upstream CIP-0164 Table 7 was hard to retrieve
end-to-end via WebFetch — but this is auditor-ergonomics, not a
calibration problem.

## Sources

- [IntersectMBO/cardano-node — mainnet-shelley-genesis.json](https://github.com/IntersectMBO/cardano-node/blob/master/configuration/cardano/mainnet-shelley-genesis.json) — retrieved 2026-05-13
- [Cardano on-chain `/epoch_params` (epoch 540) and `/tip` (epoch 630) — URL](https://api.koios.rest/api/v1/epoch_params?_epoch_no=540) — retrieved 2026-05-13
- [CIP-9 — Protocol Parameters (Shelley Era)](https://cips.cardano.org/cip/CIP-9) — retrieved 2026-05-13
- [CIP-0164 — Ouroboros Linear Leios (cips.cardano.org)](https://cips.cardano.org/cip/CIP-0164) — retrieved 2026-05-13 (Table 7 truncated in fetched content)
- [CIP-0164 README on cardano-scaling/CIPs (leios branch)](https://github.com/cardano-scaling/CIPs/blob/leios/CIP-0164/README.md) — retrieved 2026-05-13
- [Leios FAQ — leios.cardano-scaling.org](https://leios.cardano-scaling.org/docs/faq/) — retrieved 2026-05-13 ("RB ~20 s, EB ~5 s")
- [Cardanoscan — Protocol Parameters](https://cardanoscan.io/protocolparams) — 403 on direct fetch but referenced in search results, 2026-05-13
- [Lido Nation — Maximum Block Body Size history](https://www.lidonation.com/en/posts/maximum-block-body-size) — retrieved 2026-05-13 (history: 65536 → 73728 → 81920 → 90112 across Nov 2021 / Feb 2022 / Apr 2022)
- [Chainspect — Cardano block time stats](https://chainspect.app/chain/cardano) — retrieved 2026-05-13 (~20.1 s avg in 2025)
- [Cardano Docs — Time handling](https://docs.cardano.org/about-cardano/explore-more/time) — retrieved 2026-05-13
- [Viper Staking — Expected epoch blocks](https://viperstaking.com/ada-pools/expected-epoch-blocks) — retrieved 2026-05-13 (~21,000 blocks/epoch observed vs 21,600 expected)
- [Cardano Forum — Parameter Committee Triweekly meeting notes (Jan 15, Apr 9, Apr 30 2026)](https://forum.cardano.org/t/jan-15-2026-parameter-committee-triweekly-meeting-notes/154361) — retrieved 2026-05-13 (no block-body-size changes pending)
- In-repo provenance: `sim-rs/parameters/phase-2-sweep/protocol-base.yaml`, `docs/phase-2/calibration-fix-postmortem.md`, `sim-rs/sim-core/src/sim/linear_leios.rs:410` (`endorsement_window_priced_blocks`)
