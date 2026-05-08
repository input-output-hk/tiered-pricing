# M5 → phase-2 closure handoff

Audience: project owners drafting the CIP and external write-up
once the `dynamic-experiment` branch is signed off, and any future
collaborator landing on the branch. Read alongside
[mechanism-design.md](mechanism-design.md) and
[implementation-plan.md](implementation-plan.md) — those are
authoritative; this note is the M5 delta on top of
[m1-handoff.md](m1-handoff.md), [m2-handoff.md](m2-handoff.md),
[m3-handoff.md](m3-handoff.md), and [m4-handoff.md](m4-handoff.md).

M5 is the final phase-2 milestone, so this handoff also doubles as
the closure note. The §Phase-2 closure addendum at the top is the
short list of items the CIP / external write-up needs to surface;
the rest of the handoff follows the m4-handoff shape.

## Phase-2 closure addendum (for the CIP / external write-up)

1. **Cross-arch CI verification is not yet built.** Determinism
   on `dynamic-experiment` is asserted intra-architecture only,
   via three layers: the M2/M3 unit-test golden constants in
   [sim-rs/sim-core/src/sim/tests/](../../sim-rs/sim-core/src/sim/tests/),
   the `experiment-suite verify` re-run-and-hash check, and
   M5's suite-level baseline goldens at
   [sim-rs/parameters/phase-2-sweep/suites/.goldens/](../../sim-rs/parameters/phase-2-sweep/suites/.goldens/).
   The underlying math (`libm::pow`/`libm::round`, u128 rationals,
   integer arithmetic) is bit-stable across architectures given
   identical inputs, but the simulator inherits f64 from `main`
   in non-pricing code paths (slot lottery, propagation,
   distribution sampling) which has not been hardened for
   cross-arch determinism. **Building a second-arch CI pipeline
   is infrastructure work outside phase-2's code scope.** Surface
   this as a known limitation in the CIP's reproducibility
   section.

2. **Calibration choices the simulator made for spec-open
   questions** (the ones the spec leaves open; full table with
   re-calibration cost in
   [mechanism-design.md §Calibration choices](mechanism-design.md)
   and [CLAUDE.md](../../CLAUDE.md)):
   - Window length 32 for capacity-varying signals (length 1
     for RB-reserved priority).
   - Update cadence per priced block.
   - Un-reserved priority signal source = option 1
     (`priority_paying_bytes / total_block_capacity`).
   - Both-dynamic standard signal = capacity-weighted aggregate
     against `eb_referenced_txs_max_size_bytes` for EBs (and no
     standard sample on RB-reserved RBs).
   - Default actor `max_fee_policy = ScaledOverLaneQuote{4, 1}`
     (4× quote-drift headroom).
   - `multiplier_floor = 4` in `phase-2-rb-scarcity` and
     `phase-2-urgency-inversion` (vs the spec default 16);
     calibration choice driven by the M3 single-producer +
     `rb-generation-probability: 1.0` topology — at 16 priority
     demand is too low to surface controller drift in 200 slots.
   - Default `target_inclusion_blocks` (priority=1, standard=4)
     seeds the actor `LatencyEstimator`.
   - Mempool cap default = `2 × eb_referenced_txs_max_size_bytes`.

   These are **calibration assumptions, not approximations** — the
   simulator picked defaults the spec leaves open. The CIP should
   surface them so a future deployment-side calibration sweep
   knows what to revisit.

