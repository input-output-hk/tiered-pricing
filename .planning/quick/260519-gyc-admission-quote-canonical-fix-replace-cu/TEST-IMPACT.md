# Unit-test impact: `current_chain_tip_quote` canonical-tip fix

## Cargo test summary

Two test-binary runs reported. Combined: **153 passed, 1 failed, 0 ignored**.

- `sim-cli --lib`: 24 passed, 0 failed, 0 ignored
- `sim-core --lib`: 129 passed, **1 failed**, 0 ignored

Command: `cd sim-rs && cargo test --workspace --lib`
Log: `/tmp/quick-fix-cargo-test.log`

## Failing tests

- **`sim::tests::m2_two_lane::admission_uses_post_step_quote_at_chain_tip`**
  ([`sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`](../../../sim-rs/sim-core/src/sim/tests/m2_two_lane.rs), line 1115–1210)
  — Classification: **unexpected logic failure (investigate)** — NOT a
  pinned-golden-hash flip.

  The test was added 2026-05-14 (see in-test comment referencing
  `.planning/chain-derived-bug-investigation.md`) as a regression-guard
  against a one-step-lag re-introduction. Its core assertion at line 1201
  is `assert_ne!(consumer_q, stored_q)` — i.e., it requires the
  consumer-visible quote (`current_chain_tip_quote`) to **differ** from
  the chain tip's stored `derived_quote`. That asserts exactly the
  hypothetical-child-of-tip semantics the current quick fix intentionally
  removes.

  Panic excerpt:

  > assertion `left != right` failed: current_chain_tip_quote must
  > return the post-step state (after folding in the tip's own samples),
  > not the tip's stored derived_quote (which is one step behind).
  > consumer_q=137 stored_q=137 — if these are equal under saturating
  > demand, the one-step-lag bug has regressed.

  The fix's whole point is that `consumer_q` SHOULD equal the tip's
  stored `derived_quote` — that's the canonical-chain-derived semantic
  required for EIP-1559 protocol fidelity. So this test now encodes the
  inverse of the protocol-correct invariant. It is not in scope to edit
  the test under this quick task (the plan forbids changes to
  `m2_two_lane.rs`).

## Pinned-golden-hash tests (M2 / M3 layer)

All M2 / M3 pricing-event-stream golden-hash tests **passed** under the fix:

- `pricing_event_stream_deterministic_across_runs`
  (M2 RB-reserved both-dynamic scenario, GOLDEN
  `2c69ab58e4d76525d79df1dd68e6c539d8303fca95b44847243e0f062617ea79`)
  — PASS.
- `pricing_event_stream_deterministic_across_runs_unreserved`
  (M3 un-reserved scenario, GOLDEN
  `7a976da3778c11887665769a6af32eccc41f6d735b2140ef035fee67d05eb91c`)
  — PASS.
- `actor_event_stream_deterministic_across_runs` (M3) — PASS.

This is itself decision-relevant: the fix changes the consumer-visible
quote in `current_chain_tip_quote`, but the bit-identical pricing event
streams on the M2/M3 unit-test scenarios mean the change did not alter
admission / eviction / inclusion behaviour in those scenarios. Whether
that holds at suite-scale is the question Task 3 was designed to answer.

## Verdict

Verdict: UNEXPECTED — `sim::tests::m2_two_lane::admission_uses_post_step_quote_at_chain_tip` is a logic-assertion test (not a pinned-golden-hash) that hard-encodes the pre-fix hypothetical-child-of-tip semantic; under the plan's halt protocol this requires operator review before proceeding to the suite smoke run.

## Recommendation (for operator review)

This is not a regression in the fix — it is a stale assertion. The test
was written to guard against re-introducing the (now-superseded)
"one-step-lag bug" framing from 2026-05-14. The current canonical-tip
read is the protocol-faithful semantic per spike 007 / Family B
(CLAUDE.md §"Mechanism abstractions" and §"Mechanism choice and audit
trail (2026-05-14)"). The test's invariant — `consumer_q != stored_q`
under saturating demand — is the inverse of what protocol soundness
requires.

Operator options:

1. **Authorise replacing the assertion** in `admission_uses_post_step_quote_at_chain_tip`
   with `assert_eq!(consumer_q, stored_q)` (and rewriting the test's doc
   comment) before proceeding to the smoke run. This converts the test
   from a guard-against-old-bug into a guard-for-new-invariant. Counts
   as an authorised edit to `m2_two_lane.rs` (currently out-of-scope per
   plan); requires explicit operator go-ahead.
2. **Delete the test outright** — the canonical-tip invariant is
   already exercised structurally by the `pricing_event_stream_deterministic_across_runs*`
   golden-hash tests and the M2/M3 scenarios that depend on quote
   determinism.
3. **Halt and re-investigate** if there is doubt that the canonical-tip
   read is in fact the protocol-faithful semantic. (Unlikely given the
   Family B decision memo is the authoritative source on this question.)

No edits made to `m2_two_lane.rs` or `m3_actors.rs` under this task. No
goldens regenerated. Task 3 (smoke run) deferred pending operator
decision.
