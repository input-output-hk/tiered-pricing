# Plan: Clean-room spec-aligned simulator on top of `main`

## Context

`main` is the upstream Leios protocol simulator (networking, clock, slot lottery, RB/EB block production, voting, cert flow). It has no transaction-pricing mechanism, no mempool admission policy, and no actor-driven demand model. The phase-2 mechanism design at [docs/phase-2/mechanism-design.md](docs/phase-2/mechanism-design.md) specifies what the pricing layer should be: five live mechanisms (single-lane EIP-1559, RB-reserved priority-only premium, un-reserved priority-only premium, both-dynamic partitioned, both-dynamic un-partitioned) plus the common transaction lifecycle (maxFee, current-quote charging, refunds, finite mempool).

Goal: branch from `main`, implement the spec from scratch as a clean-room build. **The `pricing-sim-base` branch is not a source.** It is observable as prior art and can inform calibration intuitions, but no file, type, or function moves across. Every line on the new branch is written from the spec. Existing welfare evidence on `pricing-sim-base` is invalidated for any mechanism touched by the rewrite — that's the deliberate cost of fidelity.

The three phase-2 experimental questions currently expressed via the tiered backend on `pricing-sim-base` (premium-queue urgency separation, RB-scarcity behaviour, urgency-inversion under mis-pricing) are valuable independent of how they were once implemented. They are restated here as two-lane experimental questions and authored as new configs on the new branch.

