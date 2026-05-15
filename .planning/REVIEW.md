# Code Review — dynamic-experiment vs main
Reviewed: 2026-05-13
Scope: 30 Rust files (sim-rs/), standard depth
Build/test state: not run (review-only)

> Provenance-filtered: findings that pre-exist on `main` have been removed.
> Excluded — former WR-1 (`LedgerState` / `resolve_ledger_state` unbounded ledger-state cache, `linear_leios.rs:297-313,1471` on main) and former IN-9 (`duration_as_secs` serialising `Duration` as `f32`, `sim-core/src/events.rs:858-859` on main). Remaining findings are caused by changes since main.

## Fix Status

Fixes applied: 2026-05-13. CR-1 was fixed in a prior commit. The
status of every other finding is recorded here. Build (`cargo build
--release`) succeeds; all 124 sim-core unit tests + 16 sim-cli tests
pass; no unit-test goldens flipped. Suite-level goldens
(`--release -- --ignored determinism`) were out-of-scope here and
were not re-checked.

| Finding | Status | Notes |
|---|---|---|
| CR-1 | Fixed (prior commit) | f64::sqrt → integer/libm replacement. |
| WR-1 | RESOLVED 2026-05-14 | Chain-derived implementation (spike 007) eliminates the contamination by construction — there is no node-local accumulator, so orphan-block samples cannot enter the canonical controller state. Every `LinearRankingBlock` carries `derived_quote: PerLaneQuote` as a pure function of `parent.derived_quote` + `parent.window_aggregate` + samples in canonical predecessors; sibling-block orphans are discarded with the block and carry no residual controller mutation. Empirically confirmed via `.planning/mechanism-welfare-impact-2026-05-14.md` — chain-derived produces clean welfare numbers on zero-slot-battle reference cases (matching the accumulator's pre-step behavior modulo the mechanism-cadence difference, which is a separate finding tracked in `.planning/chain-derived-bug2-investigation.md` and resolved by the Family B commitment). Family B (EIP-1559-faithful, 1-step-per-canonical-block) committed 2026-05-14 as the mechanism for publication; see `.planning/family-b-decision-2026-05-14.md`. |
| WR-2 | Deferred (design needed) | Adding `AdmissionRejected { reason }` to the event stream is a non-trivial public-API change with potential golden impact. Needs a design pass before code. |
| WR-3 | Applied | `debug_assert_eq!(mempool_max_size_bytes, gate.max_total_size_bytes)` in `LinearLeiosNode::new` and `debug_assert!(mempool_count <= queue.len())` invariant in `Mempool::try_insert`. |
| WR-4 | Applied | Lifted overflow validation into `Eip1559Settings::validate`: requires `window_length × target_num × max_change_denominator ≤ 2^23`, which keeps the u128 intermediates safe for per-sample bytes up to 2^40. Existing `saturating_mul`s left as belt-and-braces with an updated comment. |
| WR-5 | Applied | `eb_endorsement_valid` now splits the match: a `None` posted-fee (arithmetic overflow) logs `tracing::warn!` with tx id, quote, bytes, and min-fee-b before returning false. Genuine `max_fee_lovelace` exceeded still returns false silently. |
| WR-6 | Applied | `debug_assert!(self.representative_node.is_some(), ...)` in `is_representative`. The lazy-fallback test was split into a `cfg(not(debug_assertions))` release-only test plus a `cfg(debug_assertions)` `#[should_panic]` test that locks the assertion in. Existing tests that ingested `PricingTick` without pre-setting the representative were updated to call `set_representative_node("n0")`. |
| WR-7 | Deferred (out of v1 scope) | Refactoring `ActorComponent` sampling helpers to take fields by reference is 50-100 lines touching the lane-choice math; needs careful human review of the f64 → libm::round → i128 pipeline. CLAUDE.md flags perf as out-of-v1-scope. |
| IN-1 | Applied | Dropped the no-op `.into()` after `.checked_mul(2)`. |
| IN-2 | Applied | Replaced the "(safe: D\|den_val? not always)" comment with a one-line proof that `den = util_den · target_num · D` is structurally divisible by D. |
| IN-3 | Considered, not applied | The suggested `debug_assert!(samples.iter().any(...))` is incorrect: `TwoLanePricing::update_after_block` passes the same unfiltered slice to both lanes' `step_with_lane`, so a "no matching samples" slice is the normal case for whichever controller didn't have a sample emitted this block. Added a comment documenting why no debug_assert is appropriate here. |
| IN-4 | Applied | Bounded `multiplier_floor` ratio at 2^32 in `TwoLaneSettings::validate`; realistic suites use 4/8/16. Updated `enforce_multiplier_floor` comment to note `validate` catches pathological ratios. |
| IN-5 | Applied | Derived `PartialEq, Eq` on `TimeSeriesRow` and rewrote `is_zero_row` as `row == &TimeSeriesRow { slot: row.slot, ..Default::default() }` so any future column is automatically covered. |
| IN-6 | Applied | Removed the redundant trailing `manifest.save` per loop iteration — Completed/Failed arms already persist. Defensive comment retained. |
| IN-7 | Applied | `m1_smoke.rs` and `m2_two_lane.rs` test drivers now use `BTreeMap<NodeId, EventResult<...>>` for the per-tick updates accumulator. Iteration is single-node today so order doesn't matter; switching to BTreeMap future-proofs the driver shape for multi-node reuse without affecting the unit-test goldens (which were re-verified). |
| IN-8 | Applied | `lane_choice::pick` tie-break now uses `match cmp { Greater => Priority, Equal | Less => Standard }` instead of the implicit `>` ternary. Behaviour identical. |

Deferred items (WR-2, WR-7) are surfaced to the user for explicit
decision. WR-1 was resolved 2026-05-14 via the chain-derived
controller refactor (spike 007 ADOPT verdict + `.planning/
chain-derived-controller-PLAN.md`); the rollback-implementation /
disclosure paths are no longer needed.

## Follow-on work (tracked)

The items below are not findings from the original review — they are
follow-on actions required by the chain-derived refactor and the
Family B mechanism commitment, plus the two deferred findings
hoisted into a single trackable list. The user can pick them up
later.

| # | Action | Status | Notes |
|---|---|---|---|
| F1 | Re-run all 19 phase-2 suites under chain-derived for publication-grade numbers | OPEN | Required by Family B mechanism choice (2026-05-14); compute investment ~hours per full sweep × 3 seeds. The runner is resumable; only chain-derived runs need to be (re-)generated. See [`family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md) for the rationale + welfare-impact data driving the decision. |
| F2 | Property-based test ensuring future implementations match the EIP-1559-faithful 1-step-per-canonical-block semantics | OPEN | One regression test added during bug-1 fix (`admission_uses_post_step_quote_at_chain_tip` in `mempool_gate.rs::tests`); broader coverage — e.g., "controller steps exactly N times over a canonical chain of length N" / "deferred-EB validation fires zero controller steps" — would lock in the Family B commitment against future regressions. |
| F3 | WR-2 (gate-reject info loss) — design pass for `AdmissionRejected { reason }` event | OPEN | Deferred from code review; non-trivial public-API change with potential golden impact. Surfacing rejection-cause counts is essential to interpret the sustained-overload calibration regime (~97-99% rejection rates) but adding the event needs a careful design pass before code. |
| F4 | WR-7 (`ActorComponent` reallocation refactor) — perf cleanup | OPEN | Deferred from code review; out of v1 scope per CLAUDE.md (~50-100 lines touching the lane-choice math; needs careful human review of the `f64 → libm::round → i128` pipeline). Cumulative cost matters for M6 multi-producer suites (4 components × 1000 slots × 100 nodes ≈ 400k short-lived allocations per run). |

## Summary
- Total findings: 16 (Critical: 1, Warning: 7, Info: 8)
- Overall verdict: The phase-2 rebuild is in good shape. The pricing kernel, mempool gate, and event-stream hashing are all tightly designed with the integer/rational discipline the spec demands. The pricing-state-no-rollback gap and the legacy-producer hashmap-iteration bug are flagged in CONCERNS.md and confined to dormant or future paths. The single Critical finding is a real f64 read in a simulation-affecting hot path (`StalenessPredictor` driver) that is **already** covered by intra-arch goldens but mixes f64 with non-libm `sqrt` — easy to harden. Several Warnings concern subtle hygiene issues around mempool-gate ↔ mempool cooperation, EB-endorsement validation under multiple lanes, debug_assert reliance, and runtime ergonomics. Nothing here would prevent shipping current single-producer suites, but the M6 multi-producer work will surface most of the Warnings.

## Critical Findings

### CR-1: `endorsement_window_priced_blocks` uses `f64::sqrt` in a simulator-decisive code path
- File: `sim-rs/sim-core/src/sim/linear_leios.rs:407-418`
- Description: The function computes a 2-sigma Poisson upper bound `n = ceil(μ + 2·√μ)` using `f64::sqrt` (not `libm::sqrt`), then casts to `u32`. The result feeds `worst_case_quote_at(...)` inside `StalenessPredictor`, which decides at packing time whether to drop priority-fee txs from the EB body. **This is simulation-affecting state** — the resulting `n` changes which txs land in the EB and therefore which `PricedBlockSample`s the controller sees and which `TXIncluded`/`TXEvictedQuoteDrift` events the event stream hashes. The doc-comment claims "f64 +, ×, and √ are bit-exact under IEEE-754; `libm::ceil` is bit-stable across architectures" but **`f64::sqrt` is NOT in IEEE-754's bit-exact mandate across architectures** (only +, −, *, /, FMA, and conversions are required to be correctly rounded; sqrt is allowed implementation latitude under §5.4.1, though most modern hardware paths are correctly rounded). The CLAUDE.md determinism contract explicitly forbids f64 in simulation-affecting state ("No f64 in simulation-affecting state. Hard rule from the plan; enforced by the cross-arch determinism golden hashes.").
- Why it matters: The intra-arch goldens pin this, but the project's stated cross-arch reproducibility goal would silently break on a non-x86_64 target. More importantly the rule "no f64 in hot paths" is breached here, so any subsequent change anywhere near this path becomes harder to reason about.
- Fix:
  ```rust
  // Replace f64::sqrt with libm::sqrt, or switch to an integer
  // approximation: μ = window_slots * p_num / p_den (rational),
  // then ceil(μ + 2·sqrt(μ)) via integer Newton's-method sqrt
  // on the numerator. Simpler short-term: use libm::sqrt:
  let mu = (window_slots as f64) * cfg.block_generation_probability;
  let bound = mu + 2.0 * libm::sqrt(mu);
  let n = libm::ceil(bound) as u32;
  n.max(1)
  ```
  Even better: switch to integer math entirely, since `block_generation_probability` is also f64 but is bounded in [0, 1] and `window_slots` is u64 — a fixed-point ×1e9 representation would be clean and would let goldens survive any arch.

## Warning Findings

### WR-1: `apply_priced_block` mutation has no rollback on slot-battle reorg (documented as "Known limitation (M1)" but worth re-stating as a Warning since M6 will exercise it)
- File: `sim-rs/sim-core/src/sim/linear_leios.rs:2018-2052` (the doc-comment) and `linear_leios.rs:1068-1091` (`finish_validating_rb_header`)
- Description: When `finish_validating_rb_header` resolves a slot battle (lower VRF wins, line 1078), the losing block is removed from `praos.blocks` and its certified EB is dropped from `incomplete_onchain_ebs`. But the controller update, gate `on_inclusion` removals, and `TXIncluded` events triggered by the losing block are **not rolled back**. The M9 shock metrics (`max_priority_shock_over_window` etc.) will reflect contamination from orphan-RB samples in any multi-producer scenario.
- Why it matters: All 7 phase-2 suites are single-producer, so this doesn't fire today. M6 ships the CIP-0164 600-pool topology and slot battles will fire. The mitigation metric `slot_battles_count` (M6) only quantifies the *upper bound* of the contamination; it doesn't fix it. This Warning is here to mark the gap as a published-results constraint: a paper running on M6 should disclose the rollback gap if it claims welfare numbers under multi-producer regimes.
- Fix: Long-term, snapshot `(pricing, gate)` at every `publish_rb` and re-apply samples for the canonical chain after `finish_validating_rb_header` restores. Tracked in CONCERNS.md "Pricing state has no rollback on fork / slot-battle".

### WR-2: `try_add_tx_to_mempool` collapses gate-reject and mempool-reject into a single `false`, losing rejection-cause counts for diagnostics
- File: `sim-rs/sim-core/src/sim/linear_leios.rs:1728-1751`
- Description: The two rejection paths (`gate.try_admit(...).is_err()` and `mempool.try_insert(...) == false`) both return `false` without distinguishing the cause. The `MempoolGate::try_admit` already exposes a rich `AdmissionRejection` enum (`InsufficientMaxFee`, `ByteCapExceeded`, `FeeOverflow`), but the simulator throws this information away. As a result, the metrics layer cannot answer "what fraction of admissions failed because of fee budget vs byte cap?" — a key calibration question highlighted in CONCERNS.md's "Mempool admission edge cases" gap.
- Why it matters: With sustained-overload calibration producing ~97-99% rejection rates (CONCERNS.md), distinguishing fee-budget vs byte-cap rejection is essential to interpret results. Re-running suites for an hour each just to surface this is the wrong way to recover it.
- Fix: Propagate `AdmissionRejection` upward and emit a per-rejection-reason event (or a counter in the gate) consumable by `MetricsCollector`. Backwards-compatible: add `AdmissionRejected { reason: ... }` event without removing the boolean return.

### WR-3: Mempool's internal "queue" path is dead code, but is structured to subtly bypass the gate if reintroduced
- File: `sim-rs/sim-core/src/sim/linear_leios.rs:2553-2694`
- Description: The internal `Mempool::try_insert` (line 2572) has a "is full" branch that inserts the tx into a queue (line 2576-2578) without admitting it to the active mempool, returning false. The intent is "queue for later promotion when slack opens up". The caller (`try_add_tx_to_mempool`) responds to false by calling `gate.remove_silent(tx.id)` — reverting the gate. If `remove_conflicting_txs` then promotes the queued tx (line 2679: `newly_added.push(tx.id)`), it lands in the active mempool **with no gate entry** (no admission re-check, no per-lane bytes counter, no `max_fee_lovelace` tracked for revalidation). The CLAUDE.md comment on line 447 and the "Note for M2" comment at line 2108-2114 both observe this path is dead "under the gate-is-sole-byte-cap-authority invariant" — they're correct as long as `mempool.max_size_bytes == gate.max_total_size_bytes`. But there's no assert or invariant check preventing future configurations from diverging. (The `Mempool::try_insert` queue mechanism itself pre-exists on `main`; the bypass concern is phase-2-introduced because the gate is new.)
- Why it matters: A future config knob that adds slack to one but not the other (e.g. "soft cap" vs "hard cap") reintroduces this dead path silently. The tx promotes without gate state and the `TXIncluded` event reports fee/refund computed against the producer's quote rather than the admission quote.
- Fix: Add `debug_assert_eq!(mempool_max_size_bytes, gate_max_total_size_bytes)` in `LinearLeiosNode::new` (line 447), and a `debug_assert!(self.queue.len() <= self.mempool_count)` invariant inside `Mempool::try_insert`. Or, more aggressively, delete the queue/promotion paths entirely since they're unreachable.

### WR-4: `Eip1559Pricing::step` uses `debug_assert!` for u128 overflow checks but `saturating_mul` in release — silent saturation under pathological configs
- File: `sim-rs/sim-core/src/tx_pricing/single_lane.rs:189-199`
- Description: The function uses `debug_assert!(util_num.checked_mul(target_den).is_some())` and similar (lines 189-196) to flag overflow in dev builds, but the actual operations on lines 197-199 use `saturating_mul`. In release (which is what suite runs use), overflow silently saturates at `u128::MAX`, producing a controller step that's nonsensical. The intent comment ("flag pathological inputs in dev so we don't silently mask a real bug behind a saturated max value") acknowledges this exact failure mode but accepts it.
- Why it matters: A misconfigured suite (large `window_length`, gigantic per-block bytes) could saturate without any signal in the diagnostics log. Combined with `multiplier_floor` enforcement happening *after* the step, a saturated step would push priority quote to `u64::MAX` and the floor would clamp standard up to it.
- Fix: Either raise to `assert!` (so release builds bail loudly on bad config — this is config-time validation, fatal misconfigurations should panic at startup, not silently saturate during a 1000-slot run) or extend `Eip1559Settings::validate` to compute the maximum possible u128 intermediate from the field bounds and reject configs that could overflow.

### WR-5: `eb_endorsement_valid` walks every tx with `checked_mul` + `checked_add`; overflow returns false instead of representing genuine staleness
- File: `sim-rs/sim-core/src/sim/linear_leios.rs:886-904`
- Description: For each tx, the function computes `posted_fee = q.checked_mul(tx.bytes).and_then(|x| x.checked_add(min_fee_b))`. The match arm `Some(fee) if fee <= tx.max_fee_lovelace => continue, _ => return false,` treats both genuine `fee > max_fee` AND overflow-to-`None` as "stale, refuse endorsement". An overflow case (e.g. quote × bytes > u64::MAX) is *not* staleness — it's a pathological config — but is silently swallowed as if it were. The resulting endorsement refusal is correct in the sense that the EB shouldn't ship, but the event stream loses the distinction between "tx authorised insufficient max-fee" and "fee arithmetic overflowed".
- Why it matters: Conflates two different failure modes in the only place the simulator decides to drop endorsements. With overflow being silent the team can't audit how often it happens — and the same code path is on the producer's critical path each RB build.
- Fix: Split: on overflow, log a `warn!("EB endorsement skipped due to fee arithmetic overflow: tx={tx.id}, q={q}, bytes={tx.bytes}")` and still refuse, so the diagnostics log shows it. Or emit a dedicated event so the metrics layer can count overflow cases.

### WR-6: `MetricsCollector::is_representative` lazily pins on first observation (test fallback) but the lazy branch mutates state inside a `&self` -> `&mut self` method without comment
- File: `sim-rs/sim-cli/src/metrics/collector.rs:577-585`
- Description: `is_representative` takes `&mut self` and on miss (no representative pinned yet) writes `self.representative_node = Some(node_name.to_string())`. The pre-set-via-runner path uses `set_representative_node` (line 333). The CLAUDE.md note specifically says this lazy fallback "is for tests/standalone callers that don't pre-set; production runs through `runner::run_job` always pin deterministically." But there's no `debug_assert!(self.representative_node.is_some())` to surface a regression where the runner forgets to pin. If a future code change deletes `runner.rs:581-583` (`if let Some(name) = config.nodes.iter().map(|n| &n.name).min() { collector.set_representative_node(name.clone()); }`), the lazy-pin would silently take over and the time-series would depend on tokio scheduling instead of being deterministic.
- Why it matters: Latent regression risk on a key determinism invariant. Worse: in multi-node M6 suites, the wrong node winning the lazy pin under load could flip the time-series entirely; the goldens would catch it but only after re-running.
- Fix: Add `debug_assert!(self.representative_node.is_some(), "representative node should be pre-pinned by the runner; lazy fallback is for tests only")` in `is_representative`, OR rename the field to make the test-only path explicit, OR pass the representative-node-name via constructor and make `set_representative_node` panic if called twice with different names.

### WR-7: `run_actors_for_slot` clones `ActorComponent` data into `ComponentInputs` per slot, then rebuilds a temporary `ActorComponent` for sampling — large allocation amplification on the hot path
- File: `sim-rs/sim-core/src/sim/linear_leios.rs:2268-2429`
- Description: Each slot, for every component, the code builds a `ComponentInputs` struct (lines 2298-2318) and then a fresh `ActorComponent` (lines 2328-2339). With 4 components × 1000 slots × 600 nodes (CIP-0164 topology), that's 2.4M short-lived allocations per run for no algorithmic benefit. The comment "Build a temporary `ActorComponent` view for the sampling helpers — keeps the f64 → integer rounding/clamping logic in one place" is correct that this preserves rounding correctness, but the temporary allocation is avoidable: the sampling helpers (`sample_arrival_count`, `sample_tx_inputs`) take `&self` and only read fields, so passing the components by reference would be just as correct. (Performance is out of v1 scope per the review brief, but this is the worst offender I encountered and worth flagging as a Warning since it's also a clarity issue — the indirection obscures what the code is doing.)
- Why it matters: Cumulative cost in M6. More importantly the code is harder to audit because the actor lane-choice math has two materialisations of the same struct.
- Fix: Refactor `ActorComponent`'s sampling helpers to take their fields by parameter (or pass a `&ActorComponent` directly from `actor_state.profile.components`), removing the temporary. The `ComponentInputs`/`ActorComponent` rebuild dance becomes a 3-line capture-by-reference.

## Info Findings

### IN-1: `config.rs:1049-1056` uses `.checked_mul(2).into()` where `.into()` is identity on `Option<u64>`
- File: `sim-rs/sim-core/src/config.rs:1049-1056`
- Description: `params.eb_referenced_txs_max_size_bytes.checked_mul(2).into()` — the `.into()` is a no-op (`Option<u64>` → `Option<u64>`). Probably leftover from a refactor where the result type changed. Cosmetic but confusing.
- Fix: Drop `.into()`.

### IN-2: `single_lane.rs:214` comment says "den / D, integer division (safe: D|den_val? not always)" then proceeds anyway
- File: `sim-rs/sim-core/src/tx_pricing/single_lane.rs:214`
- Description: The comment expresses doubt about divisibility, but the math actually does always hold: `den = util_den · target_num · D`, so `D | den` is structurally true and `den / D = util_den · target_num` is exact. The comment introduces fear without grounds. (I verified by inspection.)
- Fix: Replace the parenthetical with a one-line proof: `// D | den by construction (den = util_den · target_num · D)`.

### IN-3: `Eip1559Pricing::step_with_lane` doc-comment claims the controller is used by `TwoLanePricing` but the body doesn't enforce that the caller used the right `Lane` filter
- File: `sim-rs/sim-core/src/tx_pricing/single_lane.rs:146-151`
- Description: `step_with_lane(lane, samples)` filters `samples.iter().filter(|s| s.controller_lane == lane)`. If the caller passes the wrong lane, the function silently produces a no-op step. No `debug_assert!` to surface accidental misuse.
- Fix: Optional `debug_assert!(samples.iter().any(|s| s.controller_lane == lane), "step_with_lane called with no samples of the expected lane")`.

### IN-4: `TwoLanePricing::enforce_multiplier_floor` saturates `u64::try_from(floor)` on overflow but only `debug_assert!`s
- File: `sim-rs/sim-core/src/tx_pricing/two_lane.rs:200-206`
- Description: Same pattern as WR-4. Release builds saturate; dev builds debug-assert. Less critical here because `floor` realistic values are well inside u64, but the same pattern would silently saturate on bad config.
- Fix: As with WR-4, validate at config time.

### IN-5: `MetricsCollector::is_zero_row` lists rows column-by-column; trivially error-prone if `TimeSeriesRow` grows a column
- File: `sim-rs/sim-cli/src/metrics/collector.rs:618-629`
- Description: The function tests an open-coded subset of `TimeSeriesRow` fields for zero-ness. New columns must be manually added here or the suppression heuristic silently misclassifies. Already missing `included_count_priority` and `included_count_standard` — those are checked, but `priority_window_util_x_1e9` and `standard_window_util_x_1e9` are not. So a row that has only window-util-x_1e9 set (no fee/inclusion activity, no pricing tick) gets suppressed. In practice the only way `priority_window_util_x_1e9` gets set is via `PricingTick`, which also sets `c_*_quote_per_byte` — so the heuristic accidentally works. But it's an accidental correctness, not designed correctness.
- Fix: Replace with `row == &TimeSeriesRow { slot: row.slot, ..Default::default() }` to compare against an all-default row including any future field, or test all fields explicitly.

### IN-6: `runner.rs::run_suite_with_run_id` always calls `manifest.save(&manifest_path)` twice per job (line 230 + 250 or 276 + 280) — last save is redundant when no error path triggered
- File: `sim-rs/sim-cli/src/runner.rs:230,250,276,280`
- Description: Cosmetic. Defensive double-save, costs one extra fs-write per (job, seed). Across 7 suites × 8 jobs × 2 seeds = 112 redundant writes per full sweep. Bounded.
- Fix: Restructure to single save at end of each iteration, OR leave as defensive belt-and-braces.

### IN-7: `m2_two_lane.rs:213` and `m1_smoke.rs:181` iterate `updates: HashMap<NodeId, ...>` in test drivers — non-deterministic but harmless in single-node tests
- File: `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:213`, `m1_smoke.rs:181`
- Description: Test driver iterates a HashMap of per-node event results. Currently single-node so the iteration order doesn't matter. If these test drivers are reused for multi-node M2/M3-style tests in M6+, the determinism guarantee breaks. Flag-only, not a blocker.
- Fix: Use `BTreeMap` or pre-collect into a sorted Vec. Or document the single-node assumption in the driver.

### IN-8: `tx_actors.rs::lane_choice::pick` ties break to Standard via `>` rather than an explicit `cmp` — comment correct but cuter than necessary
- File: `sim-rs/sim-core/src/tx_actors.rs:340-344`
- Description: `if exp_util_priority > exp_util_standard { Lane::Priority } else { Lane::Standard }` — strict `>` means ties go to Standard. Documented and tested; just noting that an explicit `cmp` would be clearer.
- Fix: Replace with `match exp_util_priority.cmp(&exp_util_standard) { Ordering::Greater => Lane::Priority, _ => Lane::Standard, }` for clarity.

## Files Reviewed (with per-file note)

- `sim-rs/sim-cli/src/bin/experiment-suite/main.rs` — Clean clap-based CLI dispatch. Nothing notable. No findings.
- `sim-rs/sim-cli/src/events.rs` — Legacy `EventMonitor` (single-binary path). Untouched by phase-2 except for new event arms (TXIncluded/TXEvictedQuoteDrift/PricingTick/LinearPricingSampleApplied) that it now ignores. Correct.
- `sim-rs/sim-cli/src/lib.rs` — Two-line surface declaration. Nothing notable.
- `sim-rs/sim-cli/src/metrics/collector.rs` — Phase-2 welfare-metrics core. Hash encoding matches M2 goldens. WR-6, IN-5.
- `sim-rs/sim-cli/src/metrics/comparison.rs` — Per-suite comparison writer. Correctly preserves negative `net_utility`. No findings.
- `sim-rs/sim-cli/src/metrics/diagnostics.rs` — Plain-text writer. No findings.
- `sim-rs/sim-cli/src/metrics/mod.rs` — Surface declaration. No findings.
- `sim-rs/sim-cli/src/metrics/time_series.rs` — Pinned CSV header. No findings.
- `sim-rs/sim-cli/src/runner.rs` — Manifest + suite runner. Resume semantics correctly handle Running → Pending on reload. The `verify_suite` malformed-hash check (line 468) is defensive and tested. IN-6.
- `sim-rs/sim-cli/src/suite.rs` — Trivial schema. No findings.
- `sim-rs/sim-core/src/config.rs` — All Raw* deserialisation + SimConfiguration. Validation is comprehensive. `auto_default_sources` (line 1119) is a thoughtful footgun guard. IN-1.
- `sim-rs/sim-core/src/events.rs` — Event enum + EventTracker. Backwards-compat defaults on TXGenerated are correctly documented. No findings.
- `sim-rs/sim-core/src/lib.rs` — Module declarations. Phase-2 added `tx_actors` and `tx_pricing`. No findings.
- `sim-rs/sim-core/src/model.rs` — Transaction, blocks, ledger types. Manual `PartialEq` on `Transaction` correctly bit-compares `urgency: f64` (handles NaN deterministically). No findings.
- `sim-rs/sim-core/src/sim/leios.rs` — Legacy protocol arm. Phase-2 diff is just adding `slot` to `track_transaction_generated` calls (correctly pinned to 0 for non-actor paths). No findings.
- `sim-rs/sim-core/src/sim/linear_leios.rs` — The phase-2 protocol. CR-1, WR-1, WR-2, WR-3, WR-5, WR-7, IN-7.
- `sim-rs/sim-core/src/sim/mempool_gate.rs` — Sole byte-cap authority. Tight, well-tested. No findings.
- `sim-rs/sim-core/src/sim.rs` — Variant-polymorphic simulation top. No phase-2-specific findings.
- `sim-rs/sim-core/src/sim/stracciatella.rs` — Legacy protocol arm. Phase-2 diff is just the `track_transaction_generated` slot arg, same as `leios.rs`. No findings.
- `sim-rs/sim-core/src/sim/tx.rs` — Legacy TransactionProducer, silenced when actor profile is present. `HashMap<NodeId, NodeState>` is iterated by `node_lookup` construction (line 82-86) which would be non-deterministic, but the path is dead under phase-2 (config = None when actor profile is set). The HashMap-iteration concern is pre-existing on `main`; phase-2 did not add it. Flag only.
- `sim-rs/sim-core/src/tx_actors.rs` — Phase-2 actor model. `lane_choice::pick` is well-disciplined (f64 → `libm::round` → `i128` lovelace before any compare). The `urgency_from_half_life_seconds` exit point is bit-stable via `libm::exp`. IN-8.
- `sim-rs/sim-core/src/tx_pricing/mod.rs` — Trait + types. Defaults are correct (single-lane backends inherit one-Standard-sample-per-block). IN-3.
- `sim-rs/sim-core/src/tx_pricing/single_lane.rs` — BaselinePricing + Eip1559Pricing. Step rounding is `ceil`-correct per spec (well-tested). WR-4, IN-2.
- `sim-rs/sim-core/src/tx_pricing/two_lane.rs` — TwoLanePricing + 4 variants. Multiplier-floor enforced post-update on `quote_per_byte` with u128, exactly as the spec requires. Constructor-time floor enforcement is also correct. IN-4.
- `sim-rs/sim-core/src/tx_pricing/window.rs` — CapacityWeightedWindow. u128 ring buffer. Sum-bytes / sum-capacity rational. Well-tested. No findings.
- `sim-rs/sim-cli/tests/determinism.rs` — Suite-level golden-hash regime. Read `UPDATE_GOLDENS=1` env var. Defensively asserts hash length and hex-ness. No findings.
- `sim-rs/sim-core/src/sim/tests/m1_smoke.rs` — M1 integration smoke. Builds one-node config from `config.default.yaml`, exercises refunds + evictions. Solid. IN-7.
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` — Per-variant deterministic scenarios. The refund-formula tests (case a/b) are excellent regression coverage for the `refund = max_fee - actual_fee` rule. IN-7.
- `sim-rs/sim-core/src/sim/tests/m3_actors.rs` — M3 actor-model tests. (Not read in full; skimmed file size 471 lines.) No findings surfaced from skim.
- `sim-rs/sim-core/src/sim/tests/mod.rs` — Module declarations. No findings.

## Cross-cutting observations

1. **The integer/rational discipline is honoured pervasively in the pricing kernel and mempool gate.** The single f64 leak I found (CR-1 in `endorsement_window_priced_blocks`) is isolated and easy to fix; the broad commitment to `u64`/`u128` is real and the codebase reflects it.

2. **`debug_assert!` is used as a release-time safety net in three places (CR-1-adjacent, WR-4, IN-4) and silently drops to saturation in release builds.** All three are config-validity issues. The right fix is to lift those checks into `Eip1559Settings::validate` / `TwoLaneSettings::validate` / equivalent so misconfigurations fail at startup rather than producing a saturated quote in the middle of a 1000-slot run.

3. **HashMap iteration order is non-deterministic in std**, and phase-2 added a handful of instances worth flagging for M6. The pricing-affecting HashMaps in `linear_leios.rs` (`leios.ebs`, `leios.votes`, `leios.missing_txs`, `leios.eb_peer_announcements`, `leios.certified_ebs`, `leios.incomplete_onchain_ebs`, `txs`, `ledger_states`) are *only* accessed by key (`.get`/`.insert`/`.remove`/`.entry`), never iterated, so they're determinism-safe in practice. The phase-2 HashMaps that ARE iterated (test drivers in m1_smoke/m2_two_lane, metrics collector `sample_producers_by_slot`/`components`) are all in either single-node test paths or order-independent computations. The legacy `TransactionProducer.nodes` map on `sim/tx.rs` also iterates, but is pre-existing on `main` and dormant under phase-2 (actor profile silences it). Latent risk for M6 multi-producer suites — flag for future work.

4. **The mempool ↔ mempool-gate cooperation has a documented but unenforced invariant** (gate.max_total_size_bytes == mempool.max_size_bytes) that, if it ever drifts, opens a path where txs land in the active mempool without gate state. Adding a `debug_assert_eq!` at construction would close this without runtime cost.

5. **The `partition_activated` bit on `LinearEndorserBlock` is a producer claim**, not a derivable property. Phase-2 is honest-producer-only so this is fine, but a published-attacker-model paper would need to either (a) move the trigger to a body-derivable invariant or (b) explicitly model "honest producer" as a security assumption. The CONCERNS.md security section mentions this; mentioning here in cross-cutting view because it'll matter for the CIP write-up.

6. **The verify-suite hash-malformedness defense (`runner.rs:468-475`) is exemplary**: rejects empty-or-non-hex stored hashes so empty-vs-empty doesn't silently pass. The two unit tests (`verify_suite_bails_on_empty_stored_hash`, `verify_suite_bails_on_non_hex_stored_hash`) lock this in. Use it as a template for similar defensive checks elsewhere.

7. **The CLAUDE.md project context is precise and accurate** about which paths are simulation-affecting vs reporting-only. Comments throughout the codebase faithfully reference the spec lines they implement. The single discrepancy: `endorsement_window_priced_blocks` (CR-1) uses f64 in a path the CLAUDE.md rule explicitly forbids, and the doc-comment defends the f64 use on the basis of an IEEE-754 property (sqrt being bit-exact) that is NOT actually mandated.
