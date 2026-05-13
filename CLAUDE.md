# CLAUDE.md — phase-2 dynamic-pricing simulator

This branch (`dynamic-experiment`) is a clean-room rebuild on top of
the upstream Leios protocol simulator (`main`), implementing the
phase-2 dynamic-pricing mechanisms specified in
[docs/phase-2/mechanism-design.md](docs/phase-2/mechanism-design.md).
The implementation plan and per-milestone deltas live under
[docs/phase-2/](docs/phase-2/) — start with
[implementation-plan.md](docs/phase-2/implementation-plan.md) and the
m1→m5 handoffs.

The build is `cd sim-rs && cargo build --release`; the test suite is
`cd sim-rs && cargo test --workspace`. The phase-2 suite runner is the
`experiment-suite` binary in `sim-cli`.

## Repository layout

```
sim-rs/
├── sim-core/                          # protocol + pricing kernel
│   └── src/
│       ├── lib.rs
│       ├── model.rs                   # Transaction, EB, RB, ledger types
│       ├── config.rs                  # all Raw* deserialisation + SimConfiguration
│       ├── events.rs                  # Event enum + EventTracker
│       ├── tx_pricing/                # the phase-2 pricing kernel
│       │   ├── mod.rs                 # PricingBackend trait, Lane, samples, lane rules
│       │   ├── window.rs              # CapacityWeightedWindow (u128 rationals)
│       │   ├── single_lane.rs         # BaselinePricing + Eip1559Pricing
│       │   └── two_lane.rs            # TwoLanePricing + 4 TwoLaneVariant arms
│       ├── tx_actors.rs               # ActorComponent, MaxFeePolicy, lane_choice, welfare
│       └── sim/
│           ├── linear_leios.rs        # the only simulated protocol relevant to phase-2
│           ├── mempool_gate.rs        # admission/revalidation/inclusion charging
│           ├── …                      # other (legacy) protocols, lottery, drivers
│           └── tests/                 # M1-M3 deterministic scenario tests
├── sim-cli/                           # phase-2 driver + metrics
│   ├── src/
│   │   ├── lib.rs
│   │   ├── runner.rs                  # Manifest, run_suite, run_job, verify_suite
│   │   ├── suite.rs                   # Suite YAML schema
│   │   ├── metrics/                   # MetricsCollector + writers
│   │   ├── events/                    # legacy event sink
│   │   └── bin/experiment-suite/      # `experiment-suite run|status|verify`
│   └── tests/
│       └── determinism.rs             # M5 suite-level golden-hash regime
└── parameters/phase-2-sweep/          # all phase-2 configs
    ├── protocol-base.yaml             # phase-2 protocol baseline
    ├── protocol-rb-reduced-{half,third,quarter}.yaml  # M4 RB-scarcity overlays
    ├── topology-single-producer.yaml  # one-node topology (every slot wins the RB lottery)
    ├── demand/                        # actor profiles (paper_like_*.yaml)
    ├── pricing/                       # 13 controller-tuning YAMLs
    └── suites/                        # 7 phase-2 suite YAMLs (+ READMEs, .goldens/)

docs/phase-2/
├── mechanism-design.md                # the spec
├── implementation-plan.md             # the rebuild plan
└── m{1,2,3,4,5}-handoff.md            # per-milestone delta notes
```

## Mechanism abstractions

- **`Lane`**: two-variant enum (`Standard`, `Priority`) carried on
  every `Transaction` (`posted_lane`) and on every inclusion event
  (`served_lane`). Single-lane mechanisms collapse both to
  `Standard`. There is **no tier vocabulary** anywhere on the
  branch.
- **`PricingBackend`** trait
  ([sim-core/src/tx_pricing/mod.rs](sim-rs/sim-core/src/tx_pricing/mod.rs)):
  policy-only. Exposes `current_quote(lane)`,
  `update_after_block(samples)`, `lane_validity_rule(block_kind)`,
  `lane_selection_order()`,
  `min_priority_premium_multiplier()`, `samples_for_block(...)`,
  `snapshot()`. **Selection lives in the simulator block builder**;
  the backend never sees simulator types.
- **`BaselinePricing`** — flat `c = 1`. **`Eip1559Pricing`** —
  single-controller dynamic; integer-rational EIP-1559 update with
  the spec's clamp formula and era floor, fed by a
  `CapacityWeightedWindow`. **`TwoLanePricing`** — wraps two
  filtered `Eip1559Pricing` controllers + multiplier-floor invariant
  enforced post-update on `quote_per_byte`. Four
  `TwoLaneVariant` arms cover the spec's RB-reserved /
  un-reserved × priority-only-static / both-dynamic matrix.
