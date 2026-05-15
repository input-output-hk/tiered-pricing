# PLAN — Realism + docs fix (multi-producer 100-node with realistic stake curve)

Date: 2026-05-13
Goal: Replace `topology.default.yaml`'s uniform `stake=100` curve with a Cardano-mainnet-faithful mass-stratified downsample (spike 006, Option 1), point all 19 phase-2 suites at the new file, regenerate the 7 M5 suite-level goldens, and propagate documentation corrections so CLAUDE.md, the realism audit, REVIEW.md, and validity-threats.md reflect the corrected multi-producer reality.

Scope:
- New file `sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml` (same 100-node structure as `topology.default.yaml`; only `stake` values change).
- New optional helper script `sim-rs/scripts/generate-realistic-100-topology.py` (mainnet on-chain pull + stratified-downsample). Optional in the sense that the YAML itself is checked in; the script documents reproducibility.
- 19 suite YAMLs under `sim-rs/parameters/phase-2-sweep/suites/` — single-line `default-topology` field re-pointed.
- 7 M5 suite-level goldens under `sim-rs/parameters/phase-2-sweep/suites/.goldens/*.sha256` — regenerated.
- 4 documentation files updated: `CLAUDE.md`, `docs/phase-2/cardano-realism-audit.md`, `.planning/spikes/004-topology-and-actor-model/README.md`, `.planning/REVIEW.md`, `docs/phase-2/validity-threats.md`.

Out of scope:
- Implementing WR-1 (pricing-state rollback on slot-battle reorg). Defer to M6.
- Switching to `topology-cip-realistic.yaml`. Spike 006 explicitly picks the 100-node middle ground.
- Modifying `sim-rs/parameters/topology.default.yaml`. Upstream main consumes it; do NOT touch.
- Modifying any Rust source code under `sim-rs/sim-core/` or `sim-rs/sim-cli/`. YAML + Markdown + Python helper only.
- Rewriting the 7-suite vs 19-suite table in CLAUDE.md beyond a small annotation. Cosmetic; flag but defer.
- Re-running phase-2 experimental suites with the new topology (separate compute investment; the team plans that downstream).
- Committing or tagging. Per `feedback_no_commits.md`, leave staged/unstaged changes for the user.

## Goal-backward verification

Done means:
1. `sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml` exists, parses as YAML, has exactly 100 nodes named `node-0..node-99`, preserves locations/latencies/producers/bandwidth from `topology.default.yaml`, and `stake` values match spike 006 Option 1's mass-stratified downsample (sorted descending, node-0 largest, total = 3 × 10^10 lovelace, min × 0.05 ≥ 100).
2. `grep -l "parameters/topology.default.yaml" sim-rs/parameters/phase-2-sweep/suites/*.yaml` returns empty. `grep -c "parameters/phase-2-sweep/topology-realistic-100.yaml" sim-rs/parameters/phase-2-sweep/suites/*.yaml` returns 1 for each of the 19 suite files.
3. `cd sim-rs && cargo build --release` succeeds. `cd sim-rs && cargo test --workspace` passes (124 sim-core unit tests + 16 sim-cli + 4 + 1 — exact counts may drift; the assertion is "no test fails").
4. The 7 `.goldens/*.sha256` files have been regenerated against the new topology, and a second `cargo test --release -- --ignored determinism` run (without `UPDATE_GOLDENS=1`) passes — proving the regen was clean and reproducible intra-arch.
5. CLAUDE.md no longer asserts `topology-single-producer.yaml` as the operational topology; instead it documents `topology-realistic-100.yaml` as the suite default, with `default-slots = 2000` corrected, and a new "Topology choice" entry under Calibration choices.
6. Spike 004 README and `cardano-realism-audit.md` carry a clearly-marked header annotation indicating the topology audited is no longer in use; original audit content is preserved.
7. REVIEW.md's WR-1 Fix Status row reads "LIVE / disclosure-required" rather than "Deferred to M6 / doesn't fire under current single-producer suites."
8. `docs/phase-2/validity-threats.md` carries a "Resolved 2026-05-13" note at top confirming the topology gap has been closed.
9. `git status --short` shows the expected modified/added files. No commits made.

