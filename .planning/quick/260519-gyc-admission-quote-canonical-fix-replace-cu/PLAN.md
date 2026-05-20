---
plan_id: 260519-gyc-admission-quote-canonical-fix-replace-cu
type: quick
autonomous: true
files_modified:
  - sim-rs/sim-core/src/sim/linear_leios.rs
  - sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml   # touched + reverted
  - .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/TEST-IMPACT.md
  - .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md
---

<objective>
Surgical fix to `current_chain_tip_quote` so admission / eviction / EB-endorsement / EB-inclusion gates read the canonical tip quote (`tip.derived_quote.get(lane)`) directly off the chain-tip `RankingBlock`, not a hypothetical-child-of-tip quote computed from the node-local mutable `block_samples` cache. Validate the fix with a build, a unit-test characterisation pass, and a 3-seed smoke of `phase-3-canonical-variance.yaml`. Compare per-(job, seed) `retained_value` deltas vs control against the existing N=20 Bias-corrected and accelerated (BCa) bootstrap Confidence Intervals (CIs) to decide whether the fix preserves the TEST-04 verdicts before a citable git tag goes live.

Purpose: protocol-soundness — the quote a user signs against (`max_fee_lovelace`) must equal the quote the network uses to evaluate that transaction. The legacy `compute_chain_derived_quote_for_child_of(tip)` path reads `self.block_samples`, which mutates when deferred Endorser Blocks (EBs) validate, producing per-node divergence at the same canonical chain tip.

Output: a code fix on `linear_leios.rs`, build/test status notes, smoke results, and a `SUMMARY.md` verdict + recommendation in the task dir.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@CLAUDE.md
@sim-rs/sim-core/src/sim/linear_leios.rs
@sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml
@cip-evidence/test-results/multi-seed-variance/results.md
@cip-evidence/test-results/multi-seed-variance/canonical/menu_rb_reserved_priority_only_static_x4.json
@cip-evidence/test-results/multi-seed-variance/canonical/menu_unreserved_priority_only_static_x4.json
@cip-evidence/test-results/multi-seed-variance/canonical/menu_rb_reserved_both_dynamic_x4.json
@cip-evidence/test-results/multi-seed-variance/canonical/menu_unreserved_both_dynamic_x4.json

<interfaces>
Relevant identifiers already in `sim-rs/sim-core/src/sim/linear_leios.rs`:

