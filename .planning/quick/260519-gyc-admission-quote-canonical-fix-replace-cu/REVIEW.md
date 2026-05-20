---
phase: quick/260519-gyc-admission-quote-canonical-fix-replace-cu
reviewed: 2026-05-20T00:00:00Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - sim-rs/sim-core/src/model.rs
  - sim-rs/sim-core/src/sim/leios.rs
  - sim-rs/sim-core/src/sim/linear_leios.rs
  - sim-rs/sim-core/src/sim/stracciatella.rs
  - sim-rs/sim-core/src/sim/tests/linear_leios.rs
  - sim-rs/sim-core/src/sim/tests/m2_two_lane.rs
  - sim-rs/sim-core/src/tx_pricing/mod.rs
  - sim-rs/sim-core/src/tx_pricing/single_lane.rs
  - sim-rs/sim-core/src/tx_pricing/two_lane.rs
findings:
  critical: 2
  warning: 6
  info: 4
  total: 12
status: issues_found
---

# Quick-task code review: scope-creep diff layered on top of `current_chain_tip_quote` canonical-tip fix

**Reviewed:** 2026-05-20
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

The quick-task `PLAN.md` describes a *surgical* change with `files_modified: [linear_leios.rs (function body + doc), one suite YAML touched-and-reverted, two planning docs]`. The committed fix (`449afd4`) and its `TEST-IMPACT.md` align with that scope.

The **staged-but-uncommitted diff under review now** is a different beast: 450 insertions / 310 deletions spread over 9 files, introducing a substantive new on-chain protocol object (`emitted_samples` on `LinearRankingBlock`), a vote-bundle field carrying eb-samples (`VoteBundle.eb_samples`), a vote-bookkeeping rekey (`votes_by_eb` -> `votes_by_eb_sample`), a parallel slot-battle resolver (`rb_slot_is_still_viable` / `adopt_rb_slot`), a parent-waiting queue (`rbs_waiting_for_parent`), and a new "EB-sample receiver-side validation" path (`validate_rb_pricing_fields`). It also deletes the `block_samples` cache, deletes `samples_for_rb` and `prune_block_samples`, and changes `ChainView::samples_in_block` to read off the RB header instead of the cache. The diff *also* edits `m2_two_lane.rs` to invert the very assertion the prior `TEST-IMPACT.md` flagged as out-of-scope (`assert_ne!` -> `assert_eq!`).

The semantic intent — replace per-node cache with on-block sample carriage so chain-derivation is bit-stable by construction — is consistent with Family B and looks correct in the happy path. But the diff has substantive **soundness, robustness, and scope-discipline** defects that I cannot wave through.

Three things make this risky:

1. The diff is **not described by any plan in the phase dir** — `PLAN.md`'s success criteria call for atomic commits, and the diff stages a single ~586-line monolithic delta to `linear_leios.rs` that is not partitioned into reviewable atoms.
2. The `TEST-IMPACT.md` workflow document encoded an "operator review required" halt before changing `m2_two_lane.rs`; the staged diff makes that exact change without recording authorisation.
3. The new `validate_rb_pricing_fields` path silently accepts producer-claimed `emitted_samples` for endorsed RBs when the local node has neither the EB body nor a vote certificate carrying samples for that EB — a real consumer-visible-quote divergence reintroduction.

## Critical Issues