Verification commands the executor will run at the end:
- `cd sim-rs && cargo build --release`
- `cd sim-rs && cargo test --workspace`
- `cd sim-rs && cargo test --release -- --ignored determinism`
- `grep -l "parameters/topology.default.yaml" sim-rs/parameters/phase-2-sweep/suites/*.yaml` (must produce no output)
- `git status --short` (review only)

## Task breakdown

### Task 1: Generate `topology-realistic-100.yaml`
- **Method:** spike 006 §"Implementation plan" — stratified mass-downsample of the 1,510 mainnet active-stake pools (≥ 1k ADA filter), bisect-on-cumsum at `(i + 0.5) / 100 × total_mass` for `i ∈ [0, 100)`, sort descending, rescale linearly so the sum = 3 × 10^10 lovelace, pin residual on the smallest pool.
- **Source:** Cardano mainnet on-chain state, epoch 582, retrieved 2026-05-14 (spike 006 §Sources). The executor MUST re-pull and confirm the sampled stake values match spike 006 §"Option 1" within a tolerance band (any individual stake within ± 5 % of the spike's value). If drift exceeds tolerance, the executor cites the new retrieval date in the YAML header and surfaces the delta to the user before continuing.
- **Output:** `sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml`. **DO NOT** create or modify `sim-rs/parameters/topology.default.yaml`.
- **Structure:** byte-for-byte derived from `sim-rs/parameters/topology.default.yaml` — copy the 100 `node-N` entries verbatim (preserving `location`, `producers`, `cpu-core-count: null`, all `latency-ms` and `bandwidth-bytes-per-second` values), then overwrite each node's `stake` field. Assign stakes in descending order: node-0 receives the largest sampled stake, node-99 the smallest. Mirrors `topology-cip-realistic.yaml`'s pool-000-largest convention.
- **`tx-generation-weight`:** if `topology.default.yaml` carries one or more `tx-generation-weight` lines, preserve them at the same node positions. If none, assign `tx-generation-weight: 1` to `node-0` (the largest-stake node) — matches `topology-cip-realistic.yaml`.
- **Lottery-quantization check:** `min(stake) × 0.05 ≥ 100`. Spike 006 reports min × 0.05 ≈ 197,160 (passes by ~3 orders of magnitude). Executor confirms via grep on the generated file.
- **Header comment** (mandatory, mirror `topology-cip-realistic.yaml` lines 1–18): on-chain query URL, epoch 582, retrieval date, "DO NOT HAND-EDIT — regenerated via `sim-rs/scripts/generate-realistic-100-topology.py` from a date-stamped mainnet snapshot." Reference spike 006's defensibility statement verbatim.
- **Helper script** (`sim-rs/scripts/generate-realistic-100-topology.py`): not strictly required to land the YAML, but recommended for auditability. Outline per spike 006 §"Implementation plan": fetch → filter ≥ 1k ADA → cumsum → bisect-sample 100 → rescale → emit YAML. Pin a Python random seed only if any tie-breaking is needed; the bisect-on-cumsum sampling rule is deterministic given the same on-chain snapshot. If the executor prefers to hand-construct the YAML from the spike 006 table directly (no script needed), they MUST cite spike 006 in the header.
- **Dependencies:** none.
- **Verification:**
  - `head -25 sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml` shows the documented header comment with mainnet retrieval date + spike 006 citation.
  - `grep -c "^  node-" sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml` returns 100.
  - `grep -E "^    stake: " sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml | wc -l` returns 100.
  - The sum of stake values equals 3 × 10^10 (within 1 lovelace from the residual pin).
  - `min(stakes) × 0.05 ≥ 100` (single arithmetic check).
  - One-shot smoke run: `cd sim-rs && cargo run --release --bin experiment-suite -- run parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml` does not panic during config load (Task 2 must have completed for the suite to pick up the new path; alternatively, hand-edit one suite first for this smoke check).
- **Owner notes:** YAML body comes from `topology.default.yaml`'s structure; only `stake` is parameterised by spike 006's table. If the locations/latencies on disk differ from spike 006's mental model, **the on-disk file is ground truth** — do not normalise them.

### Task 2: Re-point the 19 suite YAMLs
- **Edit pattern:** in each YAML, locate the `default-topology:` line. Expected before-state across all 19: `default-topology: parameters/topology.default.yaml`. Replace with: `default-topology: parameters/phase-2-sweep/topology-realistic-100.yaml`.
- **Per-file line numbers** (current as of 2026-05-13; executor should re-confirm via `grep -n default-topology` before editing):
  - Line 5: `phase-2-eip1559-robustness.yaml`, `phase-2-eip1559-smoothing.yaml`, `phase-2-priority-only-rb-reserved.yaml`, `phase-2-priority-only-unreserved.yaml`, `phase-2-rb-scarcity.yaml`, `phase-2-two-lane-both-dynamic.yaml`, `phase-2-urgency-inversion.yaml`
  - Line 9: `phase-2-congested-singlelane.yaml`, `phase-2-moderate-singlelane.yaml`, `phase-2-realistic-singlelane.yaml`, `phase-2-sundaeswap-singlelane.yaml`
  - Line 10: `phase-2-congested-both-dynamic.yaml`, `phase-2-congested-priority-only.yaml`, `phase-2-moderate-both-dynamic.yaml`, `phase-2-moderate-priority-only.yaml`, `phase-2-realistic-both-dynamic.yaml`, `phase-2-realistic-priority-only.yaml`, `phase-2-sundaeswap-both-dynamic.yaml`, `phase-2-sundaeswap-priority-only.yaml`
- **Idiosyncrasy escape hatch:** if any suite's `default-topology` line does NOT match the expected before-state (e.g. it's commented, multi-line, or already pointing elsewhere), the executor MUST surface the file name + actual line content to the user before proceeding. Do not silently force-replace.
- **Dependencies:** Task 1 (the new YAML must exist on disk before the suite re-pointing has any effect at runtime; the literal edit can occur in any order with Task 1 since suites only read the file at run-time, but for clean staging, do Task 1 first).
- **Verification:**
  - `grep -l "parameters/topology.default.yaml" sim-rs/parameters/phase-2-sweep/suites/*.yaml` produces no output.
  - `grep -l "parameters/phase-2-sweep/topology-realistic-100.yaml" sim-rs/parameters/phase-2-sweep/suites/*.yaml | wc -l` returns 19.
  - `cargo build --release` succeeds (YAML schema-validated at deserialization time).