- `fn latest_rb_id(&self) -> Option<BlockId>` (line ~1316) — canonical chain tip id.
- `self.praos.blocks.get(&id)` — chain view lookup; returns a `BlockView`-like wrapper exposing `.received_rb() -> Option<&Arc<RankingBlock>>`.
- `RankingBlock.derived_quote: PerLaneQuote` — canonical per-block quote, immutable, all nodes agree.
- `PerLaneQuote { standard: u64, priority: u64 }` with `.get(lane: Lane) -> u64` accessor (see line 26 import and call sites at 2354 / 2502 / 2554).
- `self.pricing.cold_start_quote(lane: Lane) -> u64` — backend fallback for the genesis path.
- Existing precedent for the canonical access pattern (line ~2289, `current_chain_tip_aggregate`):
  `self.latest_rb_id().and_then(|id| self.praos.blocks.get(&id)).and_then(|view| view.received_rb()).map(|rb| rb.<field>).unwrap_or(<cold-start fallback>)`
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Replace current_chain_tip_quote internals with canonical-tip read; build + check; atomic commit</name>
  <files>sim-rs/sim-core/src/sim/linear_leios.rs</files>
  <action>
    Edit ONLY the body of `fn current_chain_tip_quote(&self, lane: Lane) -> u64` (currently at line 2278) and its preceding doc comment (lines ~2260-2277). Function signature is unchanged. No consumer call site is changed.

    New body semantics: return `tip.derived_quote.get(lane)` for the canonical chain tip RB; if there is no canonical RB yet (genesis path), return `self.pricing.cold_start_quote(lane)`. Mirror the access pattern already used by `current_chain_tip_aggregate` at line 2289:

      self.latest_rb_id()
          .and_then(|id| self.praos.blocks.get(&id))
          .and_then(|view| view.received_rb())
          .map(|rb| rb.derived_quote.get(lane))
          .unwrap_or_else(|| self.pricing.cold_start_quote(lane))

    Replace the doc comment. The current comment justifies the hypothetical-child semantic by appealing to legacy accumulator parity; that justification is exactly wrong post-spike-007. The new comment must state plainly: returns `tip.derived_quote.get(lane)` for the canonical chain tip (the same value every node sees once the tip's RB header is on chain); cold-start fallback at genesis. Note that RB body inclusion charging uses the new RB's own `rb.derived_quote` directly (unchanged, see `produce_rb` near line 2502), so producer-side admission and consumer-side validation agree on the canonical-tip-quote everywhere.

    Do NOT touch the four consumer call-site clusters (lines 955-956, 1862, 2002-2003, 2093-2094, 2354). Do NOT touch `compute_chain_derived_quote_for_child_of` — it stays in place because the production path at line 893 still needs it to compute the new RB's `derived_quote` from the parent's samples. Do NOT touch `current_chain_tip_quote_for_test` (line 2516) — it remains a thin wrapper.

    Validate and commit:
      cd sim-rs && cargo check --workspace
      cd sim-rs && cargo build --release
      git add sim-rs/sim-core/src/sim/linear_leios.rs
      git commit -m "fix(controller): current_chain_tip_quote reads canonical tip derived_quote directly"

    Commit body should note: why (per-node block_samples cache divergence on deferred-EB validate violates EIP-1559 protocol fidelity); what changed (function body + doc comment only); scope (no consumer call site changes; RB body charging path unchanged). Do NOT tag.
  </action>
  <verify>
    <automated>cd sim-rs &amp;&amp; cargo check --workspace 2&gt;&amp;1 | tail -5 &amp;&amp; cargo build --release 2&gt;&amp;1 | tail -5 &amp;&amp; git log -1 --format=%s | grep -q "fix(controller): current_chain_tip_quote reads canonical tip derived_quote directly"</automated>
  </verify>
  <done>
    `current_chain_tip_quote` body reads `tip.derived_quote.get(lane)` with cold-start fallback; doc comment rewritten (no "legacy accumulator semantics" justification). `cargo check --workspace` exit 0. `cargo build --release` exit 0. Single commit on HEAD with subject `fix(controller): current_chain_tip_quote reads canonical tip derived_quote directly`. No edits to any consumer call site, no edits to `compute_chain_derived_quote_for_child_of`, no edits to `current_chain_tip_quote_for_test`.
  </done>
</task>

<task type="auto">
  <name>Task 2: Characterise unit-test impact via cargo test --workspace --lib; write TEST-IMPACT.md; commit</name>
  <files>.planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/TEST-IMPACT.md</files>
  <action>
    Run the workspace lib tests and capture which tests flipped:
      cd sim-rs && cargo test --workspace --lib 2>&1 | tee /tmp/quick-fix-cargo-test.log

    Parse the log for failing tests (status `FAILED`) and assertion-mismatch lines that mention pinned golden hashes. Expected failure surface: the M2 and M3 unit-test goldens at `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` and `sim-rs/sim-core/src/sim/tests/m3_actors.rs` — these constants are sensitive to the consumer-quote semantic on `current_chain_tip_quote` and are documented in `CLAUDE.md` §"Determinism scope" as the M2/M3 unit-test goldens layer.

    Write `.planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/TEST-IMPACT.md` with:
      - Header summarising the cargo test summary line (passed / failed / ignored counts).
      - Bulleted list of every failing test: test name, file path, and one-line classification — `golden-hash mismatch (M2/M3 expected)` OR `unexpected logic failure (investigate)`.
      - Closing `Verdict:` line, exactly one of:
          `Verdict: EXPECTED — only M2/M3 pinned-golden-hash flips`
          `Verdict: UNEXPECTED — <comma-separated list of non-golden failures>`

    Do NOT regenerate goldens. Do NOT edit `m2_two_lane.rs` or `m3_actors.rs`. Goldens regeneration is explicitly out of scope for this quick task.

    Commit:
      git add .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/TEST-IMPACT.md
      git commit -m "docs(quick-fix): record unit-test golden-hash flips from current_chain_tip_quote fix"

    If the verdict is UNEXPECTED, halt — surface the failures in the commit body and do NOT proceed to Task 3 without operator review.
  </action>
  <verify>
    <automated>test -f .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/TEST-IMPACT.md &amp;&amp; grep -q "^Verdict:" .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/TEST-IMPACT.md &amp;&amp; git log -1 --format=%s | grep -q "docs(quick-fix): record unit-test golden-hash flips"</automated>
  </verify>
  <done>
    `TEST-IMPACT.md` exists at the task dir with a `Verdict:` line that is either `EXPECTED — only M2/M3 pinned-golden-hash flips` (proceed to Task 3) or `UNEXPECTED — ...` (halt). Atomic commit on HEAD with subject `docs(quick-fix): record unit-test golden-hash flips from current_chain_tip_quote fix`. No goldens regenerated; `m2_two_lane.rs` and `m3_actors.rs` untouched.
  </done>
</task>

<task type="auto">
  <name>Task 3: Run 3-seed smoke of phase-3-canonical-variance.yaml; revert YAML; no commit</name>
  <files>sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml</files>
  <action>
    Temporarily restrict the suite to seeds 1, 2, 3 for a smoke run, execute it against a fresh output directory so the existing N=20 artefacts at `sim-rs/output/phase-3/canonical-variance/` are not contaminated, then restore the YAML so `git diff` is clean.

    Step 3a — Edit the YAML in place:
      - Replace `seeds: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]` with `seeds: [1, 2, 3]`.
      - Replace `output-dir: output/phase-3/canonical-variance` with `output-dir: output/phase-3/canonical-variance-smoke-quick-fix`.
      - Do NOT commit the modified YAML.

    Step 3b — Run the suite. Wall-clock budget ~30 minutes; abort and document in SUMMARY.md if it exceeds 45 minutes without all 15 (5 jobs × 3 seeds) pairs completing:
      cd sim-rs && cargo run --release --bin experiment-suite -- run \
          parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml \
          --parallelism 8

    Step 3c — Verify the 5 expected job names all appear in `sim-rs/output/phase-3/canonical-variance-smoke-quick-fix/metrics_comparison.txt`: `control_eip1559_d8_t50_w32`, `menu_unreserved_priority_only_static_x4`, `menu_rb_reserved_priority_only_static_x4`, `menu_unreserved_both_dynamic_x4`, `menu_rb_reserved_both_dynamic_x4`.

    Step 3d — Restore the YAML to its original `seeds: [1..20]` and `output-dir: output/phase-3/canonical-variance` and confirm `git diff sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml` is empty.

    Do NOT commit anything in this task — the YAML edit is intentionally non-persistent and the smoke-output dir is a transient artefact that Task 4 reads.
  </action>
  <verify>
    <automated>test -f sim-rs/output/phase-3/canonical-variance-smoke-quick-fix/metrics_comparison.txt &amp;&amp; for job in control_eip1559_d8_t50_w32 menu_unreserved_priority_only_static_x4 menu_rb_reserved_priority_only_static_x4 menu_unreserved_both_dynamic_x4 menu_rb_reserved_both_dynamic_x4; do grep -q "$job" sim-rs/output/phase-3/canonical-variance-smoke-quick-fix/metrics_comparison.txt || exit 1; done &amp;&amp; test -z "$(git diff sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml)"</automated>
  </verify>
  <done>
    `sim-rs/output/phase-3/canonical-variance-smoke-quick-fix/metrics_comparison.txt` exists and contains all 5 expected job names. All 15 (job, seed) pairs reflected in the suite manifest as `Completed`. `git diff` on `phase-3-canonical-variance.yaml` is empty (seeds and output-dir restored). No new commit on HEAD.
  </done>
</task>

<task type="auto">
  <name>Task 4: Compute per-(job, seed) retained_value deltas; compare against N=20 BCa CIs; write SUMMARY.md with verdict + recommendation; commit SUMMARY.md</name>
  <files>.planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md</files>
  <action>
    Parse the new smoke run's metrics and produce the verdict + recommendation memo.

    Step 4a — Extract per-(job, seed) `retained_value` from `sim-rs/output/phase-3/canonical-variance-smoke-quick-fix/`. The canonical source is the per-(job, seed) `run-summary.json` files at `sim-rs/output/phase-3/canonical-variance-smoke-quick-fix/<job_name>/<seed>/`. `retained_value` is a top-level `f64` reporting metric on `RunSummary`. Also extract `priority_lane_retained_value_ratio` and `standard_lane_retained_value_ratio` (per-cell aggregates) from `metrics_comparison.txt` for the same 5 jobs.

    Step 4b — Compute paired deltas. For each menu job and each of seeds 1, 2, 3:
      delta(menu_job, seed) = retained_value(menu_job, seed) − retained_value(control_eip1559_d8_t50_w32, seed)

    There are 4 menu jobs × 3 seeds = 12 deltas total.

    Step 4c — Load the existing N=20 BCa CIs from `cip-evidence/test-results/multi-seed-variance/results.md` §"TEST-04 canonical menu-item variance bands" and from the per-job JSONs under `cip-evidence/test-results/multi-seed-variance/canonical/menu_*.json` (each file's BCa CI for `retained_value_delta_vs_control`).

    Step 4d — Decide three checks:
      (a) Containment: do all 12 smoke deltas fall inside the existing N=20 BCa 95% CI for their respective menu arm? Report containment per (menu_job, seed).
      (b) Sign ordering: do the un-reserved arms remain positive and the RB-reserved arms remain negative across the 3 smoke seeds, matching the canonical TEST-04 qualitative finding?
      (c) Cross-arm equivalence: does the rough equivalence of RB-reserved priority-only vs RB-reserved both-dynamic (partitioned) survive at N=3?

    Step 4e — Write `.planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md` with this structure (use markdown; abbreviations expanded on first use per CLAUDE.md):

      # Quick fix: current_chain_tip_quote reads canonical tip derived_quote — verification

      ## Code diff summary
      - File: sim-rs/sim-core/src/sim/linear_leios.rs (function body + doc comment only)
      - Before: `compute_chain_derived_quote_for_child_of(tip)` (reads node-local mutable block_samples cache)
      - After: `tip.derived_quote.get(lane)` from canonical chain tip, cold-start fallback at genesis
      - Consumer call sites: unchanged (admission 1862, eviction 2002, EB endorsement 955-956, EB charging 2093-2094 / 2354)

      ## Build status
      - cargo check --workspace: <PASS/FAIL>
      - cargo build --release: <PASS/FAIL>

      ## Unit-test status
      - Summary line: passed=<n> failed=<n> ignored=<n>
      - Flipped tests: <list, with file paths>
      - Classification: <all-M2/M3-golden OR includes-non-golden>
      - Goldens regenerated: NO (out of scope; deferred to follow-up task)

      ## Smoke run (3 seeds × 5 jobs)
      - Output dir: sim-rs/output/phase-3/canonical-variance-smoke-quick-fix/
      - Wall-clock: <observed minutes>
      - Per-(job, seed) retained_value table:
        | job | seed=1 | seed=2 | seed=3 |
        | --- | --- | --- | --- |
        | control_eip1559_d8_t50_w32 | ... | ... | ... |
        | menu_unreserved_priority_only_static_x4 | ... | ... | ... |
        | menu_rb_reserved_priority_only_static_x4 | ... | ... | ... |
        | menu_unreserved_both_dynamic_x4 | ... | ... | ... |
        | menu_rb_reserved_both_dynamic_x4 | ... | ... | ... |
      - Per-(menu_job, seed) delta vs control (`menu − control`):
        <4 × 3 table of deltas>

      ## Comparison against existing N=20 BCa Confidence Intervals (CIs)
      - For each menu arm, list: N=20 CI low, N=20 CI high, N=3 smoke deltas (seed 1, 2, 3), and per-seed containment (inside / outside CI).
      - (a) Containment verdict: <all 12 inside / X outside>
      - (b) Sign-ordering verdict: <preserved / not preserved>
      - (c) Cross-arm-equivalence verdict: <preserved / not preserved / inconclusive at N=3>

      ## Overall verdict
      One of:
        - SURVIVES: all three checks pass → fix preserves TEST-04 qualitative findings under N=3 smoke; safe to schedule full N=20 re-run.
        - DOES NOT SURVIVE: at least one check fails with high confidence (e.g. multiple seeds outside CI, sign flip) → fix changes mechanism welfare; re-investigate before tagging.
        - INCONCLUSIVE: smoke is consistent with the existing CIs but N=3 cannot rule out drift inside the CI band → full N=20 re-run required to decide.

      ## Recommendation
      One of:
        - FULL N=20 RE-RUN of `phase-3-canonical-variance.yaml` to produce the citable-tag-grade numbers; then regenerate M2/M3 unit-test goldens; then tag.
        - NO-GO: investigate <specific failure>; do not tag.
        - RE-INVESTIGATE THE FIX: <specific concern>; do not regenerate goldens or tag.

    Commit:
      git add .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md
      git commit -m "docs(quick-fix): SUMMARY.md verdict for current_chain_tip_quote canonical-tip fix"

    Do NOT tag. Do NOT regenerate goldens. Do NOT edit any document under `cip-evidence/` (audit-document edits are out of scope per the task context).
  </action>
  <verify>
    <automated>test -f .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md &amp;&amp; grep -q "^## Overall verdict" .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md &amp;&amp; grep -q "^## Recommendation" .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md &amp;&amp; grep -Eq "SURVIVES|DOES NOT SURVIVE|INCONCLUSIVE" .planning/quick/260519-gyc-admission-quote-canonical-fix-replace-cu/SUMMARY.md &amp;&amp; git log -1 --format=%s | grep -q "docs(quick-fix): SUMMARY.md verdict"</automated>
  </verify>
  <done>
    `SUMMARY.md` exists with sections: Code diff summary, Build status, Unit-test status, Smoke run (with the per-(job, seed) retained_value table and the 4 × 3 delta table), Comparison against existing N=20 BCa CIs (with per-arm containment), Overall verdict (one of SURVIVES / DOES NOT SURVIVE / INCONCLUSIVE), Recommendation (full N=20 re-run / no-go / re-investigate). Atomic commit on HEAD with subject `docs(quick-fix): SUMMARY.md verdict for current_chain_tip_quote canonical-tip fix`. No tag, no goldens regenerated, no `cip-evidence/` edits.
  </done>
</task>

</tasks>

<success_criteria>
The quick task is complete when:

- `current_chain_tip_quote` returns `tip.derived_quote.get(lane)` from the canonical chain tip RB (with cold-start fallback), and its doc comment is rewritten accordingly. No consumer call site is changed.
- `cargo check --workspace` and `cargo build --release` both succeed on the fix commit.
- `TEST-IMPACT.md` records the unit-test impact with a `Verdict:` line; the only flips are M2/M3 pinned-golden-hash assertions (no unexpected logic failures).
- 3-seed smoke of `phase-3-canonical-variance.yaml` completed against the smoke-only output directory; the 5 expected job names all appear in its `metrics_comparison.txt`; the suite YAML is restored to its original 20-seed / canonical-output-dir state (`git diff` empty).
- `SUMMARY.md` contains the code-diff summary, build status, unit-test status, per-(job, seed) `retained_value` table and 4 × 3 delta table, comparison against the existing N=20 BCa CIs (with per-arm containment and the sign-ordering / cross-arm-equivalence checks), an Overall verdict line (SURVIVES / DOES NOT SURVIVE / INCONCLUSIVE), and a Recommendation (full N=20 re-run / no-go / re-investigate the fix).
- Three atomic commits on HEAD: the code fix, the TEST-IMPACT.md, and the SUMMARY.md. No tag applied. No goldens regenerated. No `cip-evidence/` edits. No CLAUDE.md edits.
</success_criteria>