- **`MempoolGate`**
  ([sim-core/src/sim/mempool_gate.rs](sim-rs/sim-core/src/sim/mempool_gate.rs)):
  the sole byte-cap authority. Owns admission
  (`minFeeB + quote × bytes ≤ max_fee_lovelace` AND not over byte
  cap), revalidation on quote change (evict tx whose lane's quote
  has risen above its `max_fee_lovelace`), and inclusion charging
  (`actual_fee = minFeeB + quote(served_lane) × bytes`,
  `refund = max_fee − actual_fee`). Reject-only on full mempool —
  no eviction of valid txs to make room.
- **`CapacityWeightedWindow`**: rolling
  `Σ relevantBytes / Σ relevantCapacity` over a u128 ring. Length
  parameterised per controller. Capacity-varying signals
  (single-lane EIP-1559, both-dynamic standard, un-reserved priority)
  default to length 32. RB-reserved priority is forced to length 1
  (mathematically reduces to per-block fill rate, which is what the
  spec prescribes since RB-reserved priority capacity is uniform per
  block).
- **EB binary fullness trigger**
  ([sim-core/src/sim/linear_leios.rs `select_eb_with_partition`](sim-rs/sim-core/src/sim/linear_leios.rs)):
  the priority partition activates iff (a) the EB is saturated, OR
  (b) the mempool has ≥1 valid unselected tx but none fits the EB's
  residual bytes. If neither holds, the EB is below capacity and
  posted-priority txs are refunded down to standard fee.
  `partition_activated` is stored on `LinearEndorserBlock` (M3
  decision) so endorsers/producers agree by construction.
- **RB priority-only validity rule**: applies only to RB-reserved
  variants. A standard-fee tx in an RB makes the block invalid.
  Enforced inside `sample_from_mempool_lane_aware` via
  `LaneValidityRule::PriorityOnly`. Un-reserved and single-lane RBs
  carry no validity rule.
- **EB-validation-at-endorsement**
  ([sim-core/src/sim/linear_leios.rs `eb_endorsement_valid`](sim-rs/sim-core/src/sim/linear_leios.rs)):
  the producer walks the candidate EB's txs, checks
  `posted_fee ≤ max_fee_lovelace` at the producer's current
  posted-lane quote, and **refuses to endorse** if any tx is stale
  (M2 design choice: refusing is cleaner than mutating
  already-gossiped EB bodies). Drops the entire endorsement; the RB
  ships unendorsed.
- **Multiplier-floor invariant**
  (`c_priority ≥ multiplier_floor × c_standard`) lives **inside**
  the controller update path and is enforced on `quote_per_byte`
  with `u128` intermediates, never on `c` directly. Constructor-time
  enforcement also raises priority's initial quote up to the floor
  if needed.
- **Actor model**
  ([sim-core/src/tx_actors.rs](sim-rs/sim-core/src/tx_actors.rs)):
  weighted multi-component value-urgency sampling.
  `MaxFeePolicy::ScaledOverLaneQuote { numerator, denominator }`
  produces `max_fee_lovelace` deterministically. `LanePolicy::
  UtilityMaximising { submit_when_underwater }` picks `posted_lane`
  by maximising `expected_utility(lane)`; the math goes through
  `libm::pow` + `libm::round` into `i128` lovelace before any
  comparison so lane choice is bit-deterministic across runs on the
  same arch. `LatencyEstimator` keeps a per-(component, lane)
  rolling EMA of observed inclusion latencies and seeds it from a
  config default at startup.

## Numeric representation contract — integer/rational vs reporting-f64

**Simulation-affecting state is integer/rational/u128 or
bit-reproducible cross-platform math; never plain `f64`.** This
includes admission, eviction, fee charging, controller coefficient,
mempool tracking, `max_fee_lovelace`, the multiplier-floor invariant,
and actor lane choice. `quote_per_byte` is stored as `u64` directly
(not derived from an f64 coefficient at query time). The EIP-1559
update rule runs in `u128` rationals
(`aggregateUtil = Σ numerator_bytes / Σ denominator_bytes`,
`target = (num, den)`, `D` integer, clamped step on
`quote_per_byte`).

