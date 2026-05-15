# Codebase Concerns

**Analysis Date:** 2026-05-13
**Resolution annotations added:** 2026-05-14

## 2026-05-14 resolution summary

The original audit (2026-05-13) predates two changes that closed
several entries below. Each affected entry is preserved as-written
for historical context and annotated inline with an **Update
2026-05-14** note pointing at the resolving artifact. The structural
changes:

- **Topology pivot (2026-05-13):** suites switched from
  `topology-single-producer.yaml` (N=1) to
  `topology-realistic-100.yaml` (N=100, mass-stratified mainnet
  curve via spike 006). Entries below that frame phase-2 as
  single-producer (the *Pricing state has no rollback...*
  Fragile-Areas entry, the *Single-producer topology covers 7 of 7
  current phase-2 suites* Scaling-Limits entry, the
  *Slot-battle / multi-producer pricing-state divergence*
  Test-Coverage-Gaps entry) describe the pre-pivot baseline.
- **Chain-derived controller refactor (2026-05-14):** the
  node-local mutable `Eip1559Pricing.quote_per_byte` +
  `CapacityWeightedWindow` accumulator was replaced by a
  chain-derived design where every `LinearRankingBlock` carries
  `derived_quote: PerLaneQuote` and `window_aggregate:
  WindowAggregate` as pure functions of canonical predecessors.
  This eliminates WR-1 (controller contamination on slot-battle
  reorg) by construction. Mechanism Family B (EIP-1559-faithful,
  1 step per canonical block) committed for publication via
  [`../family-b-decision-2026-05-14.md`](../family-b-decision-2026-05-14.md).
  See also [`../REVIEW.md`](../REVIEW.md) Fix Status table —
  WR-1 row reads RESOLVED 2026-05-14.

Resolved 2026-05-14:
- *Pricing state has no rollback on fork / slot-battle* (Fragile
  Areas) — superseded by chain-derived design.
- *Single-producer topology covers 7 of 7 current phase-2 suites*
  (Scaling Limits) — superseded by `topology-realistic-100.yaml`
  pivot.
- *Slot-battle / multi-producer pricing-state divergence* (Test
  Coverage Gaps) — superseded by chain-derived design (the
  divergence pathway no longer exists; what remains is the
  representative-node aggregation question, which is orthogonal).

