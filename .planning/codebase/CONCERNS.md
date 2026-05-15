# Codebase Concerns

**Analysis Date:** [2026-05-15]

This document catalogues active concerns on the `dynamic-experiment`
branch of `arc-tiered-pricing` (phase-2 dynamic-pricing simulator).
The authoritative review-tracker is `.planning/REVIEW.md` (Fix
Status table); concerns below reflect that tracker plus operational
gaps in the build/test/run regime as of 2026-05-15.

**Headline status update:** WR-1 (pricing-state contamination on
slot-battle reorg) is **RESOLVED 2026-05-14** by the chain-derived
(Family B, EIP-1559-faithful) refactor — there is no longer any
node-local mutable controller state for orphan blocks to
contaminate. See [Resolved concerns](#resolved-concerns-for-context)
at the end of this document for the audit trail.

## Tech Debt

### Mixed serde casing across persisted artefacts (low priority)

- Issue: YAML configs and the runner's `Manifest` / `JobEntry` use
  `#[serde(rename_all = "kebab-case")]`; `RunSummary` uses Rust
  snake_case (no `rename_all`). Both shapes coexist on disk in
  persisted artefacts under `sim-rs/output/`.
- Files: `sim-rs/sim-cli/src/runner.rs:45,54,68`,
  `sim-rs/sim-cli/src/suite.rs:16,29,38`,
  `sim-rs/sim-cli/src/metrics/collector.rs` (RunSummary).
- Impact: Confusing inconsistency for readers of the persisted JSON.
  Standardising would invalidate every persisted manifest under
  `sim-rs/output/` and force re-runs of all 72 (job, seed) pairs
  pinned by the M5 goldens.
- Fix approach: Defer. Future schema additions should match the
  surrounding type's existing convention. Listed in
  `CLAUDE.md §Conventions / gotchas` as an explicit "not worth the
  churn for M5" decision.

### RB-reduced overlays are full replacements, not stacked overlays

- Issue: `protocol-rb-reduced-{half,third,quarter}.yaml` duplicate
  every field of `protocol-base.yaml` and override only
  `rb-body-max-size-bytes`. The runner's `JobOverrides` picks
  `overrides.protocol` OR `default_protocol`, never both.
- Files:
  `sim-rs/parameters/phase-2-sweep/protocol-base.yaml`,
  `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-half.yaml`,
  `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-third.yaml`,
  `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-quarter.yaml`,
  `sim-rs/sim-cli/src/runner.rs` (`JobOverrides`).
- Impact: **Any future addition to `protocol-base.yaml` must be
  manually propagated to all three RB-reduced overlays.** Easy to
  forget; would cause the rb-scarcity suite goldens to silently
  diverge from `protocol-base.yaml` semantics.
- Fix approach: Extend `JobOverrides` with stacked
  `protocol_overlay: Vec<PathBuf>` semantics. Deferred enhancement;
  documented in `CLAUDE.md §Conventions / gotchas`.

### WR-7: `ActorComponent` reallocation on the hot path

- Issue: `run_actors_for_slot` clones `ActorComponent` data into
  `ComponentInputs` per slot, then rebuilds a temporary
  `ActorComponent` for sampling — 4 components × 1000 slots × 100
  nodes ≈ 400k short-lived allocations per run.
- Files:
  `sim-rs/sim-core/src/sim/linear_leios.rs:2268-2429`.
- Impact: Cumulative perf cost (worst in larger-topology suites).
  Also a clarity issue — the actor lane-choice math has two
  materialisations of the same struct, obscuring the f64 →
  `libm::round` → i128 pipeline.
- Fix approach: Refactor `ActorComponent`'s sampling helpers to take
  fields by reference; pass `&ActorComponent` directly from
  `actor_state.profile.components`. **Deferred** per REVIEW.md F4
  (out of v1 scope, ~50–100 lines touching lane-choice math,
  needs careful human review).

### WR-2: gate-reject vs mempool-reject collapsed into a single bool

- Issue: `try_add_tx_to_mempool` returns `false` for both
  `gate.try_admit(...).is_err()` and `mempool.try_insert(...) ==
  false`. The gate's rich `AdmissionRejection` enum
  (`InsufficientMaxFee`, `ByteCapExceeded`, `FeeOverflow`) is
  thrown away.
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs:1728-1751`,
  `sim-rs/sim-core/src/sim/mempool_gate.rs:36-50` (rejection enum).
- Impact: Metrics layer cannot distinguish "fee budget exceeded"
  from "byte cap exceeded" rejections. Important for interpreting
  the current sustained-overload calibration regime (~97-99%
  rejection rates per `docs/phase-2/calibration-fix-postmortem.md`).
- Fix approach: Propagate `AdmissionRejection` upward; emit an
  `AdmissionRejected { reason }` event (backwards-compatible
  addition). **Deferred** per REVIEW.md F3 — non-trivial
  public-API change with potential golden impact; needs a design
  pass before code.

### Legacy protocols in `sim-core/src/sim/` (informational, not a defect)

- Files: `sim-rs/sim-core/src/sim/leios.rs` (1833 lines),
  `sim-rs/sim-core/src/sim/stracciatella.rs` (1234 lines),
  `sim-rs/sim-core/src/sim/tx.rs` (143 lines).
- Status: Untouched by phase-2 except for minor signature changes
  (e.g. `slot` added to `track_transaction_generated` calls, pinned
  to 0 for the non-actor paths). Dormant under the phase-2 actor
  profile — `TransactionProducer` in `tx.rs` is silenced when the
  actor profile is set.
- Impact: Adds ~3,200 lines to `sim-core/` that phase-2 does not
  exercise. Inflates the size budget reported in `CLAUDE.md §Size
  sanity check`. The `HashMap<NodeId, NodeState>` iterated by
  `tx.rs:82-86` is pre-existing on `main` and would be a
  determinism risk if reactivated, but is dead under phase-2.
- Fix approach: None planned — keeping the upstream Leios variants
  preserves the rebuild-on-top-of-`main` contract from
  `docs/phase-2/implementation-plan.md`. Informational only.

## Known Bugs

### TODOs in legacy & current code paths (low priority)

- `sim-rs/sim-core/src/sim/linear_leios.rs:713` — "TODO: should
  send to producers instead (make configurable)" on the
  RB-fallback gossip path. Mirrors the legacy comment in
  `sim-rs/sim-core/src/sim/stracciatella.rs:405`.
- `sim-rs/sim-core/src/sim/linear_leios.rs:1219,1367` — "TODO:
  freshest first" on tx-fetch ordering. Affects gossip throughput
  modelling, not pricing correctness.
- Impact: No simulation-output correctness implications; cosmetic
  / future-work markers.
- Fix approach: Address opportunistically; not blocking.

### Mempool-gate cooperation invariants (mitigated; informational)

- Issue: The mempool's internal "queue for later promotion" branch
  is dead code under the gate-is-sole-byte-cap-authority
  invariant, but its existence means a future config knob (soft
  vs hard cap) could silently reintroduce a path where txs land in
  the active mempool without gate state.
- Files: `sim-rs/sim-core/src/sim/linear_leios.rs:2553-2694`.
- Status: WR-3 **applied 2026-05-13** —
  `debug_assert_eq!(mempool_max_size_bytes, gate.max_total_size_bytes)`
  in `LinearLeiosNode::new`, plus
  `debug_assert!(self.queue.len() <= self.mempool_count)` invariant
  in `Mempool::try_insert`. Listed here only as a fragile-area
  reminder for future code touching the mempool layer.

## Security Considerations

### `partition_activated` is a producer claim, not a body-derivable property

- File: `sim-rs/sim-core/src/sim/linear_leios.rs`
  (`select_eb_with_partition`, `LinearEndorserBlock.partition_activated`).
- Risk: Phase-2 simulates honest producers only. A dishonest
  producer could set `partition_activated` to either value
  regardless of the actual mempool state, manipulating which
  posted-priority txs get refunded down to standard fee.
- Current mitigation: Honest-producer assumption inherent in the
  M3 design choice to store `partition_activated` on the EB
  header (endorsers/producers agree by construction).
- Recommendations: A published-attacker-model write-up (CIP) would
  need to either (a) move the trigger to a body-derivable
  invariant, or (b) explicitly model "honest producer" as a
  security assumption. Cross-cutting observation #5 in
  `.planning/REVIEW.md`.

## Performance Bottlenecks

### Peak RSS scales linearly in `--parallelism` for `experiment-suite`

- Files: `sim-rs/sim-cli/src/runner.rs` (parallel job dispatch);
  `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`.
- Problem: `experiment-suite run` and `experiment-suite verify`
  run (job, seed) pairs concurrently. Default cap is
  `min(available_parallelism(), 8)`; each parallel job owns its
  own simulator state (config, topology, mempool, metrics
  collector) and runs inside its own OS thread + per-thread
  `current_thread` tokio runtime.
- Cause: `Simulation` contains `Box<dyn Actor>` which isn't
  `Send`, forcing per-thread runtimes; each thread holds a full
  copy of the 100-node topology + actor + mempool state for the
  duration of its job.
- Impact: With the default `topology-realistic-100.yaml` and 8
  parallel jobs, peak RSS stays comfortably under 32 GB on the
  dev machine but is the practical cap on home-machine runs.
  Stacking with `scripts/run-parallel-suites.sh` (parallelises
  *across* suites) multiplies the total: total tokio worker
  threads ≈ cross-suite K × intra-suite P. Easy to OOM if both
  knobs are raised without measurement.
- Improvement path: Document `--parallelism N` (`-P N`) usage in
  per-machine README guidance. For larger topologies, lower the
  intra-suite cap explicitly. A future optimisation pass on the
  hot allocations identified in WR-7 would reduce per-job
  baseline RSS but is deferred.

### WR-7 cumulative allocation cost (see Tech Debt above)

- ~400k short-lived `ComponentInputs` / `ActorComponent`
  allocations per run with 4 components × 1000 slots × 100 nodes.
  No fix yet; deferred to v2.

## Fragile Areas

### Determinism is intra-architecture only; cross-arch CI pipeline unbuilt

- Files: `sim-rs/sim-cli/tests/determinism.rs` (suite-level
  goldens runner), `sim-rs/parameters/phase-2-sweep/suites/.goldens/`
  (7 pinned hash files), `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`
  and `sim-rs/sim-core/src/sim/tests/m3_actors.rs` (unit-test
  goldens with constants pinned in source).
- Why fragile: Determinism is asserted on three levels (unit-test
  goldens, `experiment-suite verify`, suite-level
  `--release -- --ignored determinism`) but all three pin
  **intra-arch**. The development machine is x86_64 / glibc; no
  second-architecture build pipeline exists to detect
  cross-platform divergence.
- Reproducibility properties of the pricing kernel: the chain-
  derived controller computation is bit-stable across architectures
  by construction (u128 rationals, `libm::pow`, `libm::round`,
  `libm::exp` throughout). Pricing-state-affecting math is
  integer/u128 / rational, with no plain `f64` in
  simulation-affecting paths in the pricing kernel itself.
- Cross-arch gaps inherited from upstream `main`: see "Historical
  f64 inheritance" below.
- Safe modification: Maintain the integer/rational discipline in
  any new pricing-kernel code; route any new f64 → integer
  conversion through `libm::round` → i128 before any compare.
- Fix approach: A second-arch build pipeline (e.g. aarch64) +
  CI job that reruns the suite-level goldens. Flagged in the
  Family B follow-on work list and the m5 handoff. Infrastructure
  work outside phase-2's code scope; not yet scheduled.

### Historical f64 inheritance from upstream non-pricing code paths

- Files: `sim-rs/sim-core/src/sim/lottery.rs:7-12,49`
  (`compute_target_vrf_stake`, `vrf_probabilities`,
  `Lottery::run`), `sim-rs/sim-core/src/sim/linear_leios.rs:428-441`
  (the `endorsement_window_priced_blocks` 2-sigma Poisson bound —
  noted in CR-1, doc-comment correctly flags that `f64::sqrt` is
  not mandated bit-exact across IEEE-754 implementations), various
  distribution-sampling paths in `sim/driver.rs`, `sim/slot.rs`.
- Why fragile: These code paths predate the phase-2 rebuild and
  feed simulation-affecting decisions (slot lottery, RB header
  diffusion, propagation timing). They are covered by the
  intra-arch suite goldens but have not been hardened for
  cross-arch determinism.
- Impact: The pricing kernel's bit-stability guarantee does not
  extend end-to-end. Cross-arch divergence in (e.g.) the slot
  lottery would change which producer wins which slot, cascading
  into different `Transaction` IDs and different `derived_quote`
  trajectories.
- Safe modification: Treat the existing intra-arch goldens as
  the only reproducibility floor when modifying upstream code
  paths. Do **not** introduce new f64 into phase-2 pricing or
  mempool code (the CLAUDE.md hard rule).
- Fix approach: Replace `f64::sqrt` with `libm::sqrt` or switch
  to integer Newton's-method sqrt in
  `endorsement_window_priced_blocks` (CR-1 fix sketch in
  `.planning/REVIEW.md`). The wider lottery / propagation paths
  remain f64 and would need a coordinated hardening pass. Listed
  as "Optional: explicit cross-architecture determinism CI" in
  `.planning/family-b-decision-2026-05-14.md §Follow-on work`.

### `MetricsCollector::is_representative` lazy fallback

- File: `sim-rs/sim-cli/src/metrics/collector.rs:577-585`.
- Status: WR-6 **applied 2026-05-13** —
  `debug_assert!(self.representative_node.is_some(), ...)` added;
  lazy-fallback split into release-only test + debug-only
  `#[should_panic]` test that locks the assertion in. Listed
  here only to flag the latent regression risk if the runner's
  pre-pin call (`runner.rs:581-583`) is ever deleted in a
  refactor.

### `Eip1559Pricing::step` saturating u128 ops in release builds

- File: `sim-rs/sim-core/src/tx_pricing/single_lane.rs:189-199`.
- Status: WR-4 **applied 2026-05-13** — `Eip1559Settings::validate`
  now requires
  `window_length × target_num × max_change_denominator ≤ 2^23`,
  keeping the u128 intermediates safe for per-sample bytes up to
  2^40. `saturating_mul`s left in as belt-and-braces. Listed
  here as a fragile-area reminder.

## Test Coverage Gaps

### 12 demand-regime suites are goldens-unpinned

- Files: The 7 pinned suites are
  `sim-rs/parameters/phase-2-sweep/suites/phase-2-{eip1559-robustness,eip1559-smoothing,priority-only-rb-reserved,priority-only-unreserved,two-lane-both-dynamic,rb-scarcity,urgency-inversion}.yaml`
  (each with a corresponding `.sha256` under `.goldens/`). The 12
  unpinned suites are the demand-regime suites under
  `paper_like_*` and `sundaeswap_*` profiles:
  `sim-rs/parameters/phase-2-sweep/suites/phase-2-{congested,moderate,realistic,sundaeswap}-{singlelane,priority-only,both-dynamic}.yaml`.
- What's not pinned: Suite-level golden hashes for the 12
  demand-regime suites. The 7 mechanism-characterisation suites
  pinned by M5 still cover all four pricing variants
  (single-lane, RB-reserved, un-reserved, both-dynamic).
- Risk: Per-suite event-stream determinism for the demand-regime
  suites can only be checked via `experiment-suite verify`
  against persisted `RunSummary.pricing_event_stream.sha256`
  values, not against externally-pinned `.goldens/` files. A
  regression that flips event-stream hashes between two `verify`
  invocations (without re-running `run`) would be detected; a
  regression that affects only freshly-run suites' welfare
  numbers without flipping pricing event hashes would not be
  surfaced by the goldens regime.
- Priority: Medium. Mitigated by the integer/rational discipline
  in the pricing kernel and by the 7 pinned mechanism suites
  exercising every pricing-backend code path.
- Fix approach: Add the 12 demand-regime suites to
  `sim-rs/sim-cli/tests/determinism.rs` and regenerate via
  `UPDATE_GOLDENS=1 cargo test --release -- --ignored
  determinism`. Cost is one full re-run of those suites
  (incremental — the runner is resumable).

### Property-based test for EIP-1559-faithful cadence

- Status: REVIEW.md F2 **OPEN**. One regression test
  (`admission_uses_post_step_quote_at_chain_tip` in
  `sim-rs/sim-core/src/sim/mempool_gate.rs::tests`) was added
  during bug-1 fix.
- What's not tested: A property test asserting "the controller
  steps exactly N times over a canonical chain of length N", or
  equivalently "deferred-EB validation fires zero controller
  steps". Would lock in the Family B commitment against future
  regressions.
- Priority: Medium. The commitment is enforced by the
  chain-derived implementation pattern (no node-local
  accumulator exists), but a property test would prevent a
  future "convenience" refactor from silently reintroducing the
  2-step behaviour.

### Required Family B re-run of all 19 phase-2 suites

- Status: REVIEW.md F1 **OPEN**. The 33-job sundaeswap smoke is
  sufficient for the Family B decision and welfare-impact
  characterisation
  (`.planning/mechanism-welfare-impact-2026-05-14.md`), but
  suite-level publication numbers should come from the full
  sweep × 3 seeds.
- Cost: Hours per full sweep × 3 seeds. The runner is resumable;
  only chain-derived runs need to be (re-)generated.
- Priority: High for publication-grade numbers; not blocking
  for the M5 / Family B docs cascade.

## Resolved concerns (for context)

### WR-1: pricing-state contamination on slot-battle reorg — RESOLVED 2026-05-14

- Files (historical, for archaeology):
  `sim-rs/sim-core/src/sim/linear_leios.rs:1068-1091`
  (`finish_validating_rb_header`) — was the site where the losing
  block was removed from `praos.blocks` and its certified EB
  dropped from `incomplete_onchain_ebs`, but the controller
  update and gate `on_inclusion` removals were not rolled back.
- Rationale for resolution: The chain-derived refactor (spike
  007 ADOPT, applied 2026-05-14) replaced node-local mutable
  controller state with pure-function-of-chain semantics. Every
  `LinearRankingBlock` carries `derived_quote: PerLaneQuote` as
  a pure function of `parent.derived_quote` +
  `parent.window_aggregate` + samples in canonical predecessors.
  Sibling-block orphans from slot battles are discarded with the
  block; they carry their own `derived_quote` which is
  discarded along with the block, so no residual controller
  mutation enters the canonical chain. By construction the
  contamination cannot occur.
- Audit trail: `.planning/REVIEW.md §Fix Status` (WR-1 row),
  `.planning/family-b-decision-2026-05-14.md` (publication-
  committed mechanism choice),
  `.planning/mechanism-welfare-impact-2026-05-14.md` (empirical
  welfare-impact characterisation across 33 sundaeswap-smoke
  jobs), `.planning/spikes/007-chain-derived-controller/README.md`
  (ADOPT verdict, design spec),
  `.planning/chain-derived-controller-PLAN.md` (implementation
  deltas).
- Single-lane EIP-1559 welfare drop under chain-derivation:
  Family B caused single-lane median welfare to fall by ~2
  orders of magnitude in the 33-job smoke (with 2 sign flips
  at `d4_t50_w32` and `d8_t25_w32`). This is **not a bug** —
  it is a more honest characterisation of single-lane EIP-1559's
  narrower welfare regime under faithful one-step-per-canonical-block
  cadence. The two-lane un-reserved arms (which carry the
  publication's headline claims) remain mechanism-robust
  (median |Δ%| ≤ 17%, no sign flips). See
  `.planning/mechanism-welfare-impact-2026-05-14.md §Recommendation`.

### Calibration bug (rb-prob 1.0 → 0.05) — RESOLVED

- File (historical): `sim-rs/parameters/phase-2-sweep/protocol-base.yaml`
  (was `rb-generation-probability: 1.0`, now `0.05`);
  `sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml`
  (was `stake: 1`, now `100000`).
- Rationale: `rb-prob: 1.0` produced 1-slot RB gaps which never
  cleared the linear-Leios 13-slot endorsement window, silently
  preventing EBs from ever landing on chain. Cardano-realistic
  cadence (`rb-prob: 0.05`, ~20-slot gaps) clears the window and
  exercises the full RB+EB endorsement path. The
  `multiplier_floor = 4` calibration choice survives independently
  of this fix; see `docs/phase-2/calibration-fix-postmortem.md`.

---

*Concerns audit: 2026-05-15*
