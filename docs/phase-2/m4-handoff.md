# M4 → M5 handoff

> **Postscript (post-M5).** The "M3-class calibration" referenced throughout this handoff — single-producer + `rb-generation-probability: 1.0` — was a calibration bug, not a calibration choice. It prevented EB endorsement entirely (every RB's parent was 1 slot back, the 13-slot endorsement window failed every time, EBs never landed on chain). The mispricing-signal framing in §Decisions ("signal manifests in `refund_lovelace = 0`, not eviction") was correct *as a description of the buggy runs* but is no longer the right framing for the fixed runs. Post-fix, mispricing manifests in *both* refund and eviction. The rb-scarcity cliff explanation in §Known-limitation #7 was correct (priority demand exceeds RB body capacity, with no EB-partition relief) but the underlying calibration that produced it has been replaced. See [calibration-fix-postmortem.md](calibration-fix-postmortem.md) for the full explanation. The body of this handoff is preserved as historical context.

Audience: the engineer picking up [implementation-plan.md §M5](implementation-plan.md#L286)
(Determinism, docs, finalisation). Read alongside [mechanism-design.md](mechanism-design.md)
and [implementation-plan.md](implementation-plan.md) — those are
authoritative; this note is just the M4 delta on top of [m1-handoff.md](m1-handoff.md),
[m2-handoff.md](m2-handoff.md), and [m3-handoff.md](m3-handoff.md).

## Branch state

`dynamic-experiment` (no worktree, per project preference). M1 + M2 +
M3 + M4 ship as one accumulated delta on this branch.

- Build: `cargo build --release` clean.
- Tests: `cargo test --workspace` → 130 green:
  - 113 sim-core lib tests (unchanged from M3 — slot-on-TXGenerated
    is policy-pass-through and doesn't add new sim-core test cases).
  - 12 sim-cli lib tests (10 existing collector tests + 2 new
    `runner::tests::verify_suite_*` tests).
  - 4 gen-test-data + 1 sim-cli main (pre-existing).
- 7 phase-2 suites all run to completion. `experiment-suite verify`
  on all 7 reports `determinism verify ok: 72 (job, seed) pairs match`
  in aggregate (15 + 9 + 9 + 9 + 12 + 12 + 6).

The hard rules (no `pricing-sim-base` content; no f64 in
simulation-affecting state) held throughout. The new `slot: u64`
field on `Event::TXGenerated` is metrics-only — the pricing-event-stream
hash is over `TXIncluded` and `TXEvictedQuoteDrift` (M2 §gotcha #4),
so the M2/M3 cross-arch determinism golden hashes did not flip. All
five M3 suites verify against their M3-era persisted hashes.

## What M4 delivered

### New configs

- [parameters/phase-2-sweep/protocol-rb-reduced-half.yaml](../../sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-half.yaml)
  — `rb-body-max-size-bytes: 45056` (half of the default 90112). Per
  spec, `priority_reservation_bytes = max_block_size = rb_body_max_size_bytes`,
  so the EB priority partition halves in lockstep.
- [parameters/phase-2-sweep/protocol-rb-reduced-third.yaml](../../sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-third.yaml)
  — `30000`.
- [parameters/phase-2-sweep/protocol-rb-reduced-quarter.yaml](../../sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-quarter.yaml)
  — `22528`.

  All three are *full replacements* for `protocol-base.yaml`, not
  stacked overlays — see *Decisions M4 made* below.

- [parameters/phase-2-sweep/demand/paper_like_mispriced.yaml](../../sim-rs/parameters/phase-2-sweep/demand/paper_like_mispriced.yaml)
  — same three-component shape as `paper_like_congested.yaml`, but the
  high-urgency component (component 0) carries
  `MaxFeePolicy::ScaledOverLaneQuote { numerator: 1, denominator: 1 }`
  (zero quote-drift headroom). Other components keep the default
  `{4, 1}`.

### New suites

- [parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml)
  + [README](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.README.md)
  — 4 jobs × 3 seeds = 12 runs. Sweeps RB body cap across baseline,
  half, third, quarter on the `two_lane_priority_only_static_x4`
  pricing variant.
- [parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml)
  + [README](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.README.md)
  — 2 jobs × 3 seeds = 6 runs. Compares correctly-priced
  (`paper_like_congested`) vs mispriced
  (`paper_like_mispriced`) on the `two_lane_both_dynamic_partitioned_x4`
  pricing variant.

### Adjacent cleanups (folded into M4 per the plan-time directive)

#### `Event::TXGenerated` carries `slot: u64`

The pre-M4 metrics collector inferred `submit_slot` from
`self.delta.slot`, which depended on
`LinearLeiosNode::handle_new_slot` running `emit_pricing_tick(slot)`
*before* `run_actors_for_slot(slot)`. M4 removes that implicit
dependency: the actor's `slot` is captured on the event itself.

Files modified:
- [sim-core/src/events.rs](../../sim-rs/sim-core/src/events.rs) —
  `Event::TXGenerated` gains `slot: u64` with `#[serde(default)]` for
  legacy-trace compatibility. `EventTracker::track_transaction_generated`
  takes a new `submit_slot: u64` parameter.
- [sim-core/src/sim/linear_leios.rs](../../sim-rs/sim-core/src/sim/linear_leios.rs)
  — `LinearLeiosNode` gains `current_slot: u64`, set in
  `handle_new_slot`. Read by `generate_tx` (the lane-blind
  `handle_new_tx` path; actor path passes `slot` directly). Removed
  the now-obsolete ordering-invariant comment.
- [sim-core/src/sim/leios.rs](../../sim-rs/sim-core/src/sim/leios.rs),
  [sim-core/src/sim/stracciatella.rs](../../sim-rs/sim-core/src/sim/stracciatella.rs)
  — non-linear-Leios protocols pass `slot=0` for the mock paths and
  the call sites that have it in scope; these paths set
  `value_lovelace = 0` so welfare collapses regardless.
- [sim-cli/src/metrics/collector.rs](../../sim-rs/sim-cli/src/metrics/collector.rs)
  — reads `submit_slot` from the event field. The previous
  `delta.slot`-derived comment is replaced with a one-liner pointing
  at the event field. Test helper updated to default `slot: 0`.

#### `verify_suite` malformed-hash bail (verified pre-existing)

The check at [runner.rs:364-374](../../sim-rs/sim-cli/src/runner.rs#L364-L374)
already bails when the persisted `pricing_event_stream.sha256` is
empty/non-hex (commit `fb1b391`'s diff confirms it was added in M3).
M4 added two regression-locking tests at
[sim-cli/src/runner.rs::tests](../../sim-rs/sim-cli/src/runner.rs):
`verify_suite_bails_on_empty_stored_hash` and
`verify_suite_bails_on_non_hex_stored_hash`. Added `tempfile` as a
sim-cli dev-dependency.

## Decisions M4 made that M5 inherits

| Decision | Where | Why |
|---|---|---|
| **Per-job protocol overrides REPLACE the default-protocol path; they do not stack** | [runner.rs `run_job`](../../sim-rs/sim-cli/src/runner.rs#L426-L430) | The runner picks `overrides.protocol OR default_protocol`, never both. M4's first attempt wrote the RB-reduced overlays as one-line "diff" files (just `rb-body-max-size-bytes`); the runs produced zero events because the overlay replaced all the phase-2 mechanics from `protocol-base.yaml`. The fix was to make the overlays full protocol replacements (everything in `protocol-base.yaml` plus the RB knob). M5 should consider whether to extend `JobOverrides` with a `protocol_overlay: PathBuf` (additive layer) — but that's a runner-feature change, and M4's three-line YAML duplication is acceptable for now. |
| **`urgency-inversion` uses `multiplier_floor = 4`, not the spec default 16** | [phase-2-urgency-inversion.yaml](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml) | The first attempt with `x16` left priority demand too low (only high-urgency picks priority at 16×, ~12 KB/slot vs ~180 KB/slot capacity → no saturation, no drift). Switching to `x4` raises priority demand (high+medium pick priority, ~60 KB/slot) and the controller does drift (priority/standard ratio reaches ~50× under saturation). The README documents this calibration choice. M5 should consider whether the methodology table notes this as a "calibration choice the simulator made" alongside the others. |
| **Mispricing's experimental signal manifests in `refund_lovelace = 0` per component, not in `evicted_quote_drift_count`** | [phase-2-urgency-inversion.README.md](../../sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.README.md) | Under the M3 single-producer + `rb-generation-probability: 1.0` + 90 KB RB calibration, every priority tx is admitted and included in the same slot — no opportunity for revalidation to evict it before inclusion. The mispricing shape shows up instead in the per-component `refund_lovelace`: `mispriced_high_urgency` component 0 sees `refund=0` (max_fee = actual_fee, zero refund margin) while `correctly_priced` component 0 sees `refund ≈ 13B` (3× the actual fee, refunded). This is a real economic differentiation — actors mispriced at `{1, 1}` are paying every lovelace of their max-fee budget on inclusion — but it isn't the eviction-cascade the experimental design originally anticipated. The eviction shape would manifest under tighter calibrations: smaller RB capacity (see `rb-scarcity rb_reduced_third`/`quarter`) or multi-producer slot-battle scenarios where txs linger. |
| **`current_slot` on `LinearLeiosNode`** | [sim/linear_leios.rs](../../sim-rs/sim-core/src/sim/linear_leios.rs#L307-L342) | A small piece of node state that lets the lane-blind `generate_tx` path (called from `handle_new_tx`) emit `TXGenerated` events with the producer's most-recent slot. Updated at the top of `handle_new_slot`. Untouched by the actor path (which passes its own `slot` argument directly). |

## Where M5 picks up

Plan §M5 ([implementation-plan.md:286-291](implementation-plan.md#L286-L291))
has three work items: determinism golden tests, a new `CLAUDE.md`,
and the methodology-table reduction.

### M3-review forward-pointers explicitly assigned to M5

These were enumerated in the M4 brief and are NOT in M4:

1. **sim-cli unit-test coverage** beyond the verify_suite tests M4
   added. The collector's per-component aggregation, the runner's
   resume path, and the suite/comparison serialisation all have
   integration coverage but limited unit tests. M5 might fold this
   in alongside the determinism golden tests.
2. **JSON `kebab-case` style cleanup** — runner persistence files
   already serialise with `serde(rename_all = "kebab-case")` but the
   embedded `run_summary.json` field naming should be audited for
   consistency.
3. **`is_representative` polish** — the metrics collector picks the
   first node to emit `PricingTick` as the time-series source. Under
   single-producer all nodes converge, so this is moot. M5 should
   either pin a canonical node-selection policy (e.g., lowest
   node-id) or surface the choice in `diagnostics.log`.

### Methodology-table claim — what M4 didn't move

The implementation-plan.md preamble (line 23) claims that the
clean-room rebuild reduces the *Methodology* table's hard divergences
from 8 entries to 1 (anti-standard cap under FIFO fallback). M4
didn't move this count — the M3 simulator already implements the
spec at the level the table describes. **M5 should:**

- **Verify the count claim against the current
  [mechanism-design.md §Methodology](mechanism-design.md#L301-L316)
  table.** The table currently lists 8 simulator approximations.
  Walk each row, confirm the simulator now does the spec-correct
  thing (or document the residual deviation as a "calibration
  choice", not an "approximation"), and rewrite the table.
- **Document the M4 calibration choices** alongside the existing M3
  defaults (window length 32, update cadence per priced block,
  unreserved priority signal option 1, both-dynamic standard signal,
  actor maxFee policy `ScaledOverLaneQuote{4, 1}`):
  - **`urgency-inversion` calibration uses `multiplier_floor = 4`**
    instead of the spec default 16, to drive priority demand into
    saturation under the M3 single-producer topology. The signal
    surfaces in `refund_lovelace`, not `evicted_quote_drift_count`,
    under this calibration.
  - **`rb-scarcity` reduces `rb_body_max_size_bytes` and tracks the
    EB priority partition by spec invariance** (`priority_reservation_bytes
    = max_block_size`). The simulator honours the spec invariance via
    `SimConfiguration::build`'s assignment ([config.rs:1029-1036](../../sim-rs/sim-core/src/config.rs#L1029-L1036)).

### M4 calibration realities for M5 to surface

The two-lane priority service depends critically on the
`multiplier_floor` choice. Under
- `multiplier_floor = 16`: only urgency≥5 components find priority
  attractive on the utility-maximising lane choice. Priority demand
  is small.
- `multiplier_floor = 4`: urgency≥2 finds priority attractive.
  Priority demand is several-fold higher.

This is "as designed" — the multiplier_floor is the spec's
price-discrimination knob and changing it changes who self-selects
into priority. But the practical effect on which experimental
signals are visible (drift evictions vs refund-budget signals) is
calibration-dependent. M5's documentation should call this out so
future suite authors know to expect it.

## Known limitations carried forward + introduced in M4

### 1. Pricing state has no rollback on fork/slot-battle (carried from M1)

Same caveat. Single-producer suites don't exercise it.

### 2. EB partition activation is a producer claim, not derivable from the EB body (carried from M3)

Same caveat. M4's RB-reduced runs use a single producer so
producer/endorser agree by construction.

### 3. `PricingTick` is per-node; metrics use a single representative (carried from M3)

Same caveat. M4's two new suites are single-producer, so the
metrics representative is deterministic.

### 4. Cross-suite hash determinism via `verify_suite` is intra-arch only (carried from M2/M3)

Same caveat. M5/CI infrastructure should add a second-architecture
verification job.

### 5. Per-job protocol overrides REPLACE rather than stack (M4-introduced documentation)

See *Decisions M4 made*. Authoring a per-job protocol override
requires duplicating `protocol-base.yaml`'s contents into the
per-job overlay file. M5 may want to extend `JobOverrides` with a
`protocol_overlay: Vec<PathBuf>` for additive stacking — but this
is a runner-feature change, not blocking for M5's scope.

### 6. `urgency-inversion` produces a refund-budget signal, not an eviction-count signal, under the current calibration

See *Decisions M4 made*. The signal IS visible (refund_lovelace = 0
for mispriced component 0 vs ~13B for correctly-priced) but not as
the eviction cascade originally envisioned. M5's documentation should
describe this calibration honestly: the *experimental question is
answered* — yes, mispricing has costs and is operationally visible —
but the costs land in the refund envelope rather than in
service-level metrics under M3-class single-producer + rb-gen-prob=1
calibrations.

### 7. `rb-scarcity` has a steep cliff between half and third RB capacities

Inclusion counts: baseline ≈ 10500 → half ≈ 6000 → third ≈ 440 →
quarter ≈ 400.

**Standard lane stays empty across the entire gradient.** A direct
inspection of `metrics_comparison.txt` shows `standard_included = 0`
for every component on every (job, seed) — the suite is a
priority-only experiment from end to end. The mechanism is:

- Under utility-maximising lane choice with `multiplier_floor = 4`,
  every actor component (high/medium/low urgency) finds priority
  more attractive than standard, even at the heavily-drifted
  priority quotes the controller produces under reduced RB. Lane
  choice does **not** flip to standard.
- `c_priority` drifts ~5× upward at reduced_third — but
  `MaxFeePolicy::ScaledOverLaneQuote{4, 1}` (the default) gives 4×
  drift headroom, so the mempool gate doesn't evict the lingering
  priority txs.
- The EB priority partition activates under saturation, so
  excess priority demand goes into the EB's priority partition
  (one RB-worth), not into the EB standard space. There's no
  priority-to-standard refund because the non-activation path
  doesn't fire.
- Excess priority demand beyond `2 × rb_body_max_size_bytes` per
  slot accumulates in the mempool. By end of run, a large fraction
  of submitted txs are mempool-resident and never block-included.

The quarter-vs-third plateau (~400 each) reflects that both
regimes are deep in "priority capacity insufficient + mempool
overflow without standard substitution". Further RB cuts saturate
the same shape.

**Implications for M5**: under the M3 single-producer +
`rb-generation-probability: 1.0` calibration, RB scarcity does
not push txs onto the standard lane — it pushes them off-chain
(mempool-resident, unincluded). The experimental answer is in
the priority inclusion gradient and `priority_lane_retained_value_ratio`
(1.0 → 0.93 → 0.12), not in any priority-vs-standard split.
M5's documentation should describe this honestly; an earlier
revision of this handoff incorrectly anticipated a "standard-lane
absorbs the overflow" mechanism that the data does not show.

## Architectural changes from M3 (flagged for M5 readers)

These are the structural shifts in M4 that change how M3 code reads:

1. **`Event::TXGenerated` has a new `slot: u64` field.** Legacy
   traces deserialise with `slot = 0`. The metrics collector reads
   it directly; the slot-tick-before-actor-arrival ordering invariant
   is no longer load-bearing.
2. **`EventTracker::track_transaction_generated` takes a third
   parameter `submit_slot: u64`.** All call sites in
   `linear_leios.rs`, `leios.rs`, `stracciatella.rs` updated. The
   non-linear paths pass slot in scope where available, else 0.
3. **`LinearLeiosNode` has `current_slot: u64`** set in
   `handle_new_slot`. Existing `LinearLeiosNode` constructions that
   bypass `NodeImpl::new` would need to set it explicitly; in
   practice, all production paths go through `NodeImpl::new` which
   defaults it to 0.
4. **`sim-cli` declares `tempfile` as a dev-dependency.**

## Gotchas

1. **Per-job protocol overrides do not stack.** When authoring a new
   protocol overlay for a suite, copy *everything* from
   `protocol-base.yaml` into the new overlay and add the per-job
   knob — don't just write the knob and expect the runner to
   layer. The first M4 attempt produced zero-event runs because of
   this.
2. **Mispricing's experimental shape is calibration-dependent.**
   Under M3-class calibrations (single producer, rb-gen-prob=1.0,
   90 KB RB), mispricing produces a refund-budget signal, not an
   eviction signal. Both are real; the README explains the
   relationship. Don't tighten `multiplier_floor` or RB capacity to
   "force" evictions in this suite — that mixes two experimental
   questions. Use `rb-scarcity` for that shape.
3. **`urgency-inversion` and `rb-scarcity` both use `multiplier_floor
   = 4`** for the same reason: at 16, priority demand isn't large
   enough relative to capacity to surface controller drift in 200
   slots. M5's documentation should treat this as a calibration
   choice the simulator made, not an arbitrary suite-author choice.
4. **The RB-reduced overlays are full replacements**, so when M5
   adds new phase-2 protocol knobs to `protocol-base.yaml`, those
   need to be propagated into all three RB-reduced overlays
   manually — or the runner's `JobOverrides` schema needs to be
   extended with stacking semantics.
5. **`verify_suite`'s malformed-hash bail is now test-locked**, but
   only against the specific empty/non-hex shapes. M5 should add
   coverage for the case where the `.sha256` file exists with valid
   contents but the manifest's `run_summary.json` is corrupt — the
   verify path doesn't currently guard that combination explicitly.

## Test infra

- `sim-cli/src/runner.rs::tests::verify_suite_bails_on_*` are
  contained tests that build a temp suite + manifest + hash files
  and assert `verify_suite` errors. Use as a template if M5 adds
  more `verify_suite` corner cases.
- The two new suites' READMEs are the canonical reference for what
  signals to look for in their respective outputs. M5's CLAUDE.md
  should link to them.

## Recommended order of work for M5

1. **Verify the methodology-table claim** ([implementation-plan.md:23](implementation-plan.md#L23)).
   Walk the 8 rows of [mechanism-design.md §Methodology](mechanism-design.md#L301-L316),
   confirm each is now spec-correct (or annotate as "calibration
   choice"), and rewrite the table to its post-rebuild shape.
2. **Author the new branch's `CLAUDE.md`** ([implementation-plan.md:289](implementation-plan.md#L289))
   — layout, mechanisms, conventions. Link to the two new
   READMEs and document the `multiplier_floor = 4` calibration
   choice explicitly.
3. **Implement `tests/determinism.rs` and the `.goldens.<step>.sha256`
   regime** ([implementation-plan.md:288](implementation-plan.md#L288)).
   Tag the branch immediately after first golden generation.
4. **Fold in the M3 forward-pointers** that were deferred to M5: sim-cli
   unit-test coverage, JSON kebab-case audit, `is_representative`
   polish. Optional: extend `JobOverrides` with protocol-overlay
   stacking if it removes friction in future suites.

## Hard rules — restated

These rules held throughout M4 and remain in force for M5:

1. **No code, configs, types, or schemas from `pricing-sim-base`.**
   Observe it as prior art only.
2. **No f64 in simulation-affecting state.** M4's only new sim-affecting
   change is the `slot: u64` plumb on `TXGenerated`; that's integer.
   The `paper_like_mispriced.yaml` demand profile uses
   `ScaledOverLaneQuote{1, 1}` which is rational arithmetic.
3. **Suite reframing risk.** Both M4 suites translated cleanly; the
   user's directive ("default to keeping both") was honoured. If M5
   adds further suites, the same bar applies — drop with written
   rationale rather than reintroducing tier-vocabulary.
