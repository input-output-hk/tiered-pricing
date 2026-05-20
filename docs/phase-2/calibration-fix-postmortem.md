# Calibration-fix postmortem (post-M5)

## What was wrong

The phase-2 sweep's protocol baseline pinned
`rb-generation-probability: 1.0` and `topology-single-producer.yaml`
gave the sole producer `stake: 1`. Both were chosen with good
intentions — uniform tx-bearing-block-per-slot time series, simple
topology — and both were defensible in isolation. Together with the
linear-Leios endorsement protocol's stage delays, they produced a
silent calibration trap: **EBs never landed on chain**, so all
EB-borne tx service was invisible to the inclusion metrics.

Three independent factors compose to produce the bug:

1. **Endorsement window**: `try_generate_rb` enforces an
   `earliest_endorse_time` check at
   [sim-rs/sim-core/src/sim/linear_leios.rs:654](../../sim-rs/sim-core/src/sim/linear_leios.rs#L654).
   With config defaults (`header_diffusion_time = 1 slot`,
   `linear_vote_stage_length = 5`,
   `linear_diffuse_stage_length = 5`), the check requires
   `slot ≥ parent_rb.slot + 13`.

2. **rb-generation-probability = 1.0**: every slot wins the lottery
   and produces an RB whose parent is the immediately preceding RB.
   Gap is always 1 slot. The 13-slot endorsement window fails on
   every RB; `endorsement = None` always.

3. **`praos_fallback: true`**: when `endorsement.is_none()`, the
   producer fills the RB body via `sample_from_mempool_lane_aware`
   ([linear_leios.rs:711](../../sim-rs/sim-core/src/sim/linear_leios.rs#L711)).
   This produces RB-body inclusions every slot, *masking* the absent
   EB-endorsement path. The simulator looks like it's working — RBs
   land, txs get included, time-series rows fire — but the EB
   priority partition and the EB body are dark.

## Visible consequences

- **RB-reserved priority-only / both-dynamic suites**: standard-fee
  txs cannot enter the RB body (validity rule rejects them) and
  cannot enter the EB body either (no EB ever endorsed). Every
  `included_count_standard` is 0 across all components and all jobs.
- **Un-reserved suites**: standard-fee txs can enter the RB body
  alongside priority-fee ones (no validity rule), so they show
  meaningful standard inclusions. EB-borne service is still
  absent. The un-reserved-vs-RB-reserved comparison was inflated.
- **Latency-blocks-mean = 0**: priority-fee tx demand always fits
  the 90 KB RB body in the same slot it's submitted, so the
  RB-reserved metrics-comparison shows latency 0 across all
  priority components. This was previously framed as "M3 §9
  degeneracy" — a calibration-by-design choice. It is *also* a
  symptom of the EB-endorsement bug: with the EB partition never
  active, even saturated priority demand has no second-channel
  service to add latency.
- **rb-scarcity cliff** between `rb_reduced_half` and
  `rb_reduced_third` was real but mis-explained: the cliff is RB
  body running out of priority service with no EB-partition relief.
  The README originally hedged that the EB partition might absorb
  overflow; with no EB endorsement, that hedge was incorrect (and
  was already corrected once in the M4 review pass — see
  m4-handoff.md §Known limitation #7).

## Root cause and fix

**Root cause.** Two problems compose:

A. **Calibration**: `rb-generation-probability: 1.0` is incompatible
   with the linear-Leios endorsement window. To make EBs reachable
   the expected RB cadence must exceed 13 slots, i.e. probability
   ≤ 1/13 ≈ 0.077.

B. **Stake quantization**: `compute_target_vrf_stake(stake,
   total_stake, success_rate)` computes
   `(total_stake as f64 × ratio × success_rate) as u64`. With
   `stake = 1` and `success_rate = 0.05`, the result is
   `1.0 × 1.0 × 0.05 = 0.05` truncated to `0` — the lottery never
   wins. The fix needs enough stake granularity to preserve the
   probability under truncation.

**Fix** (in
[sim-rs/parameters/phase-2-sweep/protocol-base.yaml](../../sim-rs/parameters/phase-2-sweep/protocol-base.yaml)
and
[sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml](../../sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml)):

- `rb-generation-probability: 1.0` → `0.05` (Cardano-realistic).
  Expected RB cadence ~20 slots, comfortably clears the 13-slot
  endorsement window.
- `stake: 1` → `100000`. With probability 0.05 and stake 100000,
  `target_vrf_stake = 5000`, lottery wins iff
  `random_range(0..100000) < 5000` — exactly 5% per slot.

Compensating change to keep statistical signal:

- All seven suite YAMLs: `default-slots: 200` → `1000`. At ~5% RB
  probability, 1000 slots gives ~50 expected RBs per (job, seed),
  which is enough to surface meaningful per-component differences
  in inclusion, eviction, and refund.

## Re-runs and goldens

All seven suites were re-run end-to-end against the fixed
calibration. The seven `.goldens/<suite>.sha256` files were
regenerated via `UPDATE_GOLDENS=1 cargo test --release --
--ignored determinism` and pinned to the new baseline.

The M2/M3 in-process cross-arch unit-test goldens (in
`sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` and
`m3_actors.rs`) test specific scenarios with calibration values
pinned in test code — they are unaffected by this fix and continue
to pass.

The branch should be re-tagged after this commit lands;
`m5-goldens-v1` (which never landed) is superseded by
`m6-goldens-v1`.

## Reinterpretation of suite results

The new runs show a substantially different picture:

- **EB endorsement now fires.** RBs whose parents are 13+ slots
  back successfully reference certified EBs, EB-borne txs charge
  inclusions via `assign_served_lanes`, and standard-fee
  inclusions appear in `metrics_comparison.txt`.
- **Per-component latencies are non-trivial.** With sparser RB
  cadence, even priority service has latency: comp 0 shows
  6+ blocks under the urgency-inversion `correctly_priced`
  baseline, vs the previous 0.0.
- **Eviction counts are meaningful.** Quote drift under the new
  cadence pushes the controller to evict marginal-max-fee txs;
  evictions per (job, seed) range from hundreds to tens of
  thousands depending on the controller D and target.
- **Mempool overflow is the dominant constraint.** With ~150
  txs/slot demand against a ~5%-RB cadence, the 32 MB mempool
  cap (`2 × eb_referenced_txs_max_size_bytes`) saturates within
  ~250 slots. Beyond that, new-arrival admission rejections
  dominate over quote-drift evictions. Inclusion rates are
  ~1–3% — realistic for a sustained-overload regime, not a
  bug.

Conclusions previously drawn from the suite data must be
re-examined against the new runs. In particular:

- The "RB-reserved gives perfect priority service at zero
  latency" narrative was an artefact of the bug. Under the
  fixed calibration RB-reserved gives priority service with
  measurable latency that depends on RB cadence.
- The un-reserved-vs-RB-reserved comparison shifts: with EB
  endorsement working, RB-reserved's standard-lane service
  finally appears (refunded priority overflow + standard-fee
  txs in the EB body when partition not activated).
- The rb-scarcity "cliff" softens to a **gentle gradient** under
  the corrected calibration: comp 0 inclusion 2.7% → 1.5% → 1.0% →
  0.9% across baseline → half → third → quarter, with latency
  roughly flat at ~8 blocks. The bottleneck shifts from "priority
  capacity per slot" (the previous cliff explanation) to "RB
  cadence" (~1 RB per 20 slots) — at sparser RB cadence even
  baseline RB priority capacity is mostly idle within an RB cycle,
  so shrinking the RB body shrinks throughput proportionally
  rather than producing a sharp regime change.
- The urgency-inversion mispricing signal is now visible in
  *both* `refund_lovelace = 0` (carried over from M4) *and*
  `evicted_quote_drift_count` — mispriced high-urgency actors
  get evicted under quote drift now that the controller
  actually drives the quote past their `{1, 1}` budget.

## What this doesn't change

- **No simulator code changed.** The
  [linear_leios.rs:1948-1952](../../sim-rs/sim-core/src/sim/linear_leios.rs#L1948-L1952)
  observation — that `apply_eb_priced_block` does not call
  `charge_inclusions` for EB txs — is *not* a bug. The
  inclusion-charging path runs from `try_generate_rb` at the
  endorsing RB ([linear_leios.rs:691](../../sim-rs/sim-core/src/sim/linear_leios.rs#L691))
  and is correct when endorsement actually fires. The fix
  re-enables endorsement so that path runs.
- **The M5 8 → 1 methodology-table claim still holds.** The
  fix does not introduce any new sim-vs-spec divergences. The
  remaining residual divergence (anti-standard cap under FIFO
  fallback) is unchanged.
- **No hard-rule violations.** The fix is YAML-only; no f64
  enters simulation-affecting state, no `pricing-sim-base`
  content is imported, no determinism contract is weakened.
- **Workspace tests still 132 green.** The in-process unit-test
  goldens pass unchanged (they test scenarios pinned in code,
  not by config).

## Open follow-ups

- **EB priority partition activation has still not been
  empirically exercised.** With ~150 KB/slot demand against a
  16 MB EB body, the EB never reaches capacity, so the
  partition's binary trigger never fires. Conclusions about
  "the EB partition delivers one RB-worth of guaranteed
  priority service under saturation" still rest on
  spec-faithfulness, not on direct measurement. Exercising
  this would require either much higher demand or a smaller
  `eb-referenced-txs-max-size-bytes`.

- **Mempool cap dominates over quote-drift eviction** under
  this calibration. The simulator now models a sustained-overload
  regime where most demand is rejected at admission, not evicted
  by controller drift. Whether that's the right regime for the
  CIP write-up is a separate question.

- **The 13-slot endorsement window is itself a calibration
  knob.** A future calibration could reduce
  `linear_vote_stage_length` and `linear_diffuse_stage_length`
  to 0 (with single-producer + `vote_threshold = 1`, votes
  are immediate) and recover the high-cadence regime *without*
  the bug. Trade-off: less realistic Cardano timing, but
  potentially more mechanism-relevant signal density.

## See also

An additional implementation-vs-spec divergence was discovered and
resolved 2026-05-14 (the pre-2026-05-14 accumulator effectively
stepped the controller twice per RB-EB pair — once at RB publish via
`apply_priced_block`, once at deferred EB validation via
`apply_eb_priced_block` — diverging from the spec's per-block-cadence
intent); see
[`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md)
for the chain-derived (Family B, EIP-1559-faithful) refactor and the
publication-committed mechanism choice. This calibration-fix
post-mortem and the Family B decision are sibling events: each
corrected a divergence between intent (spec) and behaviour
(simulator), discovered through empirical revalidation rather than
by code inspection.