**Reporting outputs are plain `f64`.** `retained_value`,
`net_utility`, `retained_value_ratio` and friends in the metrics
collector are computed and stored as `f64` — they're derived from the
deterministic event stream but **never feed back into simulation
decisions**. The pricing event-stream golden hashes (M2/M3 unit-test
constants, M5 suite-level goldens) are over `TXIncluded` and
`TXEvictedQuoteDrift` only — exactly the events that determine
simulator outcomes — so any accidental f64 entry into a hot path
flips them.

## Determinism scope

Determinism is asserted **intra-architecture** with pinned golden
hashes. Three layers, growing in scope:

1. **Unit-test goldens** in
   [sim-rs/sim-core/src/sim/tests/m2_two_lane.rs](sim-rs/sim-core/src/sim/tests/m2_two_lane.rs)
   and
   [m3_actors.rs](sim-rs/sim-core/src/sim/tests/m3_actors.rs):
   tightly-scoped scenarios with constants pinned in source.
2. **`experiment-suite verify <suite.yaml>`**: re-runs every
   `Completed` (job, seed) and asserts the freshly-computed
   `pricing_event_stream.sha256` equals the persisted on-disk
   value.
3. **Suite-level goldens** in
   [sim-rs/parameters/phase-2-sweep/suites/.goldens/](sim-rs/parameters/phase-2-sweep/suites/.goldens/):
   one canonical baseline (job, seed=1) per suite, asserted by
   [sim-rs/sim-cli/tests/determinism.rs](sim-rs/sim-cli/tests/determinism.rs).
   These are slow-by-default (`#[ignore]`'d) — run via
   `cd sim-rs && cargo test --release -- --ignored determinism`.
   To regenerate after intentional simulator changes:
   `cd sim-rs && UPDATE_GOLDENS=1 cargo test --release -- --ignored
   determinism`, then commit and tag.

**Cross-architecture CI verification is not yet built.** The
underlying math (`libm::pow`/`libm::round`, u128 rationals, integer
arithmetic) is bit-stable across architectures given identical inputs,
but the simulator inherits f64 from `main` in non-pricing code paths
(slot lottery, propagation, distribution sampling) which has not been
hardened for cross-arch determinism. A second-arch build pipeline is
infrastructure work outside phase-2's code scope; flagged in the m5
handoff for the CIP / external write-up.

## Calibration choices

The simulator picks concrete defaults for spec-open questions. Each
entry: value, the spec section it answers, a forward-pointer to the
cost of re-calibrating.

- **Window length 32** for capacity-varying signals (single-lane,
  both-dynamic standard, un-reserved priority). Length 1 for
  RB-reserved priority controllers. Answers
  [mechanism-design.md §"Open calibration choices"](docs/phase-2/mechanism-design.md).
  *Re-calibrating*: change the `window-length` field in the relevant
  pricing YAML; suite goldens flip; re-run `UPDATE_GOLDENS=1` and
  re-tag.
- **Update cadence: per priced block**. Every priced block emits
  zero or more `PricedBlockSample`s; the controller steps once per
  block. Answers
  [mechanism-design.md §"Open calibration choices"](docs/phase-2/mechanism-design.md).
  *Re-calibrating*: per-RB or per-epoch cadence requires a rewrite of
  `apply_priced_block`/`apply_eb_priced_block`; intrusive.
- **Un-reserved priority signal source = option 1**
  (`priority_paying_bytes / total_block_capacity`). Answers
  [mechanism-design.md §"Un-reserved priority-only premium"](docs/phase-2/mechanism-design.md)
  (open-question framing + three options near lines 207-211).
  *Re-calibrating*: option 2 (notional priority share) needs a
  config knob; option 3 (delay-gap signal) is a controller-rewrite.
- **Both-dynamic standard signal source**: capacity-weighted
  aggregate of `standard_paying_bytes` against
  `eb_referenced_txs_max_size_bytes` for EBs. **No standard sample
  fires on RB-reserved RBs**, so RB traffic does not move the
  standard quote. Answers
  [mechanism-design.md line 238](docs/phase-2/mechanism-design.md).
  *Re-calibrating*: change the standard sample's `relevant_capacity`
  formula in `samples_for_block`.
- **Default actor `max_fee_policy = ScaledOverLaneQuote { numerator:
  4, denominator: 1 }`** — i.e., `max_fee_lovelace ≈ minFeeB +
  4 × quote × bytes`, giving 4× quote-drift headroom. Answers
  [mechanism-design.md §"Open questions: Intent vs fee"](docs/phase-2/mechanism-design.md)
  partially. *Re-calibrating*: per-component override in a demand
  YAML; M4's `paper_like_mispriced.yaml` already does this for the
  high-urgency component (drops to `{1, 1}`).
- **`multiplier_floor = 4` in `phase-2-rb-scarcity` and
  `phase-2-urgency-inversion`** rather than the spec default 16 (also
  the default in the other 5 suites' `*_x16` jobs). At 16, only
  urgency≥5 components find priority attractive on the utility-
  maximising lane choice and priority demand stays too low to
  surface controller drift; at 4, urgency≥2 picks priority and the
  controller does drift. *Re-calibrating*: raise the floor in the
  suites' pricing YAMLs; signals will weaken because fewer urgency
  components self-select into priority. (An earlier framing of this
  knob attributed it to the M3 single-producer +
  `rb-generation-probability: 1.0` calibration; that calibration
  was a bug, not a choice — see
  [docs/phase-2/calibration-fix-postmortem.md](docs/phase-2/calibration-fix-postmortem.md).
  The `multiplier_floor = 4` choice is independent of that bug
  and survives into the corrected calibration.)
- **`rb-generation-probability = 0.05` and `default-slots = 1000`.**
  Cardano-realistic RB cadence (~20 slots between RBs) clears the
  linear-Leios 13-slot endorsement window so EBs land on chain.
  An earlier revision pinned `rb-prob = 1.0` for "uniform
  tx-bearing-block-per-slot time series" — see
  [docs/phase-2/calibration-fix-postmortem.md](docs/phase-2/calibration-fix-postmortem.md)
  for why that was a calibration bug and what changed. The
  `topology-single-producer.yaml` `stake: 100000` is paired with
  this — at low rb-prob, single-stake values truncate to
  `target_vrf_stake = 0` and the lottery never wins.
  *Re-calibrating*: raise rb-prob, but keep `expected_RB_gap >
  header_diffusion × 3 + linear_vote_stage_length +
  linear_diffuse_stage_length` (currently 13 slots) or
  endorsement breaks again.
- **Default `target_inclusion_blocks` (priority=1, standard=4)**
  seeds the actor's `LatencyEstimator` per (component, lane). The
  observed-latency EMA overwrites this once inclusion events arrive,
  but the seed shapes early-run lane choice. *Re-calibrating*: per-
  component override in the demand YAML.
- **Mempool cap = `2 × eb_referenced_txs_max_size_bytes`**. The
  simulator's interpretation of the spec's "max block body size"
  in linear-Leios. Answers
  [mechanism-design.md line 59](docs/phase-2/mechanism-design.md).
  *Re-calibrating*: set
  `mempool-max-total-size-bytes` in `protocol-base.yaml`.

## Running the suites

All commands assume `pwd = sim-rs/`.

```sh
# Run a suite end-to-end (resumable; skips Completed (job, seed)).
cargo run --release --bin experiment-suite -- run \
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml

# Status of a previously-run (or interrupted) suite.
cargo run --release --bin experiment-suite -- status \
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml

# Determinism verify: re-run every Completed (job, seed) and assert
# the freshly-computed pricing_event_stream.sha256 matches the
# persisted on-disk value.
cargo run --release --bin experiment-suite -- verify \
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml

# Standard test cycle (excludes the slow #[ignore]'d goldens).
cargo test --workspace

# Suite-level determinism goldens (slow; ~1.5s in --release).
cargo test --release -- --ignored determinism

# Regenerate the suite goldens after an intentional simulator change.
UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism
git add parameters/phase-2-sweep/suites/.goldens
git commit -m "M5 goldens regenerated after <reason>"
git tag -a m5-goldens-<n> -m "..."
```

The 7 suites:

| Suite | Question |
|---|---|
| `phase-2-eip1559-robustness.yaml` | Single-lane EIP-1559 across `D` and `target` |
| `phase-2-eip1559-smoothing.yaml` | Single-lane EIP-1559 window-length sweep |
| `phase-2-priority-only-rb-reserved.yaml` | RB-reserved priority-only-static-standard (×4 / ×8 / ×16 multiplier floor) |
| `phase-2-priority-only-unreserved.yaml` | Un-reserved priority-only premium (same multiplier sweep) |
| `phase-2-two-lane-both-dynamic.yaml` | Both-dynamic in partitioned and un-partitioned forms |
| `phase-2-rb-scarcity.yaml` | RB-capacity scarcity restated as a two-lane experiment |
| `phase-2-urgency-inversion.yaml` | Urgency inversion under mis-priced actors |

Suite READMEs live next to each YAML
(`<suite>.README.md`) for the M4 suites; the M3 suites do not have
READMEs because their framing matches the spec directly.

## Conventions / gotchas

- **No `pricing-sim-base` content.** That branch is observable as
  prior art only — no file, type, or function moved across. Hard rule
  from [implementation-plan.md](docs/phase-2/implementation-plan.md).
- **No `f64` in simulation-affecting state.** Hard rule from the
  plan; enforced by the cross-arch determinism golden hashes.
- **Serde rename casing is mixed by historical accident**:
  YAML configs and the runner's `Manifest`/`JobEntry` use kebab-case
  via `#[serde(rename_all = "kebab-case")]`; `RunSummary` uses Rust
  snake_case (no `rename_all`). Both shapes coexist on disk in
  persisted artefacts. Standardising would invalidate every persisted
  manifest under `sim-rs/output/`, forcing re-runs of all 72 (job,
  seed) pairs — not worth the churn for M5. Future schema additions
  should match the surrounding type's existing convention.
- **RB-reduced overlays are full replacements**, not stacked
  overlays. The runner's `JobOverrides` picks
  `overrides.protocol` OR `default_protocol`, never both — so the
  three `protocol-rb-reduced-{half,third,quarter}.yaml` files
  duplicate everything from `protocol-base.yaml` and override only
  the `rb-body-max-size-bytes` knob. **Future additions to
  `protocol-base.yaml` must be propagated to all three RB-reduced
  overlays manually.** Extending `JobOverrides` with stacked
  `protocol_overlay: Vec<PathBuf>` semantics is a deferred
  enhancement.
- **Determinism is intra-arch.** The repo's pinned hashes (M2/M3
  unit-test constants, M5 suite goldens) reproduce bit-identically
  on the same arch (the development machine is x86_64 / glibc).
  Cross-arch CI verification is documented as not-yet-built.
- **"Single-producer" ≠ "single mempool"; "one tx source" ≠ "one
  mempool".** Every node has its own mempool — admission/eviction/
  inclusion run per-node, gossip distributes txs across the network.
  `topology-single-producer.yaml` is the only topology where N=1, so
  the producer/source/mempool counts all happen to coincide; in any
  multi-node topology (e.g. `topology.default.yaml`, `topology-cip-
  realistic.yaml`) there are N mempools regardless of how many nodes
  carry `tx-generation-weight`. Don't infer "one source ⇒ one
  mempool" — gossip propagation, slot-battle dynamics, and per-node
  `LatencyEstimator` state all behave per-mempool even with a
  single explicit source.
- **`Event::TXGenerated` carries `slot: u64`** (M4) so the metrics
  collector reads `submit_slot` from the event field, not from a
  delta-tracking ordering invariant. Don't re-introduce a delta-slot
  read pattern.
- **`urgency: f64` on `Transaction`** is read **only** by the actor
  lane-choice math, which routes it through `libm::pow` + `libm::round`
  into `i128` lovelace before comparison. Never read it from any
  other simulation-affecting code path.
- **The metrics collector's representative node** is pre-set by the
  runner to the lexicographically smallest node name from the
  topology. The lazy "first-tick wins" fallback in
  `is_representative` is for tests/standalone callers that don't
  pre-set; production runs through `runner::run_job` always pin
  deterministically.

## Size sanity check

Informational. Phase-2 code (the rebuild's surface):

| Path | Lines | Plan target |
|---|---|---|
| `sim-core/src/tx_pricing/` | 1,437 | ~3,500 |
| `sim-cli/src/metrics/` | 1,205 | ~3,500 (events.rs equivalent) |
| `sim-cli/src/` (incl. metrics, runner, suite, bins) | 4,558 | n/a |
| `sim-core/src/` (incl. main's existing code) | 16,305 | ~10,000 (rebuild only; main's preexisting protocol code is not a phase-2 line item) |
| Whole simulator (`sim-core` + `sim-cli` + tests) | 21,104 | ~12,000 (rebuild only) |

The `tx_pricing/` and `metrics/` modules came in well under their
respective targets — the rebuild is leaner than projected. The
`sim-core/` total exceeds the target because it includes the
upstream `main` simulator (slot lottery, propagation, voting) which
phase-2 builds on top of without rewriting.