### Task 3: Update CLAUDE.md
- **Edit 1 — Repository layout (around line 50):** the line `topology-single-producer.yaml  # one-node topology (every slot wins the RB lottery)` is still accurate as a file-listing entry (the file may still exist on disk), but add an adjacent line below it for the new file:
  ```
  ├── topology-realistic-100.yaml      # 100-node, mass-stratified mainnet curve — phase-2 suites' default since 2026-05-13
  ```
  Do NOT delete the `topology-single-producer.yaml` listing — the file still exists and may still be used for kernel-correctness unit tests.
- **Edit 2 — Calibration choices, `rb-generation-probability = 0.05` entry (around line 252):** the current text claims `default-slots = 1000` and ties single-producer's `stake: 100000` to the rb-prob calibration. Replace the offending sentences:
  - Update `default-slots = 1000` → `default-slots = 2000` (matches the actual on-disk suite slot count per validity-threats.md).
  - Remove the sentence claiming the suites use `topology-single-producer.yaml` and `stake: 100000`. Replace with: "Phase-2 suites use `topology-realistic-100.yaml` (100 nodes, mainnet-snapshot mass-stratified stakes, rescaled to total = 3 × 10^10 lovelace). The minimum stake in that curve clears the lottery-quantization check (min × rb-prob ≥ 100) by three orders of magnitude."
  - Retain the existing forward-pointer to `calibration-fix-postmortem.md` (the rb-prob bug-vs-choice distinction is independent of topology choice).
- **Edit 3 — Add new entry under Calibration choices: "Topology choice."**
  - Body text (paste-ready):
    > **Topology = `parameters/phase-2-sweep/topology-realistic-100.yaml`.** 100 nodes; same locations/latencies/producers/bandwidth as upstream `parameters/topology.default.yaml`; stake values are a mass-stratified downsample of the 1,510 Cardano mainnet pools with ≥ 1k ADA active stake (Cardano mainnet on-chain state, epoch 582, retrieved 2026-05-14), rescaled linearly to total = 3 × 10^10 lovelace. Top-1 stake share = 1.97 %; Nakamoto coefficient = 35; Gini = 0.253. See `.planning/spikes/006-curve-design/README.md` for the curve-design rationale and `topology-realistic-100.yaml`'s header comment for reproduction recipe. *Re-calibrating*: re-run the on-chain query at a later epoch and regenerate via `sim-rs/scripts/generate-realistic-100-topology.py`; the M5 suite goldens flip and require `UPDATE_GOLDENS=1` re-pinning.