Still pending (not resolved by today's changes):
- *Cross-architecture determinism is asserted intra-arch only*
  (Fragile Areas) — still true; chain-derived computation is
  bit-stable across architectures by construction, but the wider
  simulator inherits `f64` from `main` in non-pricing paths. CR-1's
  `f64::sqrt` resolution closes one specific instance; cross-arch
  CI build pipeline remains infrastructure work outside phase-2's
  code scope.
- WR-2 (gate-reject info loss → `AdmissionRejected` event design),
  WR-7 (`ActorComponent` reallocation perf refactor) — both
  deferred from REVIEW.md by explicit decision, tracked as F3 / F4
  in REVIEW.md's "Follow-on work" section.
- All other Tech Debt, Security Considerations, Performance
  Bottlenecks, Missing Critical Features, and Test Coverage Gaps
  not annotated below — unchanged by today's pass.

## Tech Debt

**RB-reduced protocol overlays are full replacements, not stacked:**
- Issue: `JobOverrides` picks `overrides.protocol` OR `default_protocol`, never both. The three RB-reduced overlay YAMLs each duplicate the entire `protocol-base.yaml` content just to override `rb-body-max-size-bytes`.
- Files: `sim-rs/parameters/phase-2-sweep/protocol-base.yaml` (105 lines), `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-half.yaml` (37 lines), `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-third.yaml` (27 lines), `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-quarter.yaml` (27 lines), `sim-rs/sim-cli/src/runner.rs` (`JobOverrides`, `run_job` ~lines 426-430, 523-534).
- Impact: Any future addition to `protocol-base.yaml` must be propagated manually to all three overlays or the RB-scarcity suite silently diverges from baseline. M4 already hit this once — the first iteration of the overlays were one-line "diffs" and the runs produced zero events because the overlay replaced the phase-2 mechanics.
- Fix approach: Extend `JobOverrides` with a `protocol_overlay: Vec<PathBuf>` for additive stacking, parse each overlay as a partial YAML, and merge with `serde_yaml::Value` before deserialising the final `RawLinearLeiosConfig`. Deferred enhancement flagged in CLAUDE.md §Conventions and m5-handoff.md §5.

**Mixed serde casing across persisted JSON artefacts:**
- Issue: `Manifest`/`JobEntry`/suite YAML schemas use `#[serde(rename_all = "kebab-case")]`, but `RunSummary` uses Rust snake_case (no `rename_all`). Both shapes coexist on disk under `sim-rs/output/`.
- Files: `sim-rs/sim-cli/src/runner.rs` (Manifest/JobEntry kebab-case), `sim-rs/sim-cli/src/metrics/collector.rs` (RunSummary snake_case), `sim-rs/sim-cli/src/suite.rs` (Suite YAML schema).
- Impact: Future schema additions must match the surrounding type's existing convention; mixing introduces subtle deserialisation failures. Auditing a file requires remembering which side of the divide it's on.
- Fix approach: Standardising would invalidate every persisted manifest under `sim-rs/output/`, forcing re-runs of all (job, seed) pairs. Not worth the churn for M5. Document only — already done in CLAUDE.md §Conventions.

**Calibration choices baked into YAMLs (not invariants):**
- Issue: The simulator picks concrete defaults for spec-open questions. Each is observable as a knob in the YAMLs and a hard-coded magic number elsewhere.
- Files:
  - Window length 32 (length 1 for RB-reserved priority): `sim-rs/parameters/phase-2-sweep/pricing/*.yaml` `window-length` field; `sim-rs/sim-core/src/tx_pricing/window.rs`; `sim-rs/sim-core/src/tx_pricing/single_lane.rs`, `two_lane.rs`.
  - `rb-generation-probability: 0.05` + `default-slots: 1000` + `stake: 100000`: `sim-rs/parameters/phase-2-sweep/protocol-base.yaml`, `topology-single-producer.yaml`, all `suites/*.yaml`. These together clear the 13-slot endorsement window (`linear_leios.rs:707-716`) and the VRF stake-quantization truncation (`sim-rs/sim-core/src/sim/lottery.rs:7-10`).
  - `multiplier_floor = 4` (in `phase-2-rb-scarcity` and `phase-2-urgency-inversion`, vs spec default 16): `sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml`, `phase-2-urgency-inversion.yaml`.
  - Mempool cap = `2 × eb_referenced_txs_max_size_bytes`: `sim-rs/parameters/phase-2-sweep/protocol-base.yaml` `mempool-max-total-size-bytes`.
  - Default `max_fee_policy = ScaledOverLaneQuote { numerator: 4, denominator: 1 }`: hard-coded in actor defaults `sim-rs/sim-core/src/tx_actors.rs`.
- Impact: Re-calibrating any of these flips suite goldens; an externally-deployed pricing config would have to be re-validated. The spec leaves these open, so they are calibration choices, not bugs — but they need to surface in the CIP / external write-up so a deployment-side sweep knows what to revisit.
- Fix approach: Surface in CIP §Reproducibility. Document re-calibration cost per-knob (already done in CLAUDE.md §Calibration choices). For the multiplier floor, considering authoring an additional suite at `x16` to span both calibrations.

**Phase-2 size targets exceeded for sim-core total (informational):**
- Issue: CLAUDE.md §"Size sanity check" projected `sim-core/src/` at ~10k lines (rebuild-only) and the simulator total at ~12k lines. Actual: 16,305 / 21,104.
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs` (2,749 lines), `sim-rs/sim-core/src/sim/leios.rs` (1,833), `sim-rs/sim-core/src/config.rs` (1,309), `sim-rs/sim-core/src/sim/stracciatella.rs` (1,234), `sim-rs/sim-core/src/events.rs` (1,046).
- Impact: The pricing kernel (`sim-rs/sim-core/src/tx_pricing/`, 1,437 lines) and metrics (`sim-rs/sim-cli/src/metrics/`, 1,205 lines) came in well under target — the overrun is in the upstream `main` simulator code (slot lottery, propagation, voting, legacy protocol arms like `leios.rs` and `stracciatella.rs`) which phase-2 builds on top of without rewriting. Not phase-2 surface.
- Fix approach: Not a phase-2 line item. A future cleanup could prune `leios.rs` (1,833 lines) and `stracciatella.rs` (1,234 lines) — neither is the simulated protocol for phase-2 (only `linear_leios.rs` is).

**Two `// TODO: freshest first` markers in EB request relay:**
- Issue: When relaying EB announcements, the node always requests from the *first* peer that announced it (under `RelayStrategy::RequestFromFirst`). The freshness-first selection comment marker is unaddressed.
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs:1134`, `sim-rs/sim-core/src/sim/linear_leios.rs:1282`, and the same pattern in `sim-rs/sim-core/src/sim/stracciatella.rs:405`.
- Impact: Inherited from upstream `main`. Does not affect pricing-state determinism; affects propagation timing only.
- Fix approach: Outside phase-2 scope. Inherits from `main`.

**Legacy `TransactionProducer` / non-actor path coexists with `ActorComponent`:**
- Issue: The codebase carries two transaction-generation paths: the legacy `RealTransactionConfig` / `TransactionProducer` (from `main`) and the phase-2 `ActorComponent`. `TXGenerated` events default `urgency_component_index = 0`, `value_lovelace = 0`, `urgency = 1.0` for legacy txs.
- Files: `sim-rs/sim-core/src/events.rs:119-138` (default-serde fields for legacy backward-compat), `sim-rs/sim-core/src/sim/tx.rs:24,52`, `sim-rs/sim-core/src/config.rs:763,800,813,1237,1246`, `sim-rs/sim-core/src/tx_actors.rs:108` (`MaxFeeContext::unused()`).
- Impact: Welfare metrics for legacy txs collapse to `retained_value = 0`, `net_utility = -fee` (documented in `metrics/collector.rs:353-358`). Surface-area cost: tests must remember which path they're exercising, default values lurk in every `TXGenerated`. Not a bug — but a deferred consolidation.
- Fix approach: Drop the legacy `TransactionProducer` path once no phase-2 (or pre-phase-2) test depends on it. Audit `sim-rs/sim-core/src/sim/tests/m1_smoke.rs`, `sim-rs/sim-core/src/sim/tests/linear_leios.rs:151`.

## Known Bugs

No known active bugs. The historical post-M5 calibration bug (`rb-generation-probability: 1.0` + linear-Leios 13-slot endorsement window → EBs never landed on chain) was fixed by dropping to `0.05` and raising stake to `100000` to survive VRF quantisation. See `docs/phase-2/calibration-fix-postmortem.md` for the full account.

## Security Considerations

**Honest-producer-only simulation; no attacker validation of EB-partition activation:**
- Risk: The `partition_activated` bit is stored on `LinearEndorserBlock` as an honest-producer *claim*, not derivable from the EB body content. A future attacker model could test inconsistency between the claim and the body.
- Files: `sim-rs/sim-core/src/model.rs:281` (`/// an honest-producer claim. Future attacker models in M4/M5 may`), `sim-rs/sim-core/src/sim/linear_leios.rs` `select_eb_with_partition`.
- Current mitigation: None — phase-2 suites are all honest-producer.
- Recommendations: A future phase-3 attacker suite would need to validate `partition_activated` against the body (or move the trigger to a body-derivable invariant). Flagged in m3-handoff.md, carried in m5-handoff.md §Known limitations #2.

**Withheld-tx attacker model coexists but skips pricing admission:**
- Risk: When `behaviours.withhold_txs = true`, attacker EBs and txs land in `self.txs` directly without going through `MempoolGate::try_admit`, defaulting `max_fee_lovelace = u64::MAX`.
- Files: `sim-rs/sim-core/src/sim/linear_leios/attackers.rs`, `sim-rs/sim-core/src/sim/linear_leios.rs:1464-1526`, `sim-rs/sim-core/src/config.rs:881-935`.
- Current mitigation: No phase-2 suite enables `late-tx` / `late-eb` attackers. The path is dormant.
- Recommendations: Flagged in m1-handoff.md §"Withheld-tx attack scenarios". Pricing-state interaction with attacker txs is not modelled and would need explicit handling before a phase-3 attacker-vs-pricing suite is authored.

## Performance Bottlenecks

**Suite-level determinism goldens are `#[ignore]`'d to keep `cargo test` fast:**
- Problem: Each baseline run is ~200ms in release; the 7 of them total ~1.5s wall-time. Adding them to default `cargo test` would slow inner-loop test cycles.
- Files: `sim-rs/sim-cli/tests/determinism.rs` (every test is `#[ignore]`'d).
- Cause: Real simulation cost (200 slots × single producer × all event types).
- Improvement path: None needed — the on-demand invocation `cargo test --release -- --ignored determinism` is documented in CLAUDE.md and the test file's doc-comment. Note: m5-handoff.md §Gotchas #1 explicitly warns to run determinism tests in `--release` (test profile times out).

**Mempool overflow dominates over quote-drift eviction under the corrected calibration:**
- Problem: With ~150 KB/slot demand against a 32 MB mempool cap (`2 × eb_referenced_txs_max_size_bytes`) and ~5% RB cadence, the mempool saturates within ~250 slots. Beyond that, new-arrival admission rejections dominate over quote-drift evictions; inclusion rates land at ~1–3%.
- Files: `sim-rs/sim-core/src/sim/mempool_gate.rs` `try_admit` (no eviction of valid txs to make room — reject-only), `sim-rs/parameters/phase-2-sweep/protocol-base.yaml` `mempool-max-total-size-bytes`.
- Cause: Sustained-overload regime by construction — the corrected calibration is sized so the controller is the bottleneck, but mempool cap is hit first.
- Improvement path: Document whether sustained-overload is the right regime for the CIP write-up. Either raise the mempool cap (changes goldens), lower per-slot demand in the demand YAMLs, or accept the regime. Flagged in `docs/phase-2/calibration-fix-postmortem.md` §"Open follow-ups".

## Fragile Areas

**Pricing state has no rollback on fork / slot-battle (long-standing M1 limitation):**
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs:2018-2029` (`apply_priced_block` code comment), `sim-rs/sim-core/src/sim/linear_leios.rs:1948-1952` (the `apply_eb_priced_block` cousin), `sim-rs/sim-core/src/sim/linear_leios.rs` `finish_validating_rb_header`.
- Why fragile: `apply_priced_block` mutates `pricing` and `gate` immediately at `publish_rb`. Slot-battle replacement (lower VRF wins) at `finish_validating_rb_header` removes the losing block from `praos.blocks`, but does *not* undo: (a) the controller update, (b) the gate `on_inclusion` removals, (c) the `TXIncluded` events the losing block triggered. The mechanism spec at `mechanism-design.md:115` treats `c` as ledger state, so canonical-chain reasoning requires rollback-and-replay.
- Safe modification: Single-producer suites (current 7 phase-2 suites) cannot trigger slot battles by construction. M6 introduces the multi-producer CIP-0164 topology (`topology-cip-realistic.yaml`, 600 pools); under that, slot battles will fire and the rollback gap matters.
- Test coverage: Zero for slot-battle scenarios. m6-implementation-plan.md §"slot_battles_count" adds a metric to *quantify* the impact, not to fix the underlying gap.
- Fix approach: Snapshot the controller state + gate state at every `publish_rb`, allow `finish_validating_rb_header` to re-apply samples for the canonical chain after restoring the snapshot. Flagged across all five handoffs (m1 through m5) — never moved.
- **Update 2026-05-14:** RESOLVED via the chain-derived controller
  refactor (spike 007 ADOPT verdict). The accumulator described
  above no longer exists — `Eip1559Pricing`/`TwoLanePricing` are
  stateless policies, and every `LinearRankingBlock` carries its
  own `derived_quote: PerLaneQuote` + `window_aggregate:
  WindowAggregate` computed as pure functions of canonical
  predecessors. Slot-battle-losing blocks are discarded along with
  their `derived_quote`, so there is nothing to roll back. The
  WR-1 row in [`../REVIEW.md`](../REVIEW.md) Fix Status table
  reads RESOLVED 2026-05-14, citing
  [`../family-b-decision-2026-05-14.md`](../family-b-decision-2026-05-14.md)
  for the mechanism choice (EIP-1559-faithful Family B) and
  [`../mechanism-welfare-impact-2026-05-14.md`](../mechanism-welfare-impact-2026-05-14.md)
  for the empirical re-validation. File line references in the
  original entry above (`apply_priced_block` at `linear_leios.rs:2018`,
  `apply_eb_priced_block` at `:1948`) refer to the pre-refactor
  state and no longer match the current code.

**`PricingTick` is per-node; metrics use a single representative:**
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs` `emit_pricing_tick`, `sim-rs/sim-cli/src/metrics/collector.rs:251` (`representative_node: Option<String>`), `sim-rs/sim-cli/src/metrics/collector.rs:333` (`set_representative_node`).
- Why fragile: M5 made the representative *deterministic* (lexicographically smallest node name, pre-set by the runner) rather than first-tick-wins. But the *other nodes are dropped* property still holds — for any multi-producer suite with diverging per-node pricing state, the time-series under-reports cross-node disagreement.
- Safe modification: For single-producer suites this is moot (all nodes converge). For M6's multi-producer topology, the collector either needs aggregation (mean? min? max?) or to surface per-node series.
- Test coverage: `representative_node_lazy_fallback_picks_first_arrived`, `representative_node_pinning_overrides_first_arrival`, `out_of_order_events_do_not_roll_slot_backwards` in `sim-rs/sim-cli/src/metrics/collector.rs::tests` (M5 additions). No multi-producer divergence test.
- Fix approach: M6 adds `slot_battles_count` and `orphaned_pricing_samples` as quantification metrics. Full cross-node aggregation deferred to phase-3 or M7+.

**Cross-architecture determinism is asserted intra-arch only:**
- Files: `sim-rs/sim-cli/tests/determinism.rs`, `sim-rs/parameters/phase-2-sweep/suites/.goldens/`, `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`, `sim-rs/sim-core/src/sim/tests/m3_actors.rs`.
- Why fragile: The underlying math (`libm::pow`/`libm::round`, u128 rationals, integer arithmetic) is bit-stable across architectures, but the simulator inherits `f64` from `main` in non-pricing code paths — slot lottery (`sim-rs/sim-core/src/sim/lottery.rs:7-12`), distribution sampling (`sim-rs/sim-core/src/probability.rs`), config-derived fractions (`sim-rs/sim-core/src/config.rs:1149`), CPU multiplier (`sim-rs/sim-core/src/sim/cpu.rs:27`). These have not been hardened for cross-arch determinism.
- Safe modification: Pinned hashes reproduce on x86_64/glibc only. A second-arch build pipeline is required to detect regressions.
- Test coverage: Single-arch CI only.
- Fix approach: Infrastructure work outside phase-2's code scope. Flagged in `docs/phase-2/m5-handoff.md` §1 closure addendum. CIP write-up should surface this as a known reproducibility limitation.

**Goldens regeneration is one-shot and irreversible:**
- Files: `sim-rs/sim-cli/tests/determinism.rs` (`UPDATE_GOLDENS=1` writes goldens instead of asserting), `sim-rs/parameters/phase-2-sweep/suites/.goldens/*.sha256` (7 files).
- Why fragile: `UPDATE_GOLDENS=1` flips every committed golden in one shot. A change that intentionally flips one golden would obscure the others in the same diff.
- Safe modification: Always run `cargo test --release -- --ignored determinism` *without* `UPDATE_GOLDENS` first to identify which goldens change; only then regenerate. Bump the goldens tag (`m5-goldens-v2`, etc.) after regeneration so prior state is recoverable.
- Test coverage: The regeneration itself is not tested for selectivity.
- Fix approach: Per-suite `UPDATE_GOLDENS=<suite>` would be a minor ergonomic improvement. Flagged in `docs/phase-2/m5-handoff.md` §Gotchas #2.

**EB-validation-at-endorsement refuses entire endorsement on any stale tx:**
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs` `eb_endorsement_valid` (~lines 727+ inside `try_generate_rb`).
- Why fragile: If any tx in the candidate EB has `posted_fee > max_fee_lovelace` at the producer's current posted-lane quote, the producer refuses to endorse — dropping the entire endorsement, shipping the RB unendorsed. M2 chose this over mutating already-gossiped EB bodies, which is cleaner but blunt: one stale tx blocks the whole EB's service.
- Safe modification: Confirm the suite-level inclusion rate impact. Under sustained-overload (~1-3% inclusion) the loss of a few EB endorsements may be amplifying mempool saturation.
- Test coverage: `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:738+` exercises the refusal in single-tx scenarios. No suite-level coverage for cumulative endorsement-refusal rates.
- Fix approach: Document the cumulative endorsement-refusal rate in `diagnostics.log` as a calibration health metric.

**`incomplete_onchain_ebs` coordinates two distinct semantics:**
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs:297-298,750,1088,1215,1412`.
- Why fragile: The `HashSet<EndorserBlockId>` is used for both (a) "we endorsed this EB but don't have the body validated" (existing main behaviour) and (b) "I, as a node, owe the priced-block sample for this EB once it validates" (M1 addition). Any future change to insertion/removal timing must audit both paths.
- Safe modification: `apply_priced_block` only emits the EB sample if `get_validated_eb` returns `Some`; otherwise `finish_validating_eb` emits it later. If the timing of `incomplete_onchain_ebs` insertion/removal changes, audit both paths.
- Test coverage: Indirect — M2 scenario tests exercise the deferred-EB-sample path but don't isolate the two semantics.
- Fix approach: Split into two collections with distinct names. Flagged in m1-handoff.md §"`incomplete_onchain_ebs` coordination".

**`urgency: f64` on `Transaction` is a hot-path landmine:**
- Files: `sim-rs/sim-core/src/model.rs` (`urgency: f64` field on `Transaction`), `sim-rs/sim-core/src/tx_actors.rs` (lane-choice math).
- Why fragile: `urgency` is read **only** by the actor lane-choice math, which routes it through `libm::pow` + `libm::round` into `i128` lovelace before comparison. Reading it from any other simulation-affecting code path would re-introduce `f64` into the determinism contract and quietly flip cross-arch hashes.
- Safe modification: When touching anything that holds a `Transaction`, do not read `tx.urgency` directly. Re-route through the actor lane-choice helpers.
- Test coverage: The `f64`-in-hot-path injection would be caught by the suite-level goldens, but only on the dev arch. Cross-arch CI does not exist.
- Fix approach: Move `urgency` off `Transaction` and into actor-local state. Or wrap in a newtype with documented "do not read me from sim-affecting code" semantics. Document-only flagged in CLAUDE.md §Conventions.

## Scaling Limits

**Mempool cap = 32 MB (`2 × eb_referenced_txs_max_size_bytes`):**
- Current capacity: 32 MB total mempool bytes, no per-lane sub-cap.
- Limit: At ~150 KB/slot demand + ~5% RB cadence (corrected calibration), the mempool saturates within ~250 slots and admission rejections dominate over quote-drift evictions.
- Scaling path: Raise `mempool-max-total-size-bytes` in `protocol-base.yaml` (changes goldens), or reduce per-slot demand in the demand YAMLs.

**Single-producer topology covers 7 of 7 current phase-2 suites:**
- Current capacity: N=1 producer (`topology-single-producer.yaml`).
- Limit: Slot battles cannot fire; multi-mempool dynamics, per-node pricing-state divergence, and gossip-driven `LatencyEstimator` divergence are all moot. M6 introduces the CIP-0164 600-pool topology (`topology-cip-realistic.yaml`) to surface these.
- Scaling path: M6 (in progress per `docs/phase-2/m6-implementation-plan.md`).
- **Update 2026-05-14:** Superseded by the 2026-05-13 topology
  pivot. Phase-2 suites now reference
  `topology-realistic-100.yaml` (100 nodes, mass-stratified mainnet
  curve, multi-producer), so the "N=1 / slot battles cannot fire"
  framing above no longer applies. Slot-battle-driven controller
  contamination (WR-1) is closed *by design* via the chain-derived
  refactor — see the *Pricing state has no rollback...* entry
  above for the resolution chain. The representative-node
  aggregation question (next entry) remains open for multi-producer
  suites; only the rollback fragility is resolved.

**13-slot linear-Leios endorsement window:**
- Current capacity: `header_diffusion_time × 3 + linear_vote_stage_length + linear_diffuse_stage_length = 13 slots` enforced at `sim-rs/sim-core/src/sim/linear_leios.rs:707-716`.
- Limit: `expected_RB_gap > 13` is required or endorsement never fires. The current calibration (`rb-generation-probability: 0.05` → ~20-slot gap) holds with margin. Raising rb-prob without lowering the stage lengths re-creates the post-M5 calibration bug.
- Scaling path: Future calibration could reduce `linear_vote_stage_length` and `linear_diffuse_stage_length` to 0 (with single-producer + `vote_threshold = 1`, votes are immediate) to recover the high-cadence regime without the bug. Trade-off: less realistic Cardano timing. Flagged in `docs/phase-2/calibration-fix-postmortem.md` §"Open follow-ups".

## Dependencies at Risk

No external dependency risks identified. The pricing kernel relies only on `libm` (for bit-stable `pow`/`round`) and core Rust; the upstream `main` simulator pulls in `tokio`, `serde`, `rand`/`rand_chacha`, all standard Rust ecosystem.

## Missing Critical Features

**Anti-standard cap under FIFO fallback is not implemented:**
- Problem: The mechanism spec mandates an anti-standard cap when `LaneSelectionOrder::Fifo` is active. The simulator carries no `max_standard_block_fraction` knob and `priority_first` selection is hard-wired in the current 7 phase-2 suites.
- Blocks: Any future FIFO-based experiment. Identified in `docs/phase-2/mechanism-design.md:312` as the single residual divergence from the spec's methodology table after M5's 8 → 1 reduction.
- Files: `sim-rs/sim-core/src/tx_pricing/mod.rs` `lane_selection_order`, `sim-rs/sim-core/src/sim/linear_leios.rs` `sample_from_mempool_lane_aware`.

**Fork-resolution metric for multi-producer suites (in-progress, M6):**
- Problem: With the multi-producer topology arriving in M6, `slot_battles_count` and `orphaned_pricing_samples` need to be observable for the welfare write-up to be honest about cross-node noise.
- Blocks: M6 welfare comparison.
- Files: To be added per `docs/phase-2/m6-implementation-plan.md` — new `Event::LinearPricingSampleApplied`, `RunSummary` fields, collector accumulator.

**Cross-architecture CI build pipeline:**
- Problem: Determinism is asserted intra-arch (x86_64 / glibc) only.
- Blocks: External reproducibility claims in the CIP.
- Files: CI infrastructure outside the repo (GitHub Actions or equivalent). Flagged in m5-handoff.md §Phase-2 closure addendum item 1.

## Test Coverage Gaps

**Cross-architecture goldens:**
- What's not tested: Whether the M2/M3 unit-test goldens and the 7 suite-level baseline goldens reproduce bit-identically on aarch64, RISC-V, or non-glibc x86_64.
- Files: `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`, `sim-rs/sim-core/src/sim/tests/m3_actors.rs`, `sim-rs/parameters/phase-2-sweep/suites/.goldens/`.
- Risk: A non-pricing `f64` code path (probability sampling, slot lottery, CPU multiplier) could silently produce different hashes on a different arch, invalidating reproducibility claims.
- Priority: Medium. Phase-2's hot paths are integer/rational; the risk is in inherited `main` code.

**Slot-battle / multi-producer pricing-state divergence:**
- What's not tested: All 7 phase-2 suites are single-producer. The fork-rollback gap and the representative-node aggregation gap are moot but real.
- Files: `sim-rs/parameters/phase-2-sweep/suites/*.yaml` (all reference `topology-single-producer.yaml`).
- Risk: M6's multi-producer topology will surface the gaps simultaneously; without dedicated unit tests, regressions could land via the goldens silently.
- Priority: High once M6 lands.
- **Update 2026-05-14:** Partially resolved. The
  **fork-rollback gap** is closed by the chain-derived
  controller refactor — orphan blocks carry their own
  `derived_quote` which is discarded with the block, so there is
  no controller divergence to detect. The
  **representative-node aggregation gap** remains: for
  multi-producer suites the metrics collector still emits a
  single-node `PricingTick` series (lex-smallest node), which
  under-reports cross-node disagreement when nodes diverge for
  reasons orthogonal to slot battles (gossip latency,
  per-node `LatencyEstimator` state). The chain-derived design
  makes the canonical-chain `derived_quote` agree across nodes,
  but per-node *gate state* (admission rejections, evictions)
  can still differ. Phase-2 suites now reference
  `topology-realistic-100.yaml`, not `topology-single-producer.yaml`
  — the file-list line above is therefore historical. Tracked
  separately for any phase-3 / multi-producer welfare-comparison
  work.

**EB partition activation under real demand:**
- What's not tested: The EB priority partition trigger (saturation OR "≥1 valid unselected tx but none fits residual bytes"). With ~150 KB/slot demand vs a 16 MB EB body, the EB never reaches capacity, so the partition's binary trigger never fires.
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs` `select_eb_with_partition`. Test coverage in `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`, `m3_actors.rs` exercises the trigger via constructed scenarios but not via real-demand suites.
- Risk: Conclusions about "the EB partition delivers one RB-worth of guaranteed priority service under saturation" rest on spec-faithfulness, not direct measurement. Flagged in `docs/phase-2/calibration-fix-postmortem.md` §"Open follow-ups" item 1.
- Priority: Medium. A higher-demand or smaller-EB suite could exercise this.

**Mempool admission edge cases:**
- What's not tested: The reject-only-on-full-mempool behaviour is exercised in `sim-rs/sim-core/src/sim/mempool_gate.rs::tests` (~6 unit tests). No suite-level test verifies the cumulative rejection-vs-eviction breakdown matches the calibration intent.
- Files: `sim-rs/sim-core/src/sim/mempool_gate.rs:336-444`.
- Risk: Under the corrected calibration, ~97-99% of demand is rejected at admission. Whether that's the right regime is a calibration question; a regression that flipped admission to drop-then-evict semantics would be caught by the goldens but not by unit tests.
- Priority: Low.

**Attacker model interaction with pricing:**
- What's not tested: `behaviours.withhold_txs = true` bypasses `MempoolGate::try_admit` entirely. Pricing-state interaction with attacker txs (`max_fee_lovelace = u64::MAX` default) is not modelled.
- Files: `sim-rs/sim-core/src/sim/linear_leios/attackers.rs`, `sim-rs/sim-core/src/sim/linear_leios.rs:1464-1526`.
- Risk: No current suite enables late-tx/late-eb attackers. The path is dormant. Flagged in m1-handoff.md.
- Priority: Low (dormant).

**Reachability of the multiplier-floor enforcement edge cases:**
- What's not tested: Constructor-time enforcement (`TwoLanePricing::new` raises priority's initial quote up to the floor if needed) is unit-tested in `sim-rs/sim-core/src/tx_pricing/two_lane.rs::tests`. Post-update enforcement on `quote_per_byte` with u128 intermediates is unit-tested. No suite directly drives a controller into the floor-bound regime to verify suite-level behaviour.
- Files: `sim-rs/sim-core/src/tx_pricing/two_lane.rs:185-210` (floor enforcement).
- Risk: Goldens would catch a regression, but only on the dev arch.
- Priority: Low.

---

*Concerns audit: 2026-05-13*