Decisions taken:
- **Approach**: clean-room implementation on top of `main`. No code, configs, or schemas brought across from `pricing-sim-base`.
- **Spec gaps closed by design** (no "approximation" footnote needed in the methodology table for these):
  - **maxFee** is a total-lovelace transaction field. Wallet/actor policy may be expressed per-byte, but the on-chain field is total. **Spec amendment**: rewrite [mechanism-design.md:39](docs/phase-2/mechanism-design.md#L39) at M1 to make this explicit. The two views are *operationally equivalent* for a fixed transaction once integer rounding is specified and `max_fee_lovelace ≥ minFeeB` is enforced (`current_quote × bytes + minFeeB > max_fee_lovelace` ⟺ `current_quote > (max_fee_lovelace − minFeeB) / bytes`); but the field commits to total lovelace because it integrates cleanly with actor value-based willingness-to-pay and avoids special-casing the additive `minFeeB`.
  - **Quote drift handling**: tx is admitted if and only if its `max_fee_lovelace` covers the current quote at submission; on every controller update, the mempool is revalidated and any tx whose lane's quote has risen above its `max_fee_lovelace` is evicted; on inclusion, the tx pays the current quote at the served lane (not the admission quote); refund is `max_fee_lovelace − actual_fee_lovelace` and is recorded on the inclusion event.
  - **Unified utilisation-signal abstraction**: capacity-varying signals (single-lane, both-dynamic standard) use a rolling capacity-weighted window; uniform-capacity priority signals (RB-reserved priority controller) use length 1, which mathematically reduces to per-block fill rate as the spec prescribes ([mechanism-design.md:76,170](docs/phase-2/mechanism-design.md#L76)). Same code path, both spec-correct via the length parameter.
  - **Finite mempool cap** defaulted to `2 × eb_referenced_txs_max_size_bytes` (the simulator's interpretation of the spec's "max block body size" in linear-Leios; full rationale in §Finite mempool cap). **Reject-new-when-full only**, no margin-based eviction of valid txs (that would be policy beyond spec).
  - **Explicit `posted_lane` / `served_lane`** on inclusion events, closing lane-mismatch refund accounting in one consistent path. No metrics-layer relabelling.
  - **Binary EB-fullness trigger** for priority-partition activation, distinguishing saturation from exhausted-mempool cases (precise rule below).
- **Spec gaps deferred**: anti-standard cap under FIFO fallback (not implemented unless a FIFO experiment is added). End-to-end on-chain refund event modelling stays out of scope (`actual_fee` plus `refund_lovelace` on the inclusion event is sufficient).
- **Vocabulary**: there is no tier vocabulary on the new branch. `Lane` is a two-variant enum: `Standard` and `Priority`. Single-lane mechanisms produce txs with `posted_lane = Lane::Standard` and always assign `served_lane = Lane::Standard`.

After this rebuild, the **hard simulator/spec divergences** in [mechanism-design.md §Methodology](docs/phase-2/mechanism-design.md#L295-L308) drop from 8 entries to 1 (anti-standard cap under FIFO fallback). The remaining table entries shift in character from "approximations" to "calibration assumptions" — concrete simulator defaults for spec-open questions (window length, update cadence, unreserved priority signal source, both-dynamic standard signal source, actor maxFee policy). These are documented as deviations-by-default with rationale, not silent.

## Architecture (modules to implement from scratch)

### Pricing core: `sim-rs/sim-core/src/tx_pricing/`

Four files, no tier vocabulary, one window abstraction.

**Ownership boundary**: the **simulator block builder** owns transaction packing and partition activation. **Pricing backends** are policy providers — they expose quotes, controller updates, lane-validity rules, ordering policy, and the multiplier-floor invariant. Selection itself lives in the simulator and calls into the backend for these queries. There is no `select_transactions_for_block` method on `PricingBackend`; that responsibility is the simulator's.

- `mod.rs` — `PricingBackend` trait (policy-only, no selection, no simulator types in the signature):
  - `current_quote(lane: Lane) -> u64` — returns `quote_per_byte`, the per-byte rate after applying the spec's clamp/floor and integer rounding (formula below).
  - `update_after_block(samples: &[PricedBlockSample])` — zero or more samples per block; both-dynamic typically emits one standard-controller sample and one priority-controller sample from the same block.
  - `lane_validity_rule(block_kind: BlockKind) -> LaneValidityRule` (`PriorityOnly` for RB-reserved RBs, `None` otherwise)
  - `lane_selection_order() -> LaneSelectionOrder` (`PriorityFirst` | `Fifo`)
  - `min_priority_premium_multiplier() -> Option<Multiplier>` where `Multiplier { numerator: u64, denominator: u64 }` (Some for two-lane, None for single-lane). **Rational, not f64**, for the same cross-platform determinism reason as `ScaledOverLaneQuote` — the multiplier-floor invariant `c_priority ≥ multiplier_floor × c_standard` is enforced inside the controller update path, which is simulation-affecting state.
  - `snapshot() -> PricingSnapshot` (for time-series logging)

  Plus types: `PricingSnapshot`, `BlockKind`, `Lane` (`Standard` | `Priority`), `LaneId`, `PricedBlockSample`, `LaneValidityRule`, `LaneSelectionOrder`.

  Mempool revalidation lives in `sim/mempool_gate.rs`, not on the trait — it calls `backend.current_quote(tx.posted_lane)` and emits eviction events on the simulator side. This keeps simulator types out of the pricing-backend signature.
- `window.rs` — `CapacityWeightedWindow { numerator_bytes, denominator_bytes, ring }` parameterised by length. `aggregate_util() = sum(relevantBytes) / sum(relevantCapacity)`. **One abstraction across all controllers, length set per controller to match spec semantics**: capacity-varying signals (single-lane, both-dynamic standard) default length 32; uniform-capacity priority signals (RB-reserved priority controller) length 1, which reduces mathematically to the per-block fill rate the spec prescribes ([mechanism-design.md:76,170](docs/phase-2/mechanism-design.md#L76)).
- `single_lane.rs` — `BaselinePricing` (flat fee) + `Eip1559Pricing` driven by `CapacityWeightedWindow`. EIP-1559 update rule with the spec's clamp formula and era floor (`c ≥ 1`).
- `two_lane.rs` — `TwoLaneDynamicPricing` with two independent EIP-1559-style controllers, `min_priority_premium_multiplier` invariant enforced post-update, `LaneSelectionOrder` (priority_first / fifo). Window lengths set per partition axis × signal source:
  - **RB-reserved partition** (priority-only-static-standard, partitioned both-dynamic): priority window length 1; standard window length 32 when dynamic.
  - **Un-reserved** (un-reserved priority-only, un-partitioned both-dynamic): no partition; spec leaves the priority signal as an open question with three candidates ([mechanism-design.md:201-207](docs/phase-2/mechanism-design.md#L201-L207)). **Default**: option 1 — `priority_paying_bytes / total_block_capacity` at length 32. Configurable; documented as experimental default.

`TwoLaneDynamicPricing` covers all four two-lane spec mechanisms via configuration; there is no separate `Eip1559PriorityLanePricing` type.

### Priced-block samples

The `PricedBlockSample` type makes update cadence explicit. Every block emits **zero or more** samples — both-dynamic mechanisms typically emit two from the same block (one per controller).

```
struct PricedBlockSample {
    block_kind: BlockKind,        // RankingBlock | EndorserBlock
    controller_lane: Lane,        // which controller this sample feeds
    relevant_bytes: u64,          // numerator
    relevant_capacity: u64,       // denominator
}
```

Cadence rules:
- **Tx-bearing RB**: emits zero or more samples depending on mechanism.
  - Single-lane: one `Standard` sample. `relevant_bytes` = total committed tx bytes; `relevant_capacity` = `max_block_size` (RB body max).
  - **Two-lane RB-reserved** (priority-only-static-standard, partitioned both-dynamic): emits **only** a `Priority` sample. `relevant_bytes = total committed RB bytes` (RB is priority-only by validity rule), `relevant_capacity = max_block_size`. **The standard controller receives no RB sample** even when standard is dynamic — RB capacity is dedicated to priority, so RB traffic must not move standard pricing. The standard controller sees only EB samples.
  - **Two-lane un-reserved** (un-reserved priority-only, un-partitioned both-dynamic): emits one sample per active controller, each with that lane's posted-fee bytes in the RB against `max_block_size`.
- **Endorsement-only RB** (cert-only, no own txs): emits **no** samples on its own. Its certified EB emits samples separately when applied (per [mechanism-design.md:174](docs/phase-2/mechanism-design.md#L174)).
- **Certified EB**: emits one sample per active controller.
  - Single-lane: `Standard` sample. `relevant_bytes` = total committed tx bytes; `relevant_capacity` = `eb_referenced_txs_max_size_bytes`.
  - Two-lane RB-reserved priority controller: `relevant_bytes = min(priority_paying_bytes, max_block_size)`; `relevant_capacity = max_block_size` (one RB-worth, capped per spec lines 168-174).
  - Two-lane un-reserved priority controller: `relevant_bytes = priority_paying_bytes`; `relevant_capacity = total_block_capacity` (per default option 1).
  - Two-lane standard controller (when dynamic, both RB-reserved and un-reserved): `relevant_bytes = standard_paying_bytes_in_eb`; `relevant_capacity = eb_referenced_txs_max_size_bytes`. **Definition**: `standard_paying_bytes_in_eb` = total bytes of txs whose `posted_lane = Standard` that landed in the EB. Priority-fee txs that landed in standard space (refunded due to non-activated partition) count toward the **priority demand signal**, not the standard signal — they were demand for priority, just not served as priority. This matches the spec's demand-driven priority controller ([mechanism-design.md:162-166](docs/phase-2/mechanism-design.md#L162)).

Each sample is appended to its controller's `CapacityWeightedWindow`; the simulator calls `update_after_block(&samples)` once per block with the full slice.

### Spec-first transaction lifecycle

Transaction fields:
- `value_lovelace: u64`
- `urgency: f64`
- `bytes: u64`
- `max_fee_lovelace: u64` — total posted max
- `posted_lane: Lane` — what the user paid into

Mempool gate: new module `sim-rs/sim-core/src/sim/mempool_gate.rs`.
- **Admission**: at submission, compute prospective `posted_fee = minFeeB + current_quote(posted_lane) × bytes`. Reject if `posted_fee > max_fee_lovelace` OR mempool is at byte cap. Reject-only; no eviction of existing txs to make room.
- **Revalidation on quote change**: after every controller update, walk the mempool and evict txs whose lane's quote has risen such that `minFeeB + current_quote(posted_lane) × bytes > max_fee_lovelace`. Counted as `evicted_quote_drift`.
- **Block selection** (lane-aware): RBs are priority-only **only for RB-reserved partitioned variants** (RB-reserved priority-only, partitioned both-dynamic); single-lane and un-reserved variants have no RB lane-validity rule. EB priority partition activates per the rule below. Selection scans by `LaneSelectionOrder`.
- **Fee rounding** (pinned, applied identically in admission, revalidation, and inclusion):
  - `quote_per_byte(lane) = max(minFeeA, ceil(c(lane) × minFeeA))` — the per-byte rate after the controller's coefficient and era floor.
  - `actual_fee_lovelace = minFeeB.checked_add(quote_per_byte.checked_mul(bytes)?)?` — checked arithmetic; admission rejects on overflow.
  - This rounding regime is what makes the maxFee algebra exact: under this regime, "tx is valid at lane `L`" ⟺ `minFeeB + quote_per_byte(L) × bytes ≤ max_fee_lovelace`.
- **Inclusion charging**:
  - `served_lane` = the partition the tx actually landed in.
  - `actual_fee_lovelace = minFeeB + quote_per_byte(served_lane) × bytes` (per the rounding regime above).
  - `refund_lovelace = max_fee_lovelace − actual_fee_lovelace`.
  - Both `posted_lane` and `served_lane` emitted on the inclusion event.

This closes maxFee invalidation, current-quote charging, and lane-mismatch refund accounting in one consistent path.

### EB binary fullness trigger

In the lane-aware block-selection routine for an EB:

1. Assemble candidate EB contents under canonical `priority_first` ordering against the full EB capacity.
2. **Activate the priority partition** iff *either*:
   - selected bytes equal the full EB capacity (saturation), OR
   - at least one valid unselected mempool tx remains and none fits in the EB's residual bytes (capacity-bound rejection).

   If neither holds — i.e., the mempool was exhausted before the EB filled up — the EB is **below capacity**, partition is **not activated**. Activation is binary; no 0.95 threshold.
3. **Assign `served_lane`**:
   - Partition activated: priority-fee txs that fit within one RB-worth of the EB get `served_lane = Priority`; further priority-fee txs get `served_lane = Standard` (refunded).
   - Partition not activated: all priority-fee txs get `served_lane = Standard` (refunded).

**RB validity rule**: applies only to **RB-reserved partitioned variants**. For those: standard-fee txs in an RB make the block invalid; all included txs have `served_lane = Priority`. For **un-reserved variants** and **single-lane**: no RB lane-validity rule. RB selection follows the configured `LaneSelectionOrder`; single-lane txs all have `served_lane = Standard`; un-reserved txs have `served_lane = posted_lane` (no partition to be in or out of).

The activation decision is computed once per EB at selection time, deterministic given the mempool state.

### Finite mempool cap

`leios_mempool_max_total_size_bytes` defaults to `2 × eb_referenced_txs_max_size_bytes` — i.e., twice the EB transaction-referenced max (~24 MB at the linear-Leios CIP-0164 default). The spec at [mechanism-design.md:53](docs/phase-2/mechanism-design.md#L53) says `2 × max block body size, matching today's mainnet convention`; in linear-Leios the largest single block that drains the mempool is an EB, so the simulator interprets "max block body size" as the EB tx-referenced max. Document this interpretation in `protocol-base.yaml` with a comment pointing at the spec line.

New-arrival admission rejects when adding the tx would exceed the cap. **No eviction of existing valid txs to make room** — that's policy beyond the spec.

### Actor model

New module `sim-rs/sim-core/src/tx_actors.rs`. Actor profile is a list of weighted *components*; each component carries:
- arrival rate
- transaction size distribution
- value distribution (samples `value_lovelace`)
- urgency distribution (samples `urgency` — a real number `> 1` per the paper)
- `lane_policy`: utility-maximising lane choice (defined below)
- `max_fee_policy`: enum of named policies; default `ScaledOverLaneQuote { numerator: 4, denominator: 1 }` (rational multiplier — **not** f64, to keep simulation-affecting state deterministic across platforms) producing
  ```
  max_fee_lovelace = minFeeB.checked_add(
      ceil_div_u128(quote_per_byte as u128 × bytes as u128 × numerator as u128, denominator as u128)
          .try_into()? // u128 → u64
  )?
  ```
  where `ceil_div_u128(a, b) = if a == 0 { 0 } else { 1 + (a - 1) / b }` — overflow-safe (never adds before dividing, so no `(a + b − 1)` overflow). **Validation at config load**: `denominator > 0` for `ScaledOverLaneQuote` (and `Multiplier`-typed fields generally); zero denominator rejects the config with a diagnostic. **On overflow** in any intermediate or the final `u128 → u64` downcast: the actor's tx-generation step returns a configuration/generation error with a diagnostic identifying the actor component and the overflowing inputs; the simulator does not panic. Result is always `≥ minFeeB` when generation succeeds.

A transaction sampled from a component records `urgency_component_index` for per-class welfare metrics.

**Welfare formulas** (pinned, not left to the implementer):

- `retained_value(value_lovelace, urgency, latency_blocks) = value_lovelace × urgency^(-latency_blocks)`. This is the Kiayias et al. paper's exponential-in-blocks decay (Kiayias et al., "Tiered Mechanisms for Blockchain Transaction Fees", arXiv:2304.06014). Latency conversion: `latency_blocks = latency_slots as f64 × block_generation_probability` (e.g., with `block_generation_probability = 0.05`, 20 slots → 1.0 block; latency may be fractional).
- `net_utility(tx) = retained_value(tx) − actual_fee_lovelace(tx)` for an *included* transaction.
- For an *evicted or unincluded* transaction: `retained_value = 0`, no fee paid, `net_utility = 0`. The actor simply lost the opportunity.
- For an *included* transaction whose `retained_value < actual_fee`, `net_utility` is negative (a regret event).

**Lane policy** (`utility-maximising`):

At submission time, the actor estimates expected net utility for each available lane and picks the maximum:

```
expected_utility(lane) = retained_value(value, urgency, expected_latency_blocks(lane))
                       − (minFeeB + current_quote(lane) × bytes)
```

`expected_latency_blocks(lane)` is computed from a per-lane rolling average of recent observed inclusion delays — initialised to a default at startup (1 block for `Lane::Priority`, `target_inclusion_blocks` from config for `Lane::Standard`; default 4 blocks for standard) and updated per inclusion event. This is the simulator's calibration default; document it as a calibration choice in M5.

If the maximum is negative for all lanes, the actor still submits (per phase-2 default — actors don't game submission). Configurable via component-level `submit_when_underwater` flag if needed for a future experiment.

**Lane-choice determinism**: lane choice is simulation-affecting (the resulting `posted_lane` flows through admission, inclusion, refunds, and controller samples). `urgency^(-latency_blocks)` cannot use `f64::powf` directly because IEEE-754 only mandates bit-exactness for `+ − × ÷ √` — `powf` is implementation-defined and may diverge bit-for-bit between x86_64 and aarch64 libm. Lane choice may use either a fixed-point approximation or a pinned `libm`-style implementation; whichever is chosen, **bit-identical lane-choice outputs across architectures are treated as a simulator invariant** and covered by the cross-architecture golden tests at M2. The lane-choice result is rounded into `i128` lovelace before the `>` comparison. Platform-dependent floating math is **not** acceptable here. Reporting-side `retained_value` (in metrics) may continue to use plain f64 since it never feeds back into simulation decisions.

**Why pin these formulas**: a clean-room actor model that says "utility-maximising" without a formula will silently re-invent old-branch behaviour by trial and error during implementation. The exponential-in-blocks decay is the paper's choice, the linear net-utility is standard, and the rolling-average expected-latency is the simplest spec-consistent way for an actor to estimate a lane.

**Numeric representation** (deterministic split):

- **Simulation-affecting state is integer/rational or uses bit-reproducible cross-platform math; never plain f64.** This includes admission, eviction, fee charging, controller coefficient, mempool byte tracking, `max_fee_lovelace` computation, and **actor lane choice**. Plain f64 is forbidden in any code path that determines tx outcomes, because cross-platform rounding could diverge given the same seed.
  - `quote_per_byte: u64` is stored directly (not derived from an f64 coefficient at query time).
  - The EIP-1559 update rule `c ← c · (1 + clamp((aggregateUtil − target)/(target · D), ±1/D))` is implemented with `u128` integer/rational arithmetic: `aggregateUtil = sum(numerator_bytes) / sum(denominator_bytes)` (rational), `target = (target_num, target_den)` (rational, e.g. `(1, 2)` for 0.5), `D` integer, clamped step expressed as a rational delta on `quote_per_byte`. Final `quote_per_byte` is integer-rounded once per update via `ceil` against `minFeeA` for the era floor.
  - All actor `max_fee_policy` variants use rational/integer arithmetic. `ScaledOverLaneQuote { numerator, denominator }` above is the model.
  - **Actor lane choice** uses a fixed-point approximation or a pinned `libm`-style `pow` for `urgency^(-latency_blocks)`, with bit-identical cross-architecture output treated as a simulator invariant. The result is rounded into an `i128` lovelace before the `>` comparison. See *Lane-choice determinism* above.
  - **Validation at config load**: `Multiplier { numerator, denominator }` and `ScaledOverLaneQuote { numerator, denominator }` both reject `denominator == 0`. `CapacityWeightedWindow` rejects length 0.
- **Reporting outputs are plain f64.** Welfare metrics in `events.rs` (`retained_value`, `net_utility`, `retained_value_ratio` for the metrics CSV/comparison tables) are computed and stored as plain f64. They are derived from the deterministic integer event stream (admission/eviction/inclusion/refund) but never feed back into simulation decisions.
- Bit-identical determinism is asserted on the integer event stream, not on the f64 metrics. Document this split in `CLAUDE.md` at M5.

### Metrics

New module `sim-rs/sim-cli/src/events.rs` (or split as needed). Welfare-metric core:
- Per-actor and per-urgency-component breakdowns: retained-value-ratio, net utility, latency, inclusion rate, eviction rate, refund total.
- Two-lane lane audit metrics (priority retained value vs standard).
- `time_series.csv` columns: slot, per-lane current quote, per-lane rolling-window utilisation, mempool bytes, included bytes/counts by lane, evictions, fees paid, refunds.
- `metrics_comparison.txt`: mechanism summary with the same per-actor/per-lane structure.
- `diagnostics.log`: resolved config, controller settings, invariant warnings (multiplier-floor breach attempts), partition-activation counts, run-level validation notes.

All numbers come directly from event-stream sums. There is no `effective_fee` relabelling layer — `actual_fee_lovelace` and `refund_lovelace` are emitted on the inclusion event by the simulation.

### Suite runner

New `sim-rs/sim-cli/src/bin/experiment-suite.rs` plus `runner.rs` and `suite.rs` modules. Capabilities required:
- Suite config: shared defaults, named jobs, optional compare-jobs, optional seed sweeps.
- Resumable: durable manifest under each suite output directory, per-job/per-attempt state.
- One-script timestamped run wrapper.
- No generated outputs committed.

## Configs and suites (authored from scratch)

The new branch carries no configs at the start. Authoring order: protocol baseline → demand profiles → pricing TOMLs → experiment overlays → suites.

**Protocol baseline** — `sim-rs/parameters/phase-2-sweep/protocol-base.yaml` overlaying `parameters/linear.yaml` with the spec's defaults (`minFeeA = 44`, `minFeeB = 155381`, `mempool_max_total_size_bytes = 2 × eb_referenced_txs_max_size_bytes` per the simulator's interpretation of the spec's "max block body size" in linear-Leios, default actor `max_fee_policy = ScaledOverLaneQuote { numerator: 4, denominator: 1 }`).

**Demand profiles** (initially 2; expand only if an experiment requires it):
- `paper_like_moderate.toml` — moderate demand, weighted high/medium/low urgency components matching the spec's value-urgency narrative.
- `paper_like_congested.toml` — sustained congestion to exercise quote drift, evictions, and partition activation.

**Pricing configs** by mechanism, one TOML per controller-tuning point:
- `eip1559_*.toml` — single-lane EIP-1559: window length × `D` × `target` sweep.
- `two_lane_priority_only_static_*.toml` — RB-reserved priority-only premium with `min_priority_premium_multiplier` sweep (×4, ×8, ×16).
- `two_lane_priority_only_unreserved_*.toml` — un-reserved priority-only premium with the same multiplier sweep.
- `two_lane_both_dynamic_partitioned_*.toml` — partitioned both-dynamic with `D_priority`, `D_standard`, multiplier-floor sweep.
- `two_lane_both_dynamic_unreserved_*.toml` — un-partitioned both-dynamic with the same.

Roughly 20-30 pricing configs total — substantially fewer than the `pricing-sim-base` count because there is no tiered surface to enumerate and no exploratory dead ends.

**Experiment overlays** (`phase-2-sweep/experiments/*.yaml`): thin overlays binding one pricing TOML × one demand TOML × any per-experiment protocol overrides.

**Suites** (`phase-2-sweep/suites/*.yaml`): seven authored suites covering the down-select evidence:

1. `phase-2-eip1559-robustness.yaml` — single-lane EIP-1559 robustness across `D`, target, window length.
2. `phase-2-eip1559-smoothing.yaml` — smoothing comparison (window length sweep).
3. `phase-2-priority-only-rb-reserved.yaml` — RB-reserved priority-only premium with multiplier sweep.
4. `phase-2-priority-only-unreserved.yaml` — un-reserved priority-only premium with the same multiplier sweep, comparing partition vs no partition.
5. `phase-2-two-lane-both-dynamic.yaml` — both-dynamic, partitioned and un-partitioned variants.
6. `phase-2-rb-scarcity.yaml` — restates the previous RB-scarcity experimental question on the two-lane mechanism: how does priority-lane access to one RB-worth of guaranteed service hold up when RB capacity is reduced? Compares baseline RB capacity vs reduced.
7. `phase-2-urgency-inversion.yaml` — restates the urgency-inversion question: under congested demand with mis-priced actors (high-urgency actors with low maxFee multipliers), does the priority lane still deliver urgency separation? Compares utility-maximising actors vs mis-priced actors on a two-lane priority-dynamic mechanism.

The "premium queue" experimental question is subsumed by suites 3 and 4 (RB-reserved priority-only is the spec equivalent of the previous "premium queue strict" intent). Documented in the suite's README, not split into its own suite, unless the multiplier sweep needs different actor demand.

If any experimental question doesn't translate cleanly to a two-lane formulation, drop or reframe it in M4 — don't reintroduce a tiered backend to preserve a question.

## Milestones

The rebuild is structured to prove the spec kernel first, then layer on the experiment surface. Each milestone is a green-build, green-test checkpoint.

### M1: Spec kernel — the smallest possible spec-faithful simulator

Goal: prove the new pricing semantics end-to-end on the simplest possible setup. No actor system, no suite runner, no config sweep.

- Branch from `main`, scaffold the new modules in `sim-core` and `sim-cli`.
- Implement `model.rs` with `Transaction` carrying `max_fee_lovelace`, `posted_lane`, `bytes`, `value_lovelace`, `urgency`, `urgency_component_index`. Add `Lane` enum.
- Implement `config.rs` with the new shape: protocol params, mempool gate config, capacity-weighted window config, lane-aware selection config. No tier types.
- Implement `tx_pricing/{mod.rs, window.rs, single_lane.rs}` from spec: `BaselinePricing`, `Eip1559Pricing`, `CapacityWeightedWindow`.
- Implement `sim/mempool_gate.rs`: admission, revalidation, lane-aware byte tracking. Reject-new-when-full only.
- Wire mempool gate and lane-aware selection into the leios block-production path on top of `main`. Single-lane assigns every tx to `Lane::Standard`.
- Inclusion path emits `posted_lane`, `served_lane`, `actual_fee_lovelace`, `refund_lovelace`.
- **Spec amendment**: update [mechanism-design.md:39](docs/phase-2/mechanism-design.md#L39) to clarify maxFee is a total-lovelace transaction field. Lands in this milestone, not later.
- **Hand-rolled deterministic generator** for testing — fixed sequence of `(value, urgency, bytes, maxFee)` tuples per slot. No actor model yet.
- Unit tests: window math on heterogeneous RB+EB blocks, maxFee admission rejection, quote-drift eviction, current-quote inclusion charging, refund formula, mempool-cap rejection, window length 1 reduces to per-block fill rate.
- Integration smoke: one short run with the deterministic generator at congestion levels chosen to force quote drift and evictions. Confirm refunds and evictions appear and are correct.

**Exit criterion**: green `cargo test`; smoke run produces refunds and evictions on the deterministic generator.

### M2: Two-lane and full mechanism set

- Implement `tx_pricing/two_lane.rs`: two controllers, multiplier-floor invariant, `LaneSelectionOrder`, partition × signal-source window-length choice.
- Implement lane-aware block selection: RB priority-only validity rule for RB-reserved variants, EB binary fullness trigger per the rules above.
- **Smoke tests at M2 are deterministic** (actor system and suite runner arrive at M3): hand-rolled scenarios for each variant — RB-reserved priority-only-static-standard, RB-reserved both-dynamic, un-reserved priority-only, un-reserved both-dynamic. Each scenario hand-tunes mempool arrivals to exercise: multiplier-floor enforcement, EB partition activation under saturation, EB partition non-activation under empty mempool, RB validity rejection of standard-fee txs, lane-mismatch refund accounting, `priority_first` vs `fifo` selection order.
- Sanity check: priority lane retains more value than standard under congestion in both partitioned and un-partitioned setups.

**Exit criterion**: all four two-lane variants green on deterministic smoke tests; multiplier-floor invariant maintained across controller updates; partition activation logic green on both empty-mempool and saturation cases.

### M3: Actor model + metrics + suite runner + authored phase-2 suites

- Implement `tx_actors.rs` from scratch: weighted multi-component value-urgency sampling, `lane_policy`, `max_fee_policy` (default `ScaledOverLaneQuote { numerator: 4, denominator: 1 }`).
- Implement `events.rs` welfare-metrics core: per-actor/per-component breakdowns, lane audit, time-series, comparison tables, diagnostics.
- Implement runner + suite-runner CLI with resumable manifest.
- Author `protocol-base.yaml`, `paper_like_moderate.toml`, `paper_like_congested.toml`.
- Author pricing TOMLs and experiment overlays for the first five suites: `eip1559-robustness`, `eip1559-smoothing`, `priority-only-rb-reserved`, `priority-only-unreserved`, `two-lane-both-dynamic`.
- Run all five suites end-to-end. Confirm completion, output schemas, qualitative welfare results match spec expectations (priority retains more value under congestion, multiplier-floor invariant visible in price traces, etc.).

**Exit criterion**: five suites green; output schemas (`metrics_comparison.txt`, `time_series.csv`, `diagnostics.log`) match the spec for the metrics layer.

### M4: Reframed experimental questions (RB scarcity, urgency inversion)

- Author `rb-scarcity.yaml` suite restating the RB-capacity scarcity question on the two-lane mechanism. New protocol-overlay TOMLs for reduced RB capacity. Run; confirm the experimental question (does priority-lane access hold up under RB scarcity?) is answerable from the output.
- Author `urgency-inversion.yaml` suite restating the urgency-inversion question on a two-lane priority-dynamic mechanism. May require a new demand profile with mis-priced actors. Run; confirm the experimental question is answerable.
- Document in each suite's README how the new framing relates to the previous tiered-backend formulation. If a question genuinely doesn't translate, drop it with a written rationale rather than reintroducing a tiered backend.

**Exit criterion**: 7 suites total green; all phase-2 experimental questions either expressed cleanly as two-lane experiments or formally dropped with rationale.

### M5: Determinism, docs, finalisation

- Implement `tests/determinism.rs` and a `.goldens.<step>.sha256` regime. Tag the branch immediately after first golden generation.
- Author the new branch's `CLAUDE.md` describing the layout, mechanisms, and conventions.
- Update [docs/phase-2/mechanism-design.md §Methodology](docs/phase-2/mechanism-design.md#L295-L308): hard simulator/spec divergences reduce to 1 entry (anti-standard cap under FIFO fallback). Separately document the simulator's concrete defaults for spec-open questions: window length 32, update cadence per priced block, unreserved priority signal option 1, both-dynamic standard signal capacity-weighted aggregate over total block capacity, actor maxFee policy `ScaledOverLaneQuote { numerator: 4, denominator: 1 }`. These are calibration choices made explicit; the spec leaves them open and the simulator picks defaults.
- Size sanity check (guardrails, not acceptance criteria): roughly `sim-core/src/tx_pricing/` ~3,500 lines, `sim-core/` total ~10,000 lines, `sim-cli/src/events.rs` ~3,500 lines, full simulator ~12,000 lines. Behavioural test coverage is the actual exit criterion. Don't let line counts drive awkward factoring.

## Verification

End-to-end:
- `cd sim-rs && cargo build --release` — clean compile.
- `cd sim-rs && cargo test` — all unit + integration tests pass.

New unit tests at M1:
- maxFee admission rejects when prospective `posted_fee > max_fee_lovelace`.
- Quote drift after admission triggers eviction on next controller update.
- Inclusion fee at `served_lane` matches `minFeeB + current_quote × bytes`.
- Refund equals `max_fee_lovelace − actual_fee_lovelace`.
- `CapacityWeightedWindow` math correct on heterogeneous RB+EB blocks.
- Window length 1 reduces to per-block fill rate (regression test for spec-priority-controller equivalence).
- Mempool cap rejects new arrivals at capacity; no eviction of valid txs.

New unit tests at M2:
- RB partition rejects standard-fee tx in RB-reserved variants (validity error, not a cap).
- RB has no lane-validity rule in un-reserved and single-lane mechanisms.
- EB binary fullness trigger: priority partition not activated when remaining EB bytes accommodate at least one valid unselected mempool tx; partition not activated when mempool is exhausted; partition activated when no remaining tx fits.
- Lane-mismatch refund formula is general: `refund_lovelace = max_fee_lovelace − actual_fee_lovelace` regardless of `posted_lane` vs `served_lane`. Test cases: (a) `posted_lane = Priority`, `served_lane = Standard`, `max_fee_lovelace` set to the current priority fee → refund equals priority fee − standard fee; (b) same lane mismatch but `max_fee_lovelace` set above the priority fee → refund equals `max_fee_lovelace − actual_standard_fee`. Both cases must produce the formula's exact value, not a hardcoded "priority − standard" shortcut.
- Multiplier-floor invariant: `c_priority ≥ multiplier_floor × c_standard` after every controller update, including when `c_standard` moves.
- **RB-reserved standard isolation**: in partitioned both-dynamic, an RB filled with priority-fee txs updates `c_priority` but does **not** change `c_standard` or the standard-controller's window state. Test: assemble a saturated priority-only RB, snapshot `c_standard` and the standard window before/after, assert no movement.
- **Cross-platform determinism**: run the same seeded scenario on x86_64 and aarch64 (or via a soft-float test harness) and assert bit-identical event-stream SHA256. This catches any accidental f64 entry into simulation-affecting state at the tightest possible feedback point.

Suite-level at M3 and M4:
- Each suite completes; manifest is well-formed; resume works after a forced kill.
- `metrics_comparison.txt` includes per-actor and per-component breakdowns; `evicted_quote_drift` and `refund_lovelace` columns populated.
- Determinism: two seeded runs of the same config produce bit-identical event-stream SHA256.
- Qualitative spec checks: priority lane retains more value than standard under congestion in priority-bearing mechanisms; multiplier-floor invariant visible in `c_priority` time series; RB-reserved variants reject standard-fee txs in RBs.

## Risk notes

- **No code, configs, or schemas are taken from `pricing-sim-base`.** That branch is observable as prior art for calibration intuitions only. Resist the temptation to import the actor-component schema or the suite-runner architecture verbatim — describe the interface in the plan, then implement from the spec.
- **Lane-aware block selection is the most subtle code in the rewrite.** The RB priority-only validity rule (variant-conditional), the EB binary fullness trigger, and the `posted_lane`/`served_lane` distinction interact during selection. Get this right with thorough unit tests at M2 — patching it later through metrics is exactly the trap the rewrite is meant to avoid.
- **Spec amendment at M1**: don't ship M1 with the simulator and spec text disagreeing on maxFee semantics. Either both say total-lovelace, or both say per-byte-coefficient — not one of each.
- **Calibration defaults are labelled, not hidden**: window length 32, update cadence per priced block, unreserved priority signal option 1, etc. These appear in protocol-base.yaml with comments pointing at the spec's open-question section. Document in M5 alongside the methodology table reduction.
- **Suite reframing at M4 is the riskiest step**. If `phase-2-rb-scarcity` or `phase-2-urgency-inversion` doesn't translate cleanly to a two-lane formulation, drop the suite with a written rationale. **Don't reintroduce a tiered backend to preserve a question.**
- **Welfare evidence on `pricing-sim-base` is not comparable** to the new branch's output. The down-select arguments need to be re-run on the new branch; flag this in the CIP/handover docs.
- **Milestone discipline** prevents the rewrite from drowning in compatibility work before the core semantics are proven. M1 must be green and minimal before any actor model or suite runner work begins.