- **Edit 4 — "Single-producer ≠ single mempool" gotcha (around line 358):** the gotcha's core point (every node has its own mempool) is still true, but the parenthetical about `topology-single-producer.yaml` being "the only topology where N=1 so producer/source/mempool counts coincide" is now misleading since the operational suites do not use single-producer. Rewrite the gotcha header and first sentence:
  - Old: "Single-producer ≠ single mempool; one tx source ≠ one mempool"
  - New: "Multi-producer topology, per-node mempool. Every node has its own mempool — admission/eviction/inclusion run per-node, gossip distributes txs across the network. The suite default `topology-realistic-100.yaml` has 100 producers and 100 mempools; in any earlier `topology-single-producer.yaml`-based test, the producer/source/mempool counts happen to coincide at N=1, but this is the special case, not the default."
- **Edit 5 — Annotate the 7-suite table (around the "The 7 suites:" heading):** add a one-line callout above the table:
  > Note: the table below covers the 7 M3/M4 mechanism-characterisation suites pinned by M5 suite-level goldens. The full suite directory holds 19 YAMLs (the 7 listed here plus 12 demand-regime suites under `paper_like_*` and `sundaeswap_*` profiles). The 12 demand-regime suites are not goldens-pinned.
  Do NOT expand the table to 19 rows in this plan — purely cosmetic, deferred.
- **Dependencies:** Task 1 (the new YAML must exist for the documentation to reference it concretely).
- **Verification:** `grep -c "topology-realistic-100" CLAUDE.md` returns ≥ 4 (one in repo-layout, one in calibration entry, one in topology-choice entry, one in gotcha). `grep -c "default-slots = 1000" CLAUDE.md` returns 0 (the stale value is gone).