3. **The 7 suites' answers to the down-select questions**, with
   forward-pointers:
   - `phase-2-eip1559-robustness` — single-lane EIP-1559 stays
     stable across `D ∈ {4, 8, 16}` and `target ∈ {0.25, 0.5,
     0.75}`. Output:
     [sim-rs/output/phase-2/eip1559-robustness/metrics_comparison.txt](../../sim-rs/output/phase-2/eip1559-robustness/metrics_comparison.txt).
   - `phase-2-eip1559-smoothing` — window length sweep
     (16/32/64). Output:
     [eip1559-smoothing/metrics_comparison.txt](../../sim-rs/output/phase-2/eip1559-smoothing/metrics_comparison.txt).
   - `phase-2-priority-only-rb-reserved` — multiplier-floor
     sweep on the RB-reserved priority-only-static-standard
     mechanism. Output:
     [priority-only-rb-reserved/metrics_comparison.txt](../../sim-rs/output/phase-2/priority-only-rb-reserved/metrics_comparison.txt).
   - `phase-2-priority-only-unreserved` — same multiplier sweep,
     no partition. Output:
     [priority-only-unreserved/metrics_comparison.txt](../../sim-rs/output/phase-2/priority-only-unreserved/metrics_comparison.txt).
   - `phase-2-two-lane-both-dynamic` — both-dynamic, partitioned
     and un-partitioned. Output:
     [two-lane-both-dynamic/metrics_comparison.txt](../../sim-rs/output/phase-2/two-lane-both-dynamic/metrics_comparison.txt).
   - `phase-2-rb-scarcity` (M4 reframing) — RB-capacity scarcity
     restated as a two-lane experiment.
     [README](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.README.md)
     +
     [output](../../sim-rs/output/phase-2/rb-scarcity/metrics_comparison.txt).
     The headline finding (m4-handoff §"`rb-scarcity` has a
     steep cliff between half and third RB capacities") is that
     under the M3 single-producer + rb-gen-prob=1 calibration,
     RB scarcity does not push txs onto the standard lane — it
     pushes them off-chain (mempool-resident, unincluded). The
     experimental answer is in the priority inclusion gradient
     and `priority_lane_retained_value_ratio`.
   - `phase-2-urgency-inversion` (M4 reframing) — urgency
     inversion restated.
     [README](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.README.md)
     +
     [output](../../sim-rs/output/phase-2/urgency-inversion/metrics_comparison.txt).
     The signal manifests in `refund_lovelace = 0` per
     mispriced component (vs ~13B for correctly-priced) — a
     refund-budget signal, not the eviction-cascade signal the
     experimental design originally anticipated. Both signals
     are economically real; the README explains the
     calibration-dependent shape.

4. **Welfare evidence on `pricing-sim-base` is invalidated**
   by the rebuild. The down-select arguments for the CIP must
   be sourced from `dynamic-experiment` only. The
   `pricing-sim-base` branch remains observable as prior art
   for calibration intuitions but no welfare numbers carry across.
   ([implementation-plan.md line 7](implementation-plan.md#L7).)

## Branch state

`dynamic-experiment` (no worktree, per project preference).
M1 + M2 + M3 + M4 + M5 ship as one accumulated delta on this
branch.

- Build: `cd sim-rs && cargo build --release` clean.
- Tests: `cd sim-rs && cargo test --workspace` → **132 green**:
  - 113 sim-core lib tests (unchanged from M3 — the M5
    collector-coverage additions are sim-cli-side).
  - 14 sim-cli lib tests (M4's 12 + 2 new M5 collector coverage
    tests).
  - 4 gen-test-data + 1 sim-cli main (pre-existing).
- Determinism goldens: `cd sim-rs && cargo test --release --
  --ignored determinism` → 7 green in ~0.3s.
- 7 phase-2 suites all run to completion.
  `experiment-suite verify` on all 7 reports
  `determinism verify ok` for every (job, seed) pair.
- Branch tag: `m5-goldens-v1` (annotated) **will annotate the
  commit that lands M5**, applied immediately after the M5
  commit so the goldens at
  [sim-rs/parameters/phase-2-sweep/suites/.goldens/](../../sim-rs/parameters/phase-2-sweep/suites/.goldens/)
  are recoverable from the tag if a future change inadvertently
  flips them. As of this handoff being written the tag does not
  yet exist; it is a verification step to be applied after the
  M5 commit lands.

The hard rules (no `pricing-sim-base` content; no f64 in
simulation-affecting state) held throughout. M5 added no
simulation-affecting code paths; the new code is integration
tests, the `MetricsCollector::set_representative_node` setter
(metrics-side), and a `pub` qualifier on `runner::run_job`.

## What M5 delivered

### New files

- [CLAUDE.md](../../CLAUDE.md) at the repository root —
  layout, mechanism abstractions, integer/rational vs f64 split,
  determinism scope, calibration choices, run commands,
  conventions, size sanity check.
- [sim-rs/sim-cli/tests/determinism.rs](../../sim-rs/sim-cli/tests/determinism.rs)
  — cargo integration test: one `#[test] #[ignore]` per suite.
  Each test loads the suite YAML, rebases relative paths onto
  `sim-rs/`, redirects `output_dir` to a `tempfile::TempDir`,
  runs the baseline (job, seed=1) via `runner::run_job`, and
  asserts the freshly-computed
  `RunSummary.pricing_event_stream_sha256` equals the committed
  golden. With `UPDATE_GOLDENS=1` the test writes the golden
  instead of asserting.
- [sim-rs/parameters/phase-2-sweep/suites/.goldens/](../../sim-rs/parameters/phase-2-sweep/suites/.goldens/)
  — 7 `<suite-name>.sha256` files, one per suite, format
  `<job-name> <seed> <hex-hash>`. Initial values:

  | Suite | Baseline (job, seed) | Hash prefix |
  |---|---|---|
  | phase-2-eip1559-robustness | d8_target0.5_window32 / 1 | `599b54ae…` |
  | phase-2-eip1559-smoothing | window32 / 1 | `599b54ae…` |
  | phase-2-priority-only-rb-reserved | multiplier_x4 / 1 | `2aa1e7f4…` |
  | phase-2-priority-only-unreserved | multiplier_x4 / 1 | `e056fb3c…` |
  | phase-2-two-lane-both-dynamic | partitioned_x4 / 1 | `2aa1e7f4…` |
  | phase-2-rb-scarcity | rb_baseline / 1 | `2aa1e7f4…` |
  | phase-2-urgency-inversion | correctly_priced / 1 | `2aa1e7f4…` |

  The `2aa1e7f4…` hash repeats across four suites because their
  baseline jobs converge to the same pricing-event stream under
  seed 1 (single-producer + paper_like_congested + similar
  partitioned-priority configs in which the priority controller
  drives the same admission/inclusion sequence). 6 of 7
  baselines reduce to 3 distinct hashes — the suite-level regime
  is a **tripwire** (catches simulator-wide drift) rather than
  high-fidelity per-suite drift detection. The
  `.goldens/<suite>.sha256` format leaves room for a future M to
  pin multiple (job, seed) pairs per suite (one line each) and
  the parser would extend cleanly; deferred.
- [docs/phase-2/m5-handoff.md](m5-handoff.md) — this file.

### Modified files

- [docs/phase-2/mechanism-design.md](mechanism-design.md) —
  §"Methodology: simulator approximations" rewritten:
  divergence count drops from 8 to 1, with a per-row resolution
  pointer. New §"Calibration choices" section between
  §Methodology and §"Calibration vs invariant" documents the
  simulator's defaults for spec-open questions.
- [sim-rs/sim-cli/src/metrics/collector.rs](../../sim-rs/sim-cli/src/metrics/collector.rs)
  — added `MetricsCollector::set_representative_node(name)`
  setter; updated the `is_representative` doc-comment.
  Renamed the existing
  `representative_node_is_first_arrived` test to
  `representative_node_lazy_fallback_picks_first_arrived` and
  added two new tests:
  `representative_node_pinning_overrides_first_arrival`
  (verifies pre-set wins over first-tick) and
  `out_of_order_events_do_not_roll_slot_backwards`.
- [sim-rs/sim-cli/src/runner.rs](../../sim-rs/sim-cli/src/runner.rs)
  — `async fn run_job` is now `pub` so the integration test
  can call it. After `MetricsCollector::new(...)`, the runner
  calls `set_representative_node` with the lexicographically
  smallest node name from `config.nodes` so the time-series
  source is deterministic across runs (independent of tokio
  scheduling). The two `verify_suite` tests'
  manifest/run-summary fixtures now use `serde_json::json!` +
  a shared `lay_down_verify_suite_fixture` helper instead of
  raw `format!()` JSON strings, for resilience to schema
  renames.

## Decisions M5 made (beyond carry-forwards)

| Decision | Where | Why |
|---|---|---|
| **Suite goldens are committed under `parameters/phase-2-sweep/suites/.goldens/`, format `<job> <seed> <hash>`** | [.goldens/](../../sim-rs/parameters/phase-2-sweep/suites/.goldens/) | The format leaves room for a future M to pin more (job, seed) pairs per suite (one line each) without changing the parser. The directory sits alongside the suite YAMLs so they're easy to find from the suite definitions. |
| **The integration test rebases relative paths onto `sim-rs/`** rather than `cd`-ing into it | [sim-cli/tests/determinism.rs `rebase_suite_paths`](../../sim-rs/sim-cli/tests/determinism.rs) | `std::env::set_current_dir` is process-global and racy with parallel tests. Rebasing per-Suite at load time is local and safe. |
| **Determinism integration tests are `#[ignore]`'d by default**; explicit run via `cargo test --release -- --ignored determinism` | [sim-cli/tests/determinism.rs](../../sim-rs/sim-cli/tests/determinism.rs) | Each baseline run is ~200ms in release; the 7 of them total ~0.3s wall-time at cargo's default parallelism (~1.4s with `--test-threads=1`). Acceptable on demand but adds a noticeable tax to every-edit `cargo test`. The `#[ignore]` keeps the inner-loop fast; CI / pre-commit runs pick it up explicitly. |
| **Representative-node pinning is pre-set in the runner, not derived in the collector** | [runner.rs `run_job`](../../sim-rs/sim-cli/src/runner.rs), [collector.rs `set_representative_node`](../../sim-rs/sim-cli/src/metrics/collector.rs) | The runner already iterates `config.nodes` for the multiplier-floor wiring; pre-setting the representative there is one extra `.iter().min()` call. The lazy "first-tick wins" fallback is preserved for unit tests/standalone callers that don't pre-set; production runs always pin deterministically. |
| **JSON kebab-case audit: documented, not standardised** | [CLAUDE.md §Conventions](../../CLAUDE.md) | `Manifest`/`JobEntry` use `#[serde(rename_all = "kebab-case")]`; `RunSummary` uses Rust snake_case (no rename_all). Standardising would invalidate every persisted manifest under `sim-rs/output/`, forcing re-runs of all 72 (job, seed) pairs. The inconsistency is documented as a convention to follow for future schema additions; not worth the churn for M5. |
| **RB-reduced overlay duplication: documented, not refactored** | [CLAUDE.md §Conventions](../../CLAUDE.md) | The three `protocol-rb-reduced-{half,third,quarter}.yaml` files duplicate `protocol-base.yaml` and override only `rb-body-max-size-bytes`. Implementing stacked `JobOverrides::protocol_overlay: Vec<PathBuf>` semantics would clean this up but is a runner-feature change not in M5's scope. CLAUDE.md flags that future protocol-base.yaml additions must be propagated to all three. |

## Methodology-table walk

The plan's claim ([implementation-plan.md line 23](implementation-plan.md#L23))
is that the rebuild reduces the methodology table's hard divergences
from 8 to 1. M5 verified this row-by-row:

- **7 rows resolved** (M1-M4): EIP-1559 maxFee semantics
  (M1), EB binary fullness trigger (M2 → M3 production path),
  per-tx refund (M1+M2), RB priority-only validity rule (M2),
  priority partition cap = one RB-worth (M2),
  logical/tag-based partition replaced by `posted_lane: Lane`
  (M1+M2), capacity-weighted window (M1).
- **1 row remains**: anti-standard cap under FIFO fallback. The
  spec mandates this cap when `LaneSelectionOrder::Fifo` is
  active; the simulator has no implementation and no FIFO suite
  is authored. This is the single residual divergence and would
  only need to be implemented before a FIFO experiment is run.

The table at [mechanism-design.md §Methodology](mechanism-design.md)
records the resolution per-row and points each resolved entry at
the milestone whose handoff describes the implementation.

## Known limitations carried forward

### 1. Pricing state has no rollback on fork/slot-battle (carried from M1)

Same caveat as M1, M2, M3, M4 handoffs §1. Single-producer suites
don't exercise it. M5 doesn't move it. The single-producer
topology and `rb-generation-probability: 1.0` together mean no
slot-battle scenarios fire on the current 7 suites. A future
multi-producer suite would need snapshot-and-replay before its
event stream could be hashed against goldens.

### 2. EB partition activation is a producer claim, not derivable from the EB body (carried from M3)

Same caveat. Honest-producer simulation only — a future attacker
model would need to test inconsistency between the
`partition_activated` bit and the EB's content.

### 3. `PricingTick` is per-node; metrics use a single representative (M5 partially addressed)

M5 made the representative *deterministic* (lexicographically
smallest node name, pre-set by the runner) rather than
non-deterministic (first tick wins). The other-nodes-are-dropped
property still holds — this still under-reports cross-node
disagreement in any future multi-producer suite.

### 4. Cross-arch CI verification is intra-arch + golden-hash only (carried from M2/M3)

See *§Phase-2 closure addendum* item 1.

### 5. Per-job protocol overrides REPLACE rather than stack (M4-introduced)

See *§Phase-2 closure addendum* item — implicit in the
RB-reduced overlay convention. M5 documented in
[CLAUDE.md §Conventions](../../CLAUDE.md). No code change.

### 6. `urgency-inversion` produces a refund-budget signal, not an eviction-count signal (M4-introduced)

See [m4-handoff.md §6](m4-handoff.md). Same caveat carries
forward; M5 doesn't move it.

### 7. `rb-scarcity` has a steep cliff between half and third RB capacities (M4-introduced)

See [m4-handoff.md §7](m4-handoff.md). Same caveat carries
forward.

## Architectural changes from M4 (flagged for future readers)

These are the structural shifts M5 made. None affect any
simulation-affecting state.

1. **`runner::run_job` is now `pub`.** Integration tests can
   call it directly. The signature and behaviour are unchanged
   from M4.
2. **`MetricsCollector::set_representative_node(name)` is a new
   pub method.** The runner calls it once per `run_job` to pin
   the time-series representative; the lazy "first-tick wins"
   fallback remains for tests/standalone callers.
3. **The `verify_suite` test fixtures now go through
   `serde_json::json!`** rather than raw `format!()` JSON
   strings, with a shared `lay_down_verify_suite_fixture`
   helper. Future schema renames in `Manifest`/`JobEntry`/
   `RunSummary` are easier to keep in sync with the test
   fixtures.

## Gotchas

1. **Run determinism tests in `--release`.** They time out (or
   are very slow) in test profile because each baseline run is
   a 200-slot single-producer simulation. The
   `cargo test --release -- --ignored determinism` invocation
   is documented in CLAUDE.md and the test file's
   doc-comment.
2. **Goldens regeneration is non-trivial.** Running
   `UPDATE_GOLDENS=1` flips every committed golden in one shot.
   A future change that intentionally flips one golden should
   surface that intent in the commit message — and ideally
   bump the goldens tag (`m5-goldens-v2`, etc.) so the prior
   state is recoverable.
3. **The integration test redirects `output_dir` to a
   tempdir.** Any future test addition must do the same or
   the working tree will pollute on every run; per-test
   `tempfile::TempDir` ensures cleanup.
4. **The integration test is the slowest part of `cargo test
   --release -- --ignored`.** If a new `#[ignore]`d test joins,
   either keep it in the same file (it's `--ignored` runs as a
   group) or pick a unique name prefix so test filters work
   cleanly.
5. **The `MetricsCollector::set_representative_node` setter
   does not validate that the name corresponds to a real
   topology node.** The runner pulls from `config.nodes` so
   the value is always valid; tests that pre-set arbitrary
   strings just lose `PricingTick` events from non-matching
   nodes.

## Test infra additions

- `sim-rs/sim-cli/tests/determinism.rs` is the canonical
  integration test for suite-level golden regeneration.
  `lay_down_verify_suite_fixture` in
  `sim-rs/sim-cli/src/runner.rs::tests` is the canonical helper
  for `verify_suite` malformed-state tests.

## Hard rules — restated

These rules held throughout M5 and remain in force on
`dynamic-experiment`:

1. **No code, configs, types, or schemas from `pricing-sim-base`.**
   Observe it as prior art only.
2. **No f64 in simulation-affecting state.** M5 added no
   simulation-affecting code; the new code is metrics-side
   (`set_representative_node`), test infrastructure, and
   visibility annotations.
3. **Suite reframing risk** ([implementation-plan.md line 327](implementation-plan.md#L327)):
   moot for M5 (no new suites). Remains in force for any
   future phase-2 work that adds suites.