### CR-01: `validate_rb_pricing_fields` accepts producer-claimed `emitted_samples` unverified when local node lacks the EB body AND has no certified-samples entry

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:2504-2530](sim-rs/sim-core/src/sim/linear_leios.rs#L2504)

**Issue:** In the endorsed-RB branch of `validate_rb_pricing_fields`, the receiver computes `expected_samples` as follows:

```rust
if let Some(certified_samples) =
    self.leios.certified_eb_samples.get(&endorsement.eb)
    && *certified_samples != rb.emitted_samples
{
    return RbPricingValidation::Invalid;
}
if let Some(eb) = self.get_received_eb(endorsement.eb) {
    self.samples_for_eb_body(&eb)
} else {
    // ... protocol-abstraction comment ...
    rb.emitted_samples.clone()  // <-- trust whatever the RB claims
}
```

So three cases:

1. `certified_eb_samples` populated and equal: pass.
2. `certified_eb_samples` populated and unequal: reject (correct).
3. `certified_eb_samples` empty AND `get_received_eb` returns None: **`expected_samples = rb.emitted_samples.clone()`, then `rb.emitted_samples != expected_samples` is trivially false, so this passes regardless of what the producer claimed.**

This means a receiver who has not seen enough vote bundles to populate `certified_eb_samples` and has not received the EB body will adopt the RB with whatever `emitted_samples` the producer chose. The downstream `compute_chain_derived_quote_for_child_of` then folds those samples into the child's controller step. Once `publish_rb` runs (line 1341 then 1116 calls `retry_rbs_waiting_for_parent`), descendants compute their own `derived_quote` against those unverified samples.

Two nodes at the same canonical tip can therefore diverge on consumer-visible quote whenever sample-bearing votes arrive **after** the RB is received — the exact per-node-divergence failure mode the quick-task fix at `current_chain_tip_quote` was installed to eliminate. The control flow that adopts an unverified sample is reachable under normal vote/RB propagation race conditions, not only under adversarial behaviour. The receiver-side check then attests "valid" by tautology because `expected_samples` was sourced from `rb.emitted_samples` itself.

**Fix:** When `certified_eb_samples` is absent and the EB body is also absent, the receiver must defer — `WaitingForParent` is the wrong tag, but the structural answer is the same: a new `RbPricingValidation::WaitingForVotes(EndorserBlockId)` arm queueing the RB until either votes-by-sample reach threshold OR the EB body arrives:

```rust
let expected_samples = match &rb.endorsement {
    Some(endorsement) => {
        // ... transaction-empty and threshold checks ...
        if let Some(certified_samples) =
            self.leios.certified_eb_samples.get(&endorsement.eb)
        {
            if *certified_samples != rb.emitted_samples {
                return RbPricingValidation::Invalid;
            }
            certified_samples.clone()
        } else if let Some(eb) = self.get_received_eb(endorsement.eb) {
            let computed = self.samples_for_eb_body(&eb);
            if computed != rb.emitted_samples {
                return RbPricingValidation::Invalid;
            }
            computed
        } else {
            // Cannot verify: defer until either the EB body or a certified
            // sample arrives. Queue analogous to rbs_waiting_for_parent.
            return RbPricingValidation::WaitingForEbSample(endorsement.eb);
        }
    }
    None => self.samples_for_rb_body(&rb.transactions),
};
```

The `incomplete_onchain_ebs` set already tracks "we have an endorsed EB we haven't validated", but it does NOT block adoption of the RB into the canonical chain — that's the gap. Either add a deferred queue keyed on the EB body / sample-certificate as above, or make `validate_rb_pricing_fields` Invalid (not Valid) for un-verifiable endorsements.

### CR-02: Staged diff makes `m2_two_lane.rs::admission_uses_post_step_quote_at_chain_tip` edit explicitly flagged as out-of-scope by `TEST-IMPACT.md` operator-halt protocol, without recording authorisation

**File:** [sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:1115-1179](sim-rs/sim-core/src/sim/tests/m2_two_lane.rs#L1115)

**Issue:** `.planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/TEST-IMPACT.md` (committed at HEAD as `3bf05a8`) records:

> Verdict: UNEXPECTED — `sim::tests::m2_two_lane::admission_uses_post_step_quote_at_chain_tip` is a logic-assertion test (not a pinned-golden-hash) that hard-encodes the pre-fix hypothetical-child-of-tip semantic; under the plan's halt protocol this requires operator review before proceeding to the suite smoke run.

The `PLAN.md` Task 2 `<action>` block similarly states: "Do NOT regenerate goldens. Do NOT edit `m2_two_lane.rs` or `m3_actors.rs`. Goldens regeneration is explicitly out of scope for this quick task." Task 2's halt protocol explicitly forbids proceeding without operator review when the verdict is UNEXPECTED.

The staged diff to `m2_two_lane.rs:1115-1179` performs exactly the edit (`assert_ne!(consumer_q, stored_q)` -> `assert_eq!(consumer_q, stored_q)`) the TEST-IMPACT operator-options bullet labelled option (1) — and *did not* record authorisation in the phase dir (no addendum to `TEST-IMPACT.md`, no `SUMMARY.md`, no operator note). This is a process violation, not a code logic bug; flagged as Critical because the project's `MEMORY.md` instructions say "Don't auto-commit — leave staged/unstaged changes for the user to commit themselves; skip commit/tag steps even when plans include them", and the staged-but-not-committed state is precisely the surface where these scope-discipline checks land.

The substantive question — whether the test's *new* assertion is the correct invariant — is settled by the canonical-tip semantic (the test, post-edit, asserts the correct invariant per spike 007 / Family B). The defect is scope and audit-trail, not correctness of the new assertion text.

**Fix:** Before committing the staged diff:

1. Either (a) split the `m2_two_lane.rs` edit into its own atomic commit with a message that explicitly acknowledges TEST-IMPACT.md's "operator option 1" authorisation, OR (b) drop the `m2_two_lane.rs` edit from this staging and address it in a follow-up commit gated by an explicit operator note in the phase dir.
2. Update `TEST-IMPACT.md` with a postscript naming the option taken (1 or 2) and the operator decision date, so the audit trail is recoverable.
3. The substance of the new assertion is fine — keep `assert_eq!(consumer_q, stored_q)`. Only the scope/audit-trail discipline is broken.

## Warnings

### WR-01: Slot-battle resolution duplicated in `finish_validating_rb_header` and the new `rb_slot_is_still_viable` / `adopt_rb_slot`

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:1174-1190](sim-rs/sim-core/src/sim/linear_leios.rs#L1174) vs [sim-rs/sim-core/src/sim/linear_leios.rs:2421-2485](sim-rs/sim-core/src/sim/linear_leios.rs#L2421)

**Issue:** `finish_validating_rb_header` already implements lower-VRF-wins slot-battle resolution at line 1174-1190 — checks `block_ids_by_slot`, dethrones old block if `old.vrf > new.vrf`, evicts incomplete_onchain_ebs. The new helpers `rb_slot_is_still_viable` (line 2421) and `adopt_rb_slot` (line 2447) implement nearly the same logic, called from `finish_validating_rb` at line 1299 and 1315. Two slot-battle resolvers running on different events (header validation vs body validation) is brittle:

- The header path inserts the block as `Pending`, then header arrival from another peer with a lower VRF could dethrone it — but the body for the first header might still be in flight, and `finish_validating_rb` for the body could now arrive against a different `block_ids_by_slot[slot]`.
- `rb_slot_is_still_viable` returns `false` if `old_header.vrf <= rb.header.vrf` AND `old_block_id != rb.header.id`. But if the header path already overwrote the slot with the new winner, the body for the *loser* would correctly bail out — OK. If the header path resolved in favour of the body's RB, viability check returns `true` and adoption proceeds — also OK. The duplication is correct as written, but is a maintenance liability and a future-bug surface.
- More concretely: `adopt_rb_slot` at line 2475-2480 removes the old RB from `praos.blocks` if dethroning — but `finish_validating_rb_header` at line 1183-1188 also performs this removal. Net effect is the same (the second removal is a no-op), but the dethrone-eviction side effect on `incomplete_onchain_ebs` is performed twice if both paths fire on the same dethrone event.

**Fix:** Factor slot-battle resolution into a single helper. The "header path resolves slot ownership; body path adopts whatever slot ownership header path decided" invariant should be enforced via shared code or a comment that explicitly justifies the duplication.

### WR-02: `rb_slot_is_still_viable` is non-monotone with `adopt_rb_slot` under header path interleaving

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:2430-2434](sim-rs/sim-core/src/sim/linear_leios.rs#L2430)

**Issue:** Both `rb_slot_is_still_viable` and `adopt_rb_slot` contain the snippet:

```rust
if old_block_id == rb.header.id {
    return !matches!(
        self.praos.blocks.get(&rb.header.id),
        Some(RankingBlockView::Received { .. })
    );
}
```

This says "if the slot is already owned by us and we are not yet Received, return true; if already Received, return false". But between the `viable` check (line 1299) and the `adopt` call (line 1315), `validate_rb_pricing_fields` may insert blocks into `self.praos.blocks` via `request_rb_parent` (line 1305) which calls `self.praos.blocks.entry(parent).or_insert(RankingBlockView::HeaderPending)`. That mutation is keyed on `parent`, not on `rb.header.id`, so it shouldn't affect the same-RB check — but the pattern of "check viability, then validate (with side effects), then adopt" is fragile if any of those side effects ever inserts at `rb.header.id`. Today it doesn't, but the asymmetric guards on `rb_slot_is_still_viable` (read-only) vs `adopt_rb_slot` (mutating) make future re-entrancy bugs likely.

**Fix:** Either (a) collapse the two helpers into one method that returns `(viable, adopt_result)` in a single call so no mutation can race between them, or (b) add a hard assertion in `adopt_rb_slot` that `rb_slot_is_still_viable(rb)` still returns true at entry.

### WR-03: Endorsement validity at production time refuses to endorse if producer's local sample doesn't match the certified sample, dropping the entire EB silently

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:770-773 and 793-798](sim-rs/sim-core/src/sim/linear_leios.rs#L770)

**Issue:** In `produce_rb`'s endorsement-construction closure:

```rust
if let Some(eb) = self.get_validated_eb(eb_id) {
    if self.samples_for_eb_body(&eb) != emitted_samples {
        return None;
    }
    // ...
} else {
    if let Some(eb) = self.get_received_eb(eb_id)
        && self.samples_for_eb_body(&eb) != emitted_samples
    {
        return None;
    }
    // ...
}
```

A producer with a validated (or merely received) EB body that disagrees with the certified samples refuses to endorse. The downstream effect: `(endorsement, mut emitted_samples) = (None, Vec::new())`, the RB is produced unendorsed, and a Praos-fallback body may be sampled if `praos_fallback` is set. There is **no event tracked** for this drop, no `track_no_vote` analog, no diagnostic. Operators investigating "why did this RB skip its endorsement" have no signal.

This is the same producer-side disagreement that the `eb_endorsement_valid` staleness check handles by also returning None silently (existing behaviour, not new). But adding *another* silent drop reason narrows the operator's ability to distinguish stale-tx-staleness from sample-disagreement.

**Fix:** Emit a structured event in both early-return arms — e.g. `self.tracker.track_endorsement_refused(EndorserBlockId, RefusalReason::SampleMismatch)`. The reason taxonomy makes M2/M3 endorsement-refusal regression tests both easier to write and easier to diagnose. Also worth a `tracing::warn!` at minimum to surface in dev logs.

### WR-04: New `eb_samples: BTreeMap<EndorserBlockId, Vec<PricedBlockSample>>` on VoteBundle is unused by `leios.rs` and `stracciatella.rs` (`BTreeMap::new()`) — silent on-chain incompatibility hazard

**File:** [sim-rs/sim-core/src/model.rs:425-431](sim-rs/sim-core/src/model.rs#L425), [sim-rs/sim-core/src/sim/leios.rs:684](sim-rs/sim-core/src/sim/leios.rs#L684), [sim-rs/sim-core/src/sim/stracciatella.rs:803](sim-rs/sim-core/src/sim/stracciatella.rs#L803)

**Issue:** The `eb_samples` field is added to `VoteBundle` at the model level, but only `linear_leios.rs` populates it (line 1738-1739). `leios.rs` and `stracciatella.rs` both ship empty `BTreeMap::new()`. The model.rs doc comment correctly notes "Other variants leave this empty."

The hazard: `count_votes` in `linear_leios.rs` at line 1857-1859 silently skips any vote whose `eb_samples` doesn't contain the eb_id:

```rust
let Some(samples) = votes.eb_samples.get(eb_id) else {
    continue;
};
```

So a linear-Leios node receiving a vote bundle from a node that somehow uses the `leios.rs` or `stracciatella.rs` codepath would silently discard those votes. Cross-variant interop is not exercised by the project's suites — but the silent `continue;` is brittle if anyone ever runs a mixed-variant simulation. Also: `VoteBundle.bytes` no longer includes `sample_bytes` for the non-linear variants (since they ship empty samples), which is correct, but should be documented or assert-checked.

**Fix:** Either (a) replace the silent `continue;` with `debug_assert!(votes.eb_samples.contains_key(eb_id), "linear-Leios votes must carry eb_samples")` so a developer who ever sees a vote without samples gets a panic in debug, or (b) emit a `tracing::warn!` and a tracker event so it's visible in logs.

### WR-05: `request_eb_from_voters` walks `votes.keys()` filtered by `consumers` — silently skips voters not directly peered

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:2392-2399](sim-rs/sim-core/src/sim/linear_leios.rs#L2392)

**Issue:**

```rust
let mut voter_peers: Vec<NodeId> =
    votes
        .keys()
        .copied()
        .filter(|node| self.consumers.contains(node))
        .collect();
```

This filters voter NodeIds to keep only those in `self.consumers`. On a 100-node mainnet-snapshot topology with a non-clique mesh, the average voter set is much larger than any single node's direct-peer set. Most voters will be filtered out; the function falls through to `if !voter_peers.is_empty()`. If empty:

```rust
} else {
    self.leios
        .ebs
        .entry(eb_id)
        .or_insert(EndorserBlockView::Pending);
}
```

— the EB is left in `Pending` and **only the `fallback_peer` (if set) is asked for it**. That's the originating peer of the message that triggered the request. If `fallback_peer` is None (as in the `produce_rb` path at line 805), no request goes out at all. The EB never gets pulled, the receiver sits forever in incomplete_onchain_ebs, and produce_rb refuses to include any RB-body txs (line 824 `produce_empty_block = !self.leios.incomplete_onchain_ebs.is_empty()`).

A producer at line 805 is calling `request_eb_from_voters(eb_id, &votes, None)` — fallback_peer = None. So if none of the voters are direct peers, the producer publishes an empty RB AND never asks anyone for the EB. The chain stalls (empty-block run) until something else triggers an EB request. The receiver path at line 1336 passes `from` as the fallback peer, so receive-RB at least guarantees one request goes out.

**Fix:** Either (a) drop the `consumers` filter (request from any voter — the network layer handles unreachable peers), or (b) ensure `request_eb_from_voters` always sends to at least one peer (e.g., a random direct peer) as a last-resort. Document the trade-off in a comment.

### WR-06: `validate_rb_pricing_fields` blocks chain progression on missing-parent without time-bounded recovery

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:2493-2502](sim-rs/sim-core/src/sim/linear_leios.rs#L2493), [sim-rs/sim-core/src/sim/linear_leios.rs:1304-1311](sim-rs/sim-core/src/sim/linear_leios.rs#L1304)

**Issue:** The parent-waiting queue `rbs_waiting_for_parent: HashMap<BlockId, Vec<(Option<NodeId>, Arc<RankingBlock>)>>` accumulates pending RBs whose parent has not yet been received. The retry is triggered only by `retry_rbs_waiting_for_parent` (called once at the end of `publish_rb`, line 1116). If the parent RB never arrives (peer offline, message dropped), the RBs accumulate indefinitely. No timeout, no max-size guard.

In addition: the queue stores `Arc<RankingBlock>`. Each RB carries `transactions: Vec<Arc<Transaction>>` and `emitted_samples: Vec<PricedBlockSample>`. A pathological pattern (Sybil flooding the node with deep-orphan-chain RBs) can grow this unboundedly. The existing `praos.blocks` map is similarly unbounded but at least it's keyed by `BlockId` so the same-block-from-different-peers case dedups; this new map keys by the parent and pushes to a `Vec`, so the same RB sent multiple times from multiple peers can accumulate N copies.

**Fix:** Add a per-parent cap (drop oldest, or reject newer when full) and a TTL (drop entries older than some `slot - threshold`). This is denial-of-service mitigation, but also robustness against legitimate network partitions.

## Info

### IN-01: `block_ids_by_slot` may now hold a `Pending` view but a body that has been dethroned by `adopt_rb_slot` — stale slot-to-block mapping reachable

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:2473-2484](sim-rs/sim-core/src/sim/linear_leios.rs#L2473)

When `adopt_rb_slot` dethrones the old block, it `remove`s from `praos.blocks` and overwrites `block_ids_by_slot[slot] = rb.header.id`. But it does not touch `peer_heads` or any data structures keyed by the old block id. If a peer announces the old block id again (re-gossip), the receive path will request it, the header path will skip it (lower-VRF check), and the body path is unaffected — so this is benign as written. Worth a doc-comment note that the dethrone path is partial-cleanup-by-design.

### IN-02: `eb_samples` field bytes count not symmetric: certificate on RB does not include sample bytes that votes carry

**File:** [sim-rs/sim-core/src/sim/linear_leios.rs:1739-1746](sim-rs/sim-core/src/sim/linear_leios.rs#L1739), [sim-rs/sim-core/src/sim/linear_leios.rs:810-815](sim-rs/sim-core/src/sim/linear_leios.rs#L810)

A voter's VoteBundle size includes `sample_bytes`. But the `Endorsement` constructed in `produce_rb` (line 810-815) computes `size_bytes = self.sim_config.sizes.cert(votes.len())` — a function of vote count only, not of sample size. The same samples then appear on the RB itself via `LinearRankingBlock::bytes()` (`+ self.emitted_samples.len() as u64 * PRICED_BLOCK_SAMPLE_SIZE_BYTES`). Net: a node pays the sample-byte propagation cost once at vote receipt and once again at RB receipt. This may be intentional (samples physically appear in both messages) — but worth confirming against the protocol spec and noting in a comment so future maintainers don't try to "fix" the apparent double-count.

### IN-03: `BlockKind` derives `Ord` on declaration order — load-bearing for `Vec<PricedBlockSample>` as BTreeMap key

**File:** [sim-rs/sim-core/src/tx_pricing/mod.rs:41-46](sim-rs/sim-core/src/tx_pricing/mod.rs#L41), [sim-rs/sim-core/src/tx_pricing/mod.rs:69-70](sim-rs/sim-core/src/tx_pricing/mod.rs#L69)

`BlockKind` is `(RankingBlock, EndorserBlock)`. Adding `Ord` means deriving from declaration order: `RankingBlock < EndorserBlock`. `PricedBlockSample` now derives `Hash, PartialOrd, Ord` for use as `BTreeMap` key in `votes_by_eb_sample`. The total ordering is `(block_kind, controller_lane, relevant_bytes, relevant_capacity)` lexicographic. Determinism is preserved. If anyone ever reorders the enum variants, the `BTreeMap` iteration order flips and golden hashes flip. Worth a `// load-bearing: order via declaration` comment near `enum BlockKind`.

### IN-04: Old comments referencing the deleted `block_samples` cache still exist in `single_lane.rs` doc-comment

**File:** [sim-rs/sim-core/src/tx_pricing/single_lane.rs:20-22](sim-rs/sim-core/src/tx_pricing/single_lane.rs#L20)

The new comment says "Each linear-Leios RB also carries its emitted pricing samples, so `ChainView::samples_in_block` is a direct block-field read. No separate backend-level cache is needed." — accurate. But searching the rest of the codebase reveals lingering references to the deleted `block_samples` cache (e.g., line 902 inside `produce_rb`'s doc-comment block — let me verify after the diff actually lands). At a minimum, the `CLAUDE.md`'s description of the `block_samples` cache pruning is now stale (the cache no longer exists). Worth a CLAUDE.md update in the same staging.

---

## Cross-cutting observations (not findings)

- **Plan-scope drift.** The staged diff is 9 files / 760 lines of net change. `PLAN.md::files_modified` lists exactly one source file. The diff implements a substantive design change ("samples live on the block; votes carry per-EB samples") that is not described in `PLAN.md` or `TEST-IMPACT.md`. The phase directory does not contain a `SUMMARY.md` (Task 4 of the plan). I cannot tell whether this diff is intended as part of this quick task or whether it was layered on as scope creep — but it should not be committed under the existing `PLAN.md` without an explicit plan update.
- **Numeric-representation contract: respected.** I traced every new field and helper. `emitted_samples` is `Vec<PricedBlockSample>` with all-integer fields; the validation path is u64/u128 throughout; no f64 enters any hot path. `samples_for_eb_body` / `samples_for_rb_body` route through the same `PricingBackend::samples_for_block` seam that already respected the contract. Good.
- **Pure-function `PricingBackend` contract: respected.** `compute_derived_quote` signature is unchanged. `TwoLanePricing` and `Eip1559Pricing` add no new state. The `two_lane.rs` diff is overwhelmingly rustfmt with one substantive rename inside `effective_window_length`'s doc comment.
- **`current_chain_tip_quote` quick-fix invariant: preserved.** I confirmed `current_chain_tip_quote` still reads `tip.derived_quote.get(lane)` directly. The staged diff does not regress the WR-1 fix at the consumer-quote path. CR-01's concern is upstream of that read: it's how `tip.derived_quote` gets *populated* on un-verifiable endorsements.
- **M2/M3 unit-test golden-hash mechanism: preserved.** No edits to `m3_actors.rs`. The `m2_two_lane.rs` edit inverts an assertion (`assert_ne!` → `assert_eq!`) but does not touch any `GOLDEN_*` constant, the `UPDATE_GOLDENS=1` regeneration mechanism, or the set of events hashed.

---

_Reviewed: 2026-05-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