### Task 4: Annotate spike 004 README
- **Edit:** prepend a header note immediately after the existing `Date: 2026-05-13` / `Verdict: NEEDS-DISCLOSURE` lines (or wherever the file's first heading-block ends — executor confirms). Insert a clearly-bracketed annotation:
  > **[Annotation added 2026-05-13]** This spike's audit assumed phase-2 suites ran on `topology-single-producer.yaml`. As of 2026-05-13, the suites have been re-pointed to `topology-realistic-100.yaml` (multi-producer, 100 nodes, mass-stratified mainnet stake curve — see `.planning/spikes/006-curve-design/README.md`). The findings below (NEEDS-DISCLOSURE verdict, single-producer disclosure ranking item 1, honest-producer item 2) are preserved for historical context but are no longer the operational reality. The current topology choice is documented in `CLAUDE.md` §Calibration choices.
- **Do NOT** rewrite the spike body. Annotation only.
- **Dependencies:** none (file is purely documentation).
- **Verification:** `grep -c "Annotation added 2026-05-13" .planning/spikes/004-topology-and-actor-model/README.md` returns 1; the rest of the file is byte-identical except for that prepended block.

### Task 5: Annotate `cardano-realism-audit.md`
- **Edit 5a — Top-of-file annotation:** add a clearly-bracketed annotation immediately after the existing title and metadata block:
  > **[Annotation added 2026-05-13]** When this audit was written, phase-2 suites referenced `topology-single-producer.yaml` in CLAUDE.md and the spike trail. As of 2026-05-13, suites have been re-pointed to `topology-realistic-100.yaml` (100-node multi-producer with a mass-stratified mainnet stake curve). The "Topology and actor model" verdict and the single-producer disclosure paragraphs below are preserved for context but are no longer the operational reality; see CLAUDE.md §Calibration choices §"Topology choice" and `.planning/spikes/006-curve-design/README.md`. Per validity-threats.md the operational suites now expose slot-battle dynamics; WR-1 (no pricing-state rollback on slot-battle reorg) is correspondingly reclassified as LIVE rather than dormant — see REVIEW.md.
- **Edit 5b — Inline correction box near the topology-disclosure paragraph** (the "Single-producer topology (N=1) vs mainnet ~3,000 SPOs" item in the "What needs disclosure" section, around line 179): insert a callout immediately above that bullet:
  > **[Corrected 2026-05-13]** The disclosure below described the topology as `topology-single-producer.yaml`, which was incorrect for the operational suites at audit-time. The suites now use `topology-realistic-100.yaml` (100-node, mass-stratified mainnet curve). The N=1 single-producer disclosure no longer applies; instead, multi-producer disclosures apply: per-node controller divergence, slot-battle siblings with different pricing samples, and the WR-1 rollback gap are all live concerns. The CIP-realistic 600-pool topology remains available for any larger multi-node cross-check.
- Do NOT rewrite the audit body or the recommended-disclosure paragraphs.
- **Dependencies:** none.
- **Verification:** `grep -c "Annotation added 2026-05-13" docs/phase-2/cardano-realism-audit.md` returns 1; `grep -c "Corrected 2026-05-13" docs/phase-2/cardano-realism-audit.md` returns 1.

### Task 6: Reclassify WR-1 in REVIEW.md
- **Edit:** in the Fix Status table, locate the WR-1 row:
  - Old: `| WR-1 | Deferred to M6 | Pricing-state rollback on slot-battle reorg is an M6 deliverable per CLAUDE.md and CONCERNS.md; doesn't fire under current single-producer suites. |`
  - New: `| WR-1 | LIVE / disclosure-required | Pricing-state rollback on slot-battle reorg is no longer dormant: as of 2026-05-13 all 19 phase-2 suites run on `topology-realistic-100.yaml` (100-node multi-producer) where slot battles can fire. Two paths: (a) implement rollback (M6 deliverable per CLAUDE.md, large work), OR (b) quantify contamination via a `slot_battles_count` metric + disclose the gap in any published welfare claim. The deferred-to-M6 framing is preserved as the long-term fix; the LIVE classification reflects current operational reality. |`
- Also locate the "Deferred items (WR-1, WR-2, WR-7) are surfaced to the user for explicit decision." paragraph below the table and update it: WR-1 is no longer purely deferred — it is LIVE with a publication-time disclosure requirement. Add a sentence: "WR-1 has been reclassified to LIVE / disclosure-required as of 2026-05-13 after the topology switch to `topology-realistic-100.yaml`; the deferred-to-M6 fix path is preserved but is no longer the only option."
- **Dependencies:** none.
- **Verification:** `grep -c "LIVE / disclosure-required" .planning/REVIEW.md` returns ≥ 1; `grep -c "doesn't fire under current single-producer suites" .planning/REVIEW.md` returns 0.

### Task 7: Note in `validity-threats.md`
- **Edit:** add a `## Resolved 2026-05-13` section immediately after the `## TL;DR` block:
  > **Topology gap resolved 2026-05-13.** The previous `## TL;DR` warned that suites ran on `topology.default.yaml` while CLAUDE.md and the realism audit described `topology-single-producer.yaml` as the operational topology. As of 2026-05-13 the suites have been switched to `topology-realistic-100.yaml` (a 100-node mass-stratified mainnet curve — see `.planning/spikes/006-curve-design/README.md`), CLAUDE.md and the realism audit have been corrected to match, and the M5 suite-level goldens have been regenerated against the new topology. The trust ratings below reflect this corrected state. WR-1 (no pricing-state rollback on slot-battle reorg) is correspondingly reclassified as LIVE rather than dormant — see REVIEW.md. The recommended next-steps "Reconcile the topology gap" item (§"Recommendations to raise trust" item 1) is therefore closed.
- Do NOT delete or rewrite the existing per-suite content; the file becomes a historical record of the trust assessment with the resolution noted at top.
- **Dependencies:** Tasks 1–2 and 8 (the note asserts the goldens have been regenerated; Task 8 must complete before this note is materially true). Practically, write the note text in this task and let it sit as accurate-once-Task-8-completes; the executor's final pass confirms the sequencing.
- **Verification:** `grep -c "Topology gap resolved 2026-05-13" docs/phase-2/validity-threats.md` returns 1.

### Task 8: Regenerate the 7 M5 suite-level goldens
- **Command:** `cd sim-rs && UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism`.
- **Expected wall-clock cost:** CLAUDE.md cites ~1.5 s for the full `--ignored determinism` test in `--release`. Regen will be similar order of magnitude (it runs the same suites once-each at seed=1 and writes the new hash). Surface progress to the user; **time-box at ~5 minutes** — if it runs longer, something is wrong (e.g. the new topology is causing different slot-by-slot behaviour that exposes a determinism issue elsewhere, or the suite is panicking).
- **Files updated:** the 7 `.goldens/*.sha256` files. The directory listing should remain at exactly 7 files (we are not adding new goldens for the 12 demand-regime suites in this plan — they are not part of the determinism harness).
- **Dependencies:** Tasks 1 and 2 must be complete (new topology in place; suites re-pointed).
- **Verification:**
  - After regen, **immediately** run `cd sim-rs && cargo test --release -- --ignored determinism` (without `UPDATE_GOLDENS=1`). This must pass. If it fails, the regen produced non-reproducible hashes — investigate before continuing. (Surface to user.)
  - `git diff --stat sim-rs/parameters/phase-2-sweep/suites/.goldens/` shows 7 files modified.

### Task 9: Final verification
- `cd sim-rs && cargo build --release` — must succeed cleanly.
- `cd sim-rs && cargo test --workspace` — all tests must pass. (The 124-sim-core / 16-sim-cli / 4 / 1 counts cited in REVIEW.md are indicative; actual counts may drift slightly but no failures.)
- `cd sim-rs && cargo test --release -- --ignored determinism` — must pass (post-regen reproducibility confirmation).
- `grep -l "parameters/topology.default.yaml" sim-rs/parameters/phase-2-sweep/suites/*.yaml` — must produce empty output.
- `grep -c "topology-realistic-100" CLAUDE.md` — must return ≥ 4.
- `grep -c "LIVE / disclosure-required" .planning/REVIEW.md` — must return ≥ 1.
- `git status --short` — review what's modified. **Do NOT commit.** Per `feedback_no_commits.md` (in user memory), the executor leaves all changes staged/unstaged for the user.
- **Dependencies:** all prior tasks.

## Task DAG / sequencing

```
Task 1 (generate topology YAML) ─┬─► Task 2 (re-point 19 suite YAMLs) ─┐
                                 │                                     │
                                 └─► Task 3 (update CLAUDE.md) ────────┤
                                                                       │
Task 4 (annotate spike 004) ──────────────────────────────────────────┤
Task 5 (annotate realism audit) ──────────────────────────────────────┤
Task 6 (reclassify WR-1 in REVIEW.md) ────────────────────────────────┤
                                                                       │
                                                  Task 2 ──► Task 8 (regenerate goldens) ──┐
                                                                                            │
                                                                                Task 7 (validity-threats note) — text written any time; semantic dependency on Task 8 completion
                                                                                            │
                                                                                            ▼
                                                                                          Task 9 (final verification, no-commit)
```

Suggested execution order (single-thread, minimises rework): 1 → 2 → 3 → 4 → 5 → 6 → 7 (write text) → 8 (regen goldens) → 9 (final verification). Tasks 3–7 are documentation-only and pairwise independent; an executor preferring to parallelise can swap their order freely.

## Risks & mitigations

- **Mainnet live-snapshot drift.** Spike 006's stake table was recorded 2026-05-14 from mainnet epoch 582. If the executor re-pulls at a later epoch, individual stakes will differ; the *shape* (top-N concentrations, Nakamoto coefficient, Gini) should drift slowly. **Mitigation:** executor re-pulls and confirms either (a) per-pool stakes within ± 5 % of spike 006 values, or (b) cites the new epoch number + retrieval date in the YAML header and flags the drift to the user before proceeding to Task 8. The goldens regen is the same operation regardless.
- **Mass-stratified sampler bisect index off-by-one.** Spike 006's recipe specifies `(i + 0.5) / 100 × total_mass` for `i ∈ [0, 100)`. A sampler implementation that uses `i / 100` or `(i + 1) / 100` produces a subtly different distribution. **Mitigation:** executor cross-checks the generated table's top-N concentrations against spike 006 §"Option 1" — top-1 = 1.97 %, top-5 = 8.19 %, top-25 = 37.48 %, top-50 = 69.11 %. If those don't match within ± 0.2 percentage points, the sampler is bugged.
- **Suite YAML idiosyncratic structure.** A future suite may add multi-line YAML, a comment block above `default-topology`, or use a different key spelling. **Mitigation:** the executor MUST `grep -n default-topology` each file before editing, confirm the before-state matches the documented pattern, and surface any mismatch to the user rather than force-replacing.
- **Goldens regen wall-clock cost.** Time-boxed at 5 min per Task 8. If it runs long, halt and investigate.
- **Goldens regen non-reproducibility.** If the post-regen verification (Task 8's second invocation) fails, the new topology may have exposed a determinism issue in code outside phase-2's pricing kernel (e.g. slot lottery, propagation — see CLAUDE.md "Determinism scope"). **Mitigation:** surface the failure to the user with the specific failing suite. Do NOT attempt to "fix" by re-regenerating — that would mask a real bug.
- **Cross-machine debugging.** Regenerated goldens are pinned to the executing machine's intra-arch determinism (x86_64 / glibc). Other developers re-running on different architectures will see legitimate hash mismatches. **Mitigation:** document the regeneration date in the user-facing summary message at the end of Task 9; CLAUDE.md already covers the intra-arch caveat.
- **Suite slot-battle activity is a new condition for the simulator.** Switching from uniform `stake=100` to the realistic skewed curve materially changes the slot-lottery winner distribution. Slot battles will occur for the first time in suite goldens. **Mitigation:** this is precisely why goldens regen is required — the new event stream is the new ground truth. If `cargo test --workspace` exposes a unit-test failure caused by the topology switch (unlikely — unit tests use `config.default.yaml`'s `topology-single-producer.yaml` or hand-built configs, not `topology.default.yaml`), surface to the user. No unit test is expected to fail.
- **The 12 demand-regime suite YAMLs are not goldens-pinned**, so their behaviour after the topology switch is not regression-tested by Task 8. They will only be exercised when a user runs `experiment-suite run <demand-regime-suite>` manually. **Mitigation:** out-of-scope here; flagged for downstream.

## Out of scope (defer to follow-on work)

- **Implementing WR-1 pricing-state rollback on slot-battle reorg.** Large work; M6 deliverable per CLAUDE.md and CONCERNS.md. This plan reclassifies WR-1 from "dormant" to "LIVE / disclosure-required" but does not fix it. The post-plan publication path is either: (a) implement the rollback before publishing welfare claims, or (b) add a `slot_battles_count` metric and disclose the contamination upper bound.
- **Switching to `topology-cip-realistic.yaml` (600 pools).** Spike 006 deliberately picks the 100-node middle ground for performance + per-knob comparison defensibility. The 600-pool topology remains available for any future cross-topology validation pass.
- **Re-running phase-2 experimental suites with the new topology.** Significant compute investment (19 suites × multiple jobs × 3 seeds). Plan as a separate downstream effort.
- **Updating CLAUDE.md's 7-suite table to reflect the actual 19-suite directory.** Cosmetic; this plan adds a one-line annotation rather than rewriting. A full table update can land in a follow-on cleanup pass.
- **Cross-architecture CI verification.** Existing CLAUDE.md caveat ("Cross-architecture CI verification is not yet built"); unchanged by this plan.
- **Resolving the 4 UNRESOLVED suite trust ratings in validity-threats.md.** Requires reading actual suite outputs after the rerun.
- **Adding `multiplier_floor = 16` companion runs to `rb-scarcity` and `urgency-inversion`** (validity-threats.md recommendation 3) — design + compute investment for a separate plan.
- **Resolving CR-1 (`f64::sqrt` in `endorsement_window_priced_blocks`).** Rust code change, explicitly excluded from this plan's "YAML + Markdown only" constraint.
- **Resolving WR-2 (`AdmissionRejected { reason }` event).** Public-API change; same reason.
- **Increasing seed count from 3 to ≥ 10.** Compute investment; separate.
- **Committing or tagging the resulting changeset.** Per `feedback_no_commits.md`, the executor leaves everything for the user to commit.
