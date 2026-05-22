# CLAUDE.md — phase-2 dynamic-pricing simulator

This branch (`dynamic-experiment`) is a clean-room rebuild on top of
the upstream Leios protocol simulator (`main`), implementing the
phase-2 dynamic-pricing mechanisms specified in
[docs/phase-2/mechanism-design.md](docs/phase-2/mechanism-design.md).
The implementation plan and per-milestone deltas live under
[docs/phase-2/](docs/phase-2/) — start with
[implementation-plan.md](docs/phase-2/implementation-plan.md) and the
m1→m5 handoffs.

**Latest mechanism decision:** Family B (EIP-1559-faithful chain-derived,
one controller step per canonical block). See
[`.planning/family-b-decision-2026-05-14.md`](.planning/family-b-decision-2026-05-14.md)
for the authoritative decision memo and audit trail.

The build is `cd sim-rs && cargo build --release`; the test suite is
`cd sim-rs && cargo test --workspace`. The phase-2 suite runner is the
`experiment-suite` binary in `sim-cli`.

## Repository layout

```
sim-rs/
├── sim-core/                          # protocol + pricing kernel
│   └── src/
│       ├── lib.rs
│       ├── model.rs                   # Transaction, EB, RB, ledger types + PerLaneQuote, WindowAggregate (chain-derived)
│       ├── config.rs                  # all Raw* deserialisation + SimConfiguration
│       ├── events.rs                  # Event enum + EventTracker
│       ├── tx_pricing/                # the phase-2 pricing kernel (chain-derived; spike 007)
│       │   ├── mod.rs                 # PricingBackend trait + ChainView, Lane, samples, lane rules
│       │   ├── window.rs              # aggregate_from_chain + update_aggregate (pure)
│       │   ├── single_lane.rs         # BaselinePricing + Eip1559Pricing (stateless policies)
│       │   └── two_lane.rs            # TwoLanePricing + 4 TwoLaneVariant arms (stateless policy)
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
    ├── topology-realistic-100.yaml    # 100-node, mass-stratified mainnet curve — phase-2 suites' default since 2026-05-13
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
  pure-function policy under the chain-derived pattern (spike 007).
  Exposes `compute_derived_quote(parent_quote, parent_aggregate,
  parent_samples, evicted_samples) -> (PerLaneQuote, WindowAggregate)`,
  `lane_validity_rule`, `lane_selection_order`,
  `min_priority_premium_multiplier`, `samples_for_block`,
  `effective_window_length`, `cold_start_quote`. The backend holds no
  mutable controller state — `derived_quote` is computed per block at
  production and stored on the `LinearRankingBlock` as a header field.
  This matches EIP-1559's stateless pattern: orphan blocks from slot
  battles carry their own `derived_quote` which is discarded with the
  block, so controller contamination from short forks is impossible
  by construction (closes WR-1, per spike 007). **Selection lives in
  the simulator block builder**; the backend never sees simulator
  types — the only seam is `ChainView` (read-only chain walk exposed
  to the backend via `&dyn ChainView` parameter).
- **`derived_quote` on `LinearRankingBlock`**: every RB carries a
  `PerLaneQuote { standard: u64, priority: u64 }` plus a
  `WindowAggregate` (the controller window's incremental state).
  These are pure functions of the parent RB plus samples in canonical
  predecessors. EBs do not carry `derived_quote` — they inherit it
  from their parent RB via chain lookup. The simulator's local block
  cache (`block_samples: BTreeMap<BlockId, Vec<PricedBlockSample>>`)
  is pruned at `2 × window_length` behind the chain tip to bound
  memory; under Cardano's k=2160 finality, this is trivially well
  within the chain-stability horizon.
- **`BaselinePricing`** — flat `c = 1`, returns `min_fee_a` for every
  `compute_derived_quote` call. **`Eip1559Pricing`** — stateless
  policy carrier; `compute_derived_quote` runs the integer-rational
  EIP-1559 step (clamp formula and era floor) against the chain-
  derived `WindowAggregate`. **`TwoLanePricing`** — same pattern with
  two controllers driven from a shared `WindowAggregate` (per-lane
  bytes/capacity split) + multiplier-floor invariant enforced on the
  output of `compute_derived_quote`. Four `TwoLaneVariant` arms cover
  the spec's RB-reserved / un-reserved × priority-only-static /
  both-dynamic matrix. None of the three backends hold any mutable
  controller state — under chain-derivation the canonical chain itself
  carries the controller state (`derived_quote` + `window_aggregate`
  per RB).
- **`MempoolGate`**
  ([sim-core/src/sim/mempool_gate.rs](sim-rs/sim-core/src/sim/mempool_gate.rs)):
  the sole byte-cap authority. Owns admission
  (`minFeeB + quote × bytes ≤ max_fee_lovelace` AND not over byte
  cap), revalidation on quote change (evict tx whose lane's quote
  has risen above its `max_fee_lovelace`), and inclusion charging
  (`actual_fee = minFeeB + quote(served_lane) × bytes`,
  `refund = max_fee − actual_fee`). Reject-only on full mempool —
  no eviction of valid txs to make room.
- **`WindowAggregate`** (chain-derived): the rolling
  `Σ relevantBytes / Σ relevantCapacity` is carried on every RB as a
  `WindowAggregate { standard_sum_bytes, standard_sum_capacity,
  priority_sum_bytes, priority_sum_capacity, blocks_in_window }`
  (u128 sums). Each block's aggregate is the parent's aggregate +
  parent's samples − any samples falling off the tail. The window
  length is parameterised per controller. Capacity-varying signals
  (single-lane EIP-1559, both-dynamic standard, un-reserved priority)
  default to length 32. RB-reserved priority is forced to length 1
  (mathematically reduces to per-block fill rate, which is what the
  spec prescribes since RB-reserved priority capacity is uniform per
  block). The `tx_pricing/window.rs` module exposes
  `aggregate_from_chain` (cold-start aggregator) and
  `update_aggregate` (incremental step) as pure functions.
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

All chain-derived computation is integer/u128 throughout:
`compute_derived_quote` is a pure function returning `PerLaneQuote`
and `WindowAggregate`, both of which are `u64`/`u128` only. Block
fields `derived_quote` and `window_aggregate` are bit-stable across
architectures.

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
handoff for the CIP / external write-up. Chain-derivation is reorg-
safe by construction: deep reorgs replace the canonical chain
entirely, and every block on the new chain carries its own
`derived_quote` (computed as a pure function of its own ancestors),
so no rollback step is needed and no contamination from orphan
blocks is possible.

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
- **Derived-quote cadence: per priced block**, materialised on the
  canonical chain as `LinearRankingBlock.derived_quote`. Each block's
  `derived_quote` is a pure function of `parent.derived_quote`,
  `parent.window_aggregate`, and the samples carried by the parent
  (and any endorsed EB). Answers
  [mechanism-design.md §"Open calibration choices"](docs/phase-2/mechanism-design.md).
  *Re-calibrating*: per-RB or per-epoch cadence requires a rewrite of
  the chain-derived production path in `linear_leios.rs`; intrusive.
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
- **`rb-generation-probability = 0.05` and `default-slots = 2000`.**
  Cardano-realistic RB cadence (~20 slots between RBs) clears the
  linear-Leios 13-slot endorsement window so EBs land on chain.
  An earlier revision pinned `rb-prob = 1.0` for "uniform
  tx-bearing-block-per-slot time series" — see
  [docs/phase-2/calibration-fix-postmortem.md](docs/phase-2/calibration-fix-postmortem.md)
  for why that was a calibration bug and what changed. Phase-2 suites
  use `topology-realistic-100.yaml` (100 nodes, mainnet-snapshot
  mass-stratified stakes, rescaled to total = 3 × 10^10 lovelace).
  The minimum stake in that curve clears the lottery-quantization
  check (min × rb-prob ≥ 100) by three orders of magnitude.
  *Re-calibrating*: raise rb-prob, but keep `expected_RB_gap >
  header_diffusion × 3 + linear_vote_stage_length +
  linear_diffuse_stage_length` (currently 13 slots) or
  endorsement breaks again.
- **Topology = `parameters/phase-2-sweep/topology-realistic-100.yaml`.**
  100 nodes; same locations/latencies/producers/bandwidth as upstream
  `parameters/topology.default.yaml`; stake values are a mass-stratified
  downsample of the 1,510 Cardano mainnet pools with ≥ 1k ADA active
  stake (Cardano mainnet on-chain state, epoch 582, retrieved 2026-05-14), rescaled linearly to
  total = 3 × 10^10 lovelace. Top-1 stake share = 1.97 %; Nakamoto
  coefficient = 35; Gini = 0.253. See
  [`.planning/spikes/006-curve-design/README.md`](.planning/spikes/006-curve-design/README.md)
  for the curve-design rationale and `topology-realistic-100.yaml`'s
  header comment for the reproduction recipe.
  *Re-calibrating*: re-run the on-chain query at a later epoch and
  regenerate via `sim-rs/scripts/generate-realistic-100-topology.py`;
  the M5 suite goldens flip and require `UPDATE_GOLDENS=1` re-pinning.
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

## Mechanism choice and audit trail (2026-05-14)

Phase-2's controller is **chain-derived (Family B)**: every
`LinearRankingBlock` carries its own `derived_quote` as a pure
function of the parent block's chain-derived state plus the samples
emitted by canonical predecessors within the smoothing window. The
controller advances exactly once per canonical block — the
EIP-1559-faithful cadence. The pre-2026-05-14 node-local accumulator
implementation effectively stepped twice per RB-EB pair (one step
at `apply_priced_block` on RB publish, a second step at
`apply_eb_priced_block` on deferred EB validation); this was an
unintentional implementation artifact diverging from
[`mechanism-design.md`](docs/phase-2/mechanism-design.md)'s
per-block-cadence intent, and was corrected by the chain-derived
refactor (spike 007 ADOPT).

**Family B was committed for publication 2026-05-14**; see
[`.planning/family-b-decision-2026-05-14.md`](.planning/family-b-decision-2026-05-14.md)
for the authoritative decision memo (rationale, ready-to-paste
publication framing, follow-on work).
Empirical welfare-impact characterisation (accumulator vs
chain-derived across 33 sundaeswap-smoke jobs) lives at
[`.planning/mechanism-welfare-impact-2026-05-14.md`](.planning/mechanism-welfare-impact-2026-05-14.md):
the un-reserved arms are mechanism-robust (median |Δ%| 15%, no
sign-flips); RB-reserved and partitioned arms gain ~30% median
welfare under Family B with isolated `x4_rb_quarter` corner-stress
flips; single-lane EIP-1559 collapses by orders of magnitude (a more
honest characterisation of single-lane's narrower welfare regime).

**WR-1** (pricing-state contamination on slot-battle reorg) is
**RESOLVED 2026-05-14** by the chain-derived design: there is no
node-local mutable controller state, so orphan-block samples cannot
contaminate the canonical controller trajectory. See the Fix Status
table in [`.planning/REVIEW.md`](.planning/REVIEW.md) for the full
disposition of every review finding.

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

Note: the table below covers the 7 M3/M4 mechanism-characterisation
suites pinned by M5 suite-level goldens. The full suite directory holds
19 YAMLs (the 7 listed here plus 12 demand-regime suites under
`paper_like_*` and `sundaeswap_*` profiles). The 12 demand-regime
suites are not goldens-pinned.

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

**Parallelism.** `experiment-suite run` and `experiment-suite verify`
run (job, seed) pairs concurrently by default. The cap is
`min(available_parallelism(), 8)`; override with `--parallelism N`
(`-P N`). Each parallel job owns its own simulator state (config,
topology, mempool, metrics collector) and runs inside its own OS
thread + per-thread `current_thread` tokio runtime — required because
`Simulation` contains `Box<dyn Actor>` which isn't `Send`. Peak RSS
scales linearly in N; with a 100-node topology, the default cap of 8
stays comfortably under 32 GB on the dev machine. Raise via
`--parallelism` if you know your machine has more headroom; lower it
if you run on memory-constrained hardware or stack with another
parallel driver (e.g. `scripts/run-parallel-suites.sh` parallelises
*across* suites — total tokio worker threads ≈ cross-suite K ×
intra-suite P).

### Visualising suite results

For browsing `sim-rs/output/` interactively in a browser, the phase-2
visualisation site lives at
[`sim-rs/scripts/viz/`](sim-rs/scripts/viz/README.md). One command builds
and serves it locally:

```sh
python sim-rs/scripts/viz/build.py --serve
```

The site renders the suite list, per-suite drill-down, per-(job, seed)
detail (headline strip + per-component latency + three Observable Plot
panes), and an in-suite cross-seed time-series overlay against the
artefacts already on disk under `sim-rs/output/`. The bundle is written
to `sim-rs/output/viz/`, which is gitignored transitively via the
existing `sim-rs/.gitignore` `/output` rule (no new gitignore entry was
added for this surface). The server binds `127.0.0.1` exclusively. See
[`sim-rs/scripts/viz/README.md`](sim-rs/scripts/viz/README.md) for the
flag reference, the three-tier output layout, and the annual recipe for
refreshing the vendored Observable Plot + D3 bundles.

## Conventions / gotchas

- **Abbreviations: expand on first use.** Every acronym or abbreviation
  in `.planning/`, `docs/phase-2/`, `CLAUDE.md`, or any documentation
  written for this project must be spelled out in full the first time
  it appears in a given document, with the abbreviation in parentheses
  immediately after — e.g. "Paired Seed Evaluation (PSE)",
  "Bias-corrected and accelerated (BCa) bootstrap", "Inter-Quartile
  Range (IQR)". Subsequent uses can use the abbreviation alone. This
  applies regardless of how common the abbreviation is in the
  surrounding literature.
- **Citations notice.** The publication-track artefacts in
  `cip-evidence/` and `docs/phase-2/` cite only sources Will has
  actually read or factual data sources (deployed-spec parameter
  tables, on-chain queries, raw source files, software dependencies).
  The only intellectual-influence citation kept is Kiayias et al.
  *"Tiered Mechanisms for Blockchain Transaction Fees"* (arXiv:2304.06014),
  which is the source of the `retained_value` formula and the
  namesake of this repo. **Do not introduce academic citations to
  motivate design choices, anchor calibration values, or justify
  methodology decisions unless Will explicitly confirms he has read
  the source.** Past phases ran literature searches that retroactively
  cited papers whose findings resembled rationale already written;
  those have been stripped from the publication track. The `.planning/`
  directory retains the historical record for audit purposes; do not
  reintroduce those citations into publication-track docs. See
  `~/.claude/projects/-home-will-git-arc-tiered-pricing/memory/feedback_no_retroactive_citations.md`
  for the operational test ("can Will defend the claim in a meeting
  without having read the cited work?").
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
- **In-suite parallelism preserves per-(job, seed) determinism.** The
  suite-level golden hashes and the `verify` subcommand both treat
  each (job, seed) as the determinism unit. Parallelism changes only
  the wall-clock interleaving of jobs, not their seeds, inputs, or
  event streams. The manifest's `BTreeMap`-keyed-by-(job_name,
  seed_string) layout (runner.rs:71) gives deterministic on-disk
  order regardless of completion order; per-(job, seed) artefact
  paths `<output_dir>/<job_name>/<seed>/` are unique so no two
  parallel jobs ever touch the same file. The only cross-job shared
  state is `manifest.json`, guarded by a single mutex; lock
  hold-times are tiny (one `fs::write` + the
  `metrics_comparison.txt` rebuild).
- **Multi-producer topology, per-node mempool.** Every node has its
  own mempool — admission/eviction/inclusion run per-node, gossip
  distributes txs across the network. The suite default
  `topology-realistic-100.yaml` has 100 producers and 100 mempools;
  in any earlier `topology-single-producer.yaml`-based test, the
  producer/source/mempool counts happen to coincide at N=1, but
  this is the special case, not the default. In any multi-node
  topology (the operational `topology-realistic-100.yaml`,
  `topology.default.yaml`) there are
  N mempools regardless of how many nodes carry
  `tx-generation-weight`. Don't infer "one source ⇒ one mempool" —
  gossip propagation, slot-battle dynamics, and per-node
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
