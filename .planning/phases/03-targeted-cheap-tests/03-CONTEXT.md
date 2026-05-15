# Phase 3: Targeted Cheap Tests - Context

**Gathered:** 2026-05-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Resolve the 12 LIVE entries in [`docs/phase-2/realism-risks-register.md`](../../../docs/phase-2/realism-risks-register.md) via targeted cheap simulator tests. Each test produces variance bands, sensitivity verdicts, or steady-state results that flip rows in [`docs/phase-2/coverage-check.md`](../../../docs/phase-2/coverage-check.md) from `UNBACKED` / `WEAK` to `BACKED` where the evidence supports it. This is the only Phase in this milestone that writes new Rust code (~150 LoC for `sim-cli/src/metrics/paired_bootstrap.rs`) and that consumes appreciable simulator compute.

The phase delivers eight artefacts:
- `sim-cli/src/metrics/paired_bootstrap.rs` (TEST-01) — paired-sample Bias-corrected and accelerated (BCa) bootstrap library
- Wall-clock scoping run determining N for the variance-band tests (TEST-02)
- `.planning/realism-tests/multi-seed-variance/` — TEST-03 (four sign-flip cells) + TEST-04 (five canonical menu-item welfare cells) results
- `.planning/realism-tests/pool-number-sensitivity/` — TEST-05 33-job smoke × {100, 150 pools} × {`sundaeswap_moderate`, 4 `paper_like_*` variants}
- `.planning/realism-tests/run-length-steady-state/` — TEST-06 results per menu option at {2000, 4000, 8000} slots
- `.planning/realism-tests/multiplier-floor-16-companion/` — TEST-07a results for `phase-2-rb-scarcity` and `phase-2-urgency-inversion` at `multiplier_floor = 16`
- `sim-rs/parameters/phase-2-sweep/topology-realistic-150.yaml` — 150-pool topology needed by TEST-05
- `docs/phase-2/coverage-check.md` updates — `CLM-NN` status fields flip from `UNBACKED` → `BACKED` / `WEAK` / `re-run-needed` as test waves land; hash-diversity gate (COV-05) applied in the closeout wave

Requirements covered: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, TEST-07 (sub-requirement TEST-07a), COV-05.

</domain>

<decisions>
## Implementation Decisions

### `paired_bootstrap.rs` shape (TEST-01)

- **D-22:** `paired_bootstrap.rs` lives in `sim-rs/sim-cli/src/metrics/` as a **pure library module**. Exports a single public function with shape `paired_bca_ci(samples_a: &[f64], samples_b: &[f64], alpha: f64, bootstrap_seed: u64) -> CiResult` (or equivalent — exact type names at planner discretion). No new sub-command on `experiment-suite`; Phase 3's multi-seed runner consumes the library directly. Unit-tested against a paired-Gaussian synthetic dataset with known mean ± confidence interval. Does not perturb existing golden hashes (the module is post-processing on collected per-seed scalars, not on the simulation hot path).

- **D-23:** The BCa bootstrap takes a **deterministic `bootstrap_seed: u64` parameter**; same inputs + same seed ⇒ same `CiResult` bit-for-bit. Outputs in `.planning/realism-tests/` are reproducible. The bootstrap-seed namespace is separate from the simulator-seed namespace (different role, different inputs); each test artefact records the bootstrap seed it used.

- **D-24:** The per-seed scalar fed into `paired_bca_ci` is `retained_value` (a single `f64` per (job, seed) pair, taken from `RunSummary`). BACKED/WEAK gate operates on the `retained_value` confidence interval. Subsidiary metrics (`net_utility`, `retained_value_ratio`) MAY be reported alongside as median + Inter-Quartile Range (IQR) for informational comparison but do NOT gate verdict.

### Multi-seed runner orchestration (TEST-02..07a)

- **D-25:** Phase 3's multi-seed runs are packaged as **new suite YAMLs** under `sim-rs/parameters/phase-2-sweep/suites/`, one per test:
  - `phase-3-scoping.yaml` — TEST-02 (N=5 calibration run on one canonical job)
  - `phase-3-sign-flip-variance.yaml` — TEST-03 (the four sign-flip cells)
  - `phase-3-canonical-variance.yaml` — TEST-04 (one canonical job per menu option + single-lane control = 5 cells)
  - `phase-3-pool-number-sensitivity.yaml` — TEST-05 (the 33-job smoke at 100 pools is already pinned; this YAML is the 150-pool variant covering the same smoke × {`sundaeswap_moderate`, 4 `paper_like_*` variants})
  - `phase-3-run-length.yaml` — TEST-06 (4 menu-option jobs × 3 slot lengths)
  - `phase-3-multiplier-floor-16-companion.yaml` — TEST-07a (`phase-2-rb-scarcity` and `phase-2-urgency-inversion` at `multiplier_floor = 16`)

  These suites are **NOT goldens-pinned**. Phase 3 outputs land in `.planning/realism-tests/<test-name>/`, not in `parameters/phase-2-sweep/suites/.goldens/`. The existing `experiment-suite run` already iterates seeds via `Suite.seeds: Vec<u64>`; no runner changes needed.

- **D-26:** Seeds for each multi-seed cell are **sequential `[1, 2, ..., N]`**. Recorded verbatim in each Phase 3 suite YAML's `seeds:` field. The four TEST-03 sign-flip cells share one seed set (paired-bootstrap requires shared seeds across the two compared arms); the five TEST-04 cells likewise; TEST-05's per-job seed-set is whatever each `paper_like_*` / `sundaeswap_moderate` smoke uses (already in tree).

- **D-27:** `docs/phase-2/coverage-check.md` is updated **incrementally**: each TEST-NN's commit drops `.planning/realism-tests/<name>/` results AND flips the relevant `CLM-NN` rows' `status` column (`UNBACKED` → `BACKED` / `WEAK` / `re-run-needed`) AND populates `confidence-method`, `seeds-cited`, `backing-job`, `golden-sha256` (where applicable) in the same commit. Reviewer can grep `git log -p docs/phase-2/coverage-check.md` to see Phase 3 evidence accumulating. No batched end-of-phase coverage-check rewrite.

### 150-pool topology (TEST-05 prerequisite)

- **D-28:** `topology-realistic-150.yaml` is generated by **re-parameterising `sim-rs/scripts/generate-realistic-100-topology.py`** with `N_NODES = 150` against the **same epoch-582 Cardano mainnet snapshot** already in tree. Mass-stratification samples 150 cumulative-mass strata instead of 100. The Δ% comparison between 100 and 150 pools isolates pool count as the only variable changing. (Re-querying Koios for a fresh epoch was considered and rejected — it would introduce simultaneous epoch-drift, defeating the test design.)

- **D-29:** The 50 extra nodes (node-100..node-149) get network properties via **sample-with-jitter from the existing 100**: for each new node, pick a random existing node as template, copy its `location` (2-vector) and `bandwidth-bytes-per-second`, then perturb each outbound producer's `latency-ms` by a Gaussian factor with ±5–10% standard deviation. A documented PRNG seed (e.g. `jitter_seed = 582` for the snapshot epoch) keeps the generator deterministic. The YAML header documents both the seed and the jitter SD. Trade-off: cloned-templates (no jitter) was rejected for creating suspicious latency-cluster structure; full distribution sampling (KDE/histogram fit) was rejected as over-engineered for ~150 LoC of generator delta.

- **D-30:** The total stake rescale target stays at **3×10^10 lovelace** (matches `topology-realistic-100.yaml` and `topology-cip-realistic.yaml`). Pool count is the only variable changing; per-pool average stake drops from `3e10 / 100` to `3e10 / 150` (deliberate — per-pool average stake decreases is itself a property of going from 100 to 150 pools at fixed total). Lottery-quantization check (`min_stake × rb_prob ≥ 100`) re-verified at N=150.

### Plan wave decomposition

- **D-31:** Three plan waves. Plan 03-01 (Wave 1, parallel): TEST-01 paired_bootstrap.rs implementation + TEST-02 scoping run (one canonical job at N=5 seeds, wall-clock measured) + `topology-realistic-150.yaml` generation. The three sub-tasks are independent and parallelise naturally. Plan 03-02 (Wave 2, parallel inside the wave): TEST-03 sign-flip variance, TEST-04 canonical variance, TEST-05 pool-number sensitivity, TEST-06 run-length / steady-state, TEST-07a multiplier-floor-16 companion. Five sub-tasks all gated on Wave 1; each runs its suite YAML, collects per-seed scalars, calls `paired_bca_ci` where applicable, writes results to `.planning/realism-tests/<name>/results.md`, and updates `coverage-check.md` `CLM-NN` rows. Plan 03-03 (Wave 3, sequential): COV-05 hash-diversity gate applied across all `BACKED`-labelled rows; any row whose distinct-`sha256` count < seed count is downgraded to WEAK with annotation OR re-run with different seed values. Final consolidation pass on coverage-check.md (table sort, anchor consistency, RSK cross-reference check).

### BACKED / WEAK / re-run-needed verdict criteria

- **D-32:** Multi-seed-variance verdict (TEST-03, TEST-04) is **conjunctive**:
  - **BACKED** iff (a) `paired_bca_ci`'s 95% confidence interval on the `retained_value` delta does NOT cross zero AND (b) the COV-05 hash-diversity gate passes (distinct `pricing_event_stream.sha256` count = N).
  - **WEAK** iff (a) holds but the CI crosses zero (sign of the welfare delta is not statistically defended at N seeds); evidence is still cited in CIP but flagged as variance-not-yet-resolved.
  - **re-run-needed** iff the hash-diversity gate fails (distinct count < N) — the seed-set collapsed and must be re-drawn before any verdict is licensed.
  - **Sign-coherence** (fraction of seeds agreeing on sign of delta) is reported in every results.md as an **informational** indicator; it does NOT gate the verdict on its own (a high sign-coherence with a tiny absolute delta and a CI crossing zero remains WEAK).

### Run-length / steady-state criterion (TEST-06)

- **D-33:** TEST-06 steady-state metric is **per-(job, seed) paired comparison on `retained_value`-per-slot**:
  - For each (job, seed): compute the rolling mean of `retained_value`-per-slot over slots `[N/4, N/2]` (first quarter of the steady portion) versus slots `[N/2, 3N/4]` (second quarter). The first quarter `[0, N/4]` is discarded as start-up transient.
  - Per-seed delta = `mean(second_quarter) − mean(first_quarter)`.
  - STEADY iff `|median(deltas across seeds)| < seed-IQR(retained_value over full run)`. Per-(job, length) verdict.
  - If the 2000-slot default fails the criterion for menu option M, the canonical job for M is re-run at 4000 slots; if 4000 fails, 8000. The suite default is raised for any menu option that requires longer than 2000 slots; the suite YAML carrying that menu option is updated accordingly.

### Claude's Discretion

The following items have planner / executor latitude with reasonable defaults named here:

- **`CiResult` type shape.** A struct with `point_estimate`, `lower`, `upper`, `alpha`, `n_bootstrap`, and ideally `bootstrap_seed` echoed for self-documenting JSON serialisation. Exact field names + serde derives at planner discretion; the type should serialise via serde so results write directly into `.planning/realism-tests/<name>/<cell>.json` (or .md with the JSON embedded). The library MAY also expose a separate `paired_delta_summary(samples_a, samples_b) -> {median, iqr, sign_coherence}` helper so the informational stats land in the same module.

- **Bootstrap iteration count.** The standard default `n_bootstrap = 9999` (rounded to make percentile cuts cleanly land on integer indices) is fine. The literature accepts anywhere from 1000 to 50000; the test's per-cell wall-clock is dominated by simulator runs, not the bootstrap, so picking 9999 has negligible cost.

- **TEST-02 canonical job choice.** The scoping run goes on **one** canonical menu-item job (the requirement says one). Pick a representative job from either the `phase-2-priority-only-unreserved` suite or `phase-2-two-lane-both-dynamic` suite (both have multiple known welfare-delta cells); document the choice in the `phase-3-scoping.yaml` header. Avoid the four sign-flip cells themselves so the scoping result isn't biased by the very cells we're trying to characterise.

- **TEST-02 target wall-clock.** REQUIREMENTS lock target N=15–20 with fallback N=10. The N choice from TEST-02's wall-clock measurement is mechanical: aim for total compute per cell ≤ `~30 min × parallelism` (the existing M5 goldens take ~1.5s × 200 slots × 100 nodes baseline; phase-2 suite jobs typically run minutes, not hours, per (job, seed) pair). The planner records the wall-clock-per-(job,seed) and the chosen N in TEST-02's results.md.

- **`.planning/realism-tests/<name>/results.md` shape.** Each test directory has one `results.md` summarising the verdict per cell, plus `<cell>.json` per cell for raw data. Markdown table at the top of `results.md`: cell | N seeds | distinct-hash count | `retained_value` delta median | BCa 95% CI [lower, upper] | sign-coherence | verdict. Body sections explain methodology, threshold application, and any re-run-needed cells. Planner picks exact column order if minor variants improve readability.

- **150-pool topology jitter SD.** ±5–10% Gaussian jitter on latencies, exact SD at planner discretion. 7% is the sensible midpoint; document in the YAML header alongside `jitter_seed`.

- **Multiplier-floor-16 companion run details (TEST-07a).** Take the existing `phase-2-rb-scarcity.yaml` and `phase-2-urgency-inversion.yaml` job definitions; create like-for-like companion jobs with the only override being `multiplier_floor = 16` (currently 4 in those two suites). All other knobs hold (window-length, demand profile, topology, slots). Two jobs total in `phase-3-multiplier-floor-16-companion.yaml`. Per Phase 1 SUMMARY: verdict MITIGATED iff the qualitative finding replicates at 16 (or is reframed as "observable only when floor is low enough to admit medium-urgency components to priority"); LIVE → DISCLOSED if the finding inverts at 16.

- **Verdict on TEST-05.** Per REG-05's locked text: `topology-realistic-150.yaml` rerun of (33-job smoke × 5 demand profiles) at 150 pools vs the existing 100-pool runs. Per (job, demand profile), compute Δ% on `retained_value`, `net_utility`, `retained_value_ratio` between 100 and 150 pools. **MITIGATED** for the underlying `RSK-pool-count` and `RSK-calibration-stale-stake-snapshot` entries iff for every (job, demand profile) the Δ% is within the 100-pool seed-IQR of the corresponding metric. **LIVE → DISCLOSED** if any (job, demand profile) pair shows Δ% outside the seed-IQR threshold — the disclosure-paragraph drafted in Phase 1 for `RSK-pool-count` becomes the load-bearing fallback. Verdict aggregation: a single LIVE → DISCLOSED failure on a single (job, profile) keeps the register entry LIVE-going-to-DISCLOSED (no per-cell DISCLOSED in the register's verdict vocabulary).

- **Coverage-check `golden-sha256` for Phase 3 rows.** Phase 3 suites are not goldens-pinned, but each cell's `pricing_event_stream.sha256` is still emitted by every run. The `golden-sha256` column in `coverage-check.md` cites the per-seed `sha256` value from the seed=1 baseline (a single value), with a parenthetical "(plus N–1 additional `sha256` values; full list in `.planning/realism-tests/<name>/<cell>.json`)". Distinct-hash gate is applied across the full N-tuple, but the table cites the seed=1 representative for readability.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level
- [`.planning/PROJECT.md`](../../PROJECT.md) — Project context, core value, Active requirements, Out of Scope, Key Decisions (especially the targeted-cheap-test pattern, pool-number-sensitivity test as prototype-pattern)
- [`.planning/REQUIREMENTS.md`](../../REQUIREMENTS.md) — REQ-IDs covered by this phase: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, TEST-07 (TEST-07a sub-requirement), COV-05
- [`.planning/ROADMAP.md`](../../ROADMAP.md) §"Phase 3: Targeted Cheap Tests" — goal, dependencies (Phase 1 + Phase 2), 7 success criteria
- [`CLAUDE.md`](../../../CLAUDE.md) — numeric-representation contract (f64 reporting only, integer/rational in sim-affecting state), abbreviation-on-first-use rule, determinism scope, calibration choices, running the suites

### Phase 1 outputs (the LIVE entries Phase 3 must resolve)
- [`docs/phase-2/realism-risks-register.md`](../../../docs/phase-2/realism-risks-register.md) — the 24 RSK-NN entries (12 LIVE + 12 DISCLOSED). LIVE entries with EXP-NN → TEST-NN cross-references are the inputs to test ordering. The four mandatory LIVE entries (RSK-pool-count, RSK-single-seed-precision, RSK-un-anchored-controller-knobs, RSK-substrate-scope) plus eight other LIVE entries are the work this phase resolves.
- [`.planning/phases/01-register-inventory/01-CONTEXT.md`](../01-register-inventory/01-CONTEXT.md) — decisions D-01..D-10; particularly D-06 (verdict vocabulary) and D-10 (REG-05 locked threshold for RSK-pool-count)
- [`.planning/phases/01-register-inventory/01-02-SUMMARY.md`](../01-register-inventory/01-02-SUMMARY.md) — the realised EXP-NN ↔ TEST-NN mapping table; surfaces TEST-07a (`EXP-multiplier-floor-16-companion-run`); names the four UNRESOLVED non-pinned suites; notes the three new Phase-2-facing EXP-NN slugs

### Phase 2 outputs (the coverage rows Phase 3 will update)
- [`docs/phase-2/coverage-check.md`](../../../docs/phase-2/coverage-check.md) — the ~25–40 CLM-NN rows that Phase 3 flips from `UNBACKED` → `BACKED` / `WEAK` / `re-run-needed`. The `confidence-method`, `seeds-cited`, `backing-job`, `golden-sha256`, and `status` columns are populated per row as each test wave lands.
- [`.planning/phases/02-coverage-check-skeleton/02-CONTEXT.md`](../02-coverage-check-skeleton/02-CONTEXT.md) — decisions D-11..D-21; particularly D-16 (CLM verdict vocabulary), D-19 (strict hash-diversity gate semantics) which Phase 3 enforces, and D-15 (CLM-NN append-only)

### Source documents for test scoping
- [`.planning/mechanism-welfare-impact-2026-05-14.md`](../../mechanism-welfare-impact-2026-05-14.md) — names the four sign-flip cells (`d4_t50_w32`, `d8_t25_w32`, and `x4_rb_quarter` under both rb-reserved-priority and partitioned arms) that TEST-03 targets; documents the 33-job sundaeswap-smoke 100-pool baseline that TEST-05 compares against
- [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md) — authoritative Family B (chain-derived controller) commit; the mechanism whose welfare claims Phase 3 substantiates
- [`.planning/family-b-results-table-2026-05-14.md`](../../family-b-results-table-2026-05-14.md) — single-seed welfare-cell results; TEST-04's canonical cells reference these for the menu-option choice
- [`.planning/family-b-full-sweep-analysis-2026-05-14.md`](../../family-b-full-sweep-analysis-2026-05-14.md) — full-sweep characterisation; informs TEST-04 menu-option selection

### Code references (where Phase 3 modifies / extends)
- [`sim-rs/sim-cli/src/metrics/`](../../../sim-rs/sim-cli/src/metrics/) — module home for new `paired_bootstrap.rs` (TEST-01); existing files: `mod.rs`, `collector.rs`, `comparison.rs`, `diagnostics.rs`, `time_series.rs`
- [`sim-rs/sim-cli/src/metrics/collector.rs`](../../../sim-rs/sim-cli/src/metrics/collector.rs) — `RunSummary.retained_value` is the scalar Phase 3 consumes per seed
- [`sim-rs/sim-cli/src/suite.rs`](../../../sim-rs/sim-cli/src/suite.rs) — `Suite.seeds: Vec<u64>` and `Job.seeds: Option<Vec<u64>>` are the existing multi-seed knobs; no runner changes needed (only new suite YAMLs)
- [`sim-rs/sim-cli/src/runner.rs`](../../../sim-rs/sim-cli/src/runner.rs) — `run_suite`, `run_job`, `verify_suite`; existing infrastructure already iterates seeds, parallelises with `--parallelism N`, deduplicates Completed (job, seed) pairs
- [`sim-rs/scripts/generate-realistic-100-topology.py`](../../../sim-rs/scripts/generate-realistic-100-topology.py) — the topology generator Phase 3 re-parameterises with `N_NODES = 150` (TEST-05 prerequisite). The script is deterministic given the same on-chain snapshot.

### Suite layout context
- [`sim-rs/parameters/phase-2-sweep/suites/`](../../../sim-rs/parameters/phase-2-sweep/suites/) — the 7 goldens-pinned suites + 12 unpinned demand-regime suites. Phase 3 adds 6 more (not goldens-pinned) per D-25.
- [`sim-rs/parameters/phase-2-sweep/protocol-base.yaml`](../../../sim-rs/parameters/phase-2-sweep/protocol-base.yaml) — Phase 3 suites inherit from this (or its RB-reduced overlays where TEST-05 / TEST-03 cells require it)
- [`sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml`](../../../sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml) — the 100-pool baseline; the 150-pool variant carries identical structure plus 50 jittered extras
- [`sim-rs/parameters/phase-2-sweep/demand/`](../../../sim-rs/parameters/phase-2-sweep/demand/) — `paper_like_congested.yaml`, `paper_like_mispriced.yaml`, `paper_like_moderate.yaml`, `paper_like_realistic.yaml`, and `sundaeswap_moderate.yaml` profiles used by TEST-05
- [`sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml`](../../../sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml) and [`sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml`](../../../sim-rs/parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml) — Phase 3 / TEST-07a creates `multiplier_floor = 16` companion jobs for these two suites

### Codebase maps
- [`.planning/codebase/STACK.md`](../../codebase/STACK.md) — `statrs` is already in the dependency tree (TEST-01 requires no new crate)
- [`.planning/codebase/TESTING.md`](../../codebase/TESTING.md) — `sha2` + `hex` + `tempfile` already in tests; unit-test pattern for `paired_bootstrap.rs` follows the `mod tests` inline convention used in `single_lane.rs` / `two_lane.rs` / `window.rs`
- [`.planning/codebase/STRUCTURE.md`](../../codebase/STRUCTURE.md) — `metrics/` module layout; new `paired_bootstrap.rs` adds one file with `pub use` from `mod.rs`

### Methodology references
- [`.planning/research/STACK.md`](../../research/STACK.md) — Paired Seed Evaluation (PSE) methodology; BCa-bootstrap selection rationale
- [`.planning/research/PITFALLS.md`](../../research/PITFALLS.md) — CRIT-1 (single-seed claims at publication precision) is what TEST-03 / TEST-04 resolve; CRIT-5 (un-validated topology) is what TEST-05 resolves; MOD-5 (threshold-before-test) is what RSK-pool-count's locked text encodes

### External precedent
- Hesterberg, T. C. (2015) "What Teachers Should Know About the Bootstrap" — BCa bootstrap reference; covered in `statrs` documentation
- DiCiccio, T. J. & Efron, B. (1996) "Bootstrap Confidence Intervals" — paired-BCa derivation reference
- The user's `paired_bootstrap.rs` implementation MAY follow a published reference implementation (e.g. Python's `scipy.stats.bootstrap` API) but must use only `statrs` for any percentile / inverse-CDF helpers

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`Suite.seeds: Vec<u64>` and per-`Job.seeds: Option<Vec<u64>>`** in [`sim-rs/sim-cli/src/suite.rs`](../../../sim-rs/sim-cli/src/suite.rs) — already supports running N seeds per job. Phase 3 needs no runner changes; just author YAMLs with the right seed lists.
- **`experiment-suite run --parallelism N`** in `sim-cli/src/bin/experiment-suite/main.rs` — already parallelises (job, seed) pairs concurrently. Phase 3 takes free advantage; raise N if your machine has headroom.
- **`RunSummary.retained_value: f64`** in [`sim-rs/sim-cli/src/metrics/collector.rs`](../../../sim-rs/sim-cli/src/metrics/collector.rs) — the per-(job, seed) scalar Phase 3's TEST-03/04 collect into samples_a / samples_b before paired bootstrap. Already serialised to `<seed>/run_summary.json` (snake_case via no rename_all) per existing runner conventions.
- **`pricing_event_stream.sha256`** in `RunSummary` — already computed and written per (job, seed). COV-05 hash-diversity gate reads these directly from the seed-directories; no new emission code needed.
- **`statrs`** crate (already in `sim-cli/Cargo.toml`) — provides Normal CDF (`statrs::distribution::Normal::cdf`) needed for BCa's z-score → percentile transform.
- **`sim-rs/scripts/generate-realistic-100-topology.py`** — the existing 100-pool generator; D-28 / D-29 re-parameterise its `N_NODES` constant from 100 to 150 and add the jitter-from-template extension subroutine for nodes 100..149.

### Established Patterns

- **`mod tests` inline at file bottom** — unit-test convention used in `tx_pricing/single_lane.rs` (lines 403-575), `tx_pricing/two_lane.rs` (lines 410-727), `tx_pricing/window.rs` (lines 114-201), `sim-cli/src/runner.rs` (lines 955-1050). TEST-01's `paired_bootstrap.rs` follows the same pattern: synthetic-paired-Gaussian dataset, deterministic bootstrap seed in test, assert returned CI bounds land within tolerance of analytic ground truth.
- **Suite YAML schema** — kebab-case via `#[serde(rename_all = "kebab-case")]` on `Suite`, `Job`, `JobOverrides`. Phase 3 suite YAMLs follow the existing M3/M4/M5 conventions (mixed snake/kebab is historical accident per CLAUDE.md; new Phase 3 suites match the YAML-side kebab-case shape).
- **Integer/rational determinism contract** — `paired_bootstrap.rs` consumes `f64` reporting scalars (per CLAUDE.md "Reporting outputs are plain `f64`") and produces `f64` CI bounds. This is allowed because the module is post-processing on `RunSummary` (already `f64`); does not feed back into simulation. CLAUDE.md's "no f64 in simulation-affecting state" rule is preserved.
- **Per-(job, seed) artefact directories** at `sim-rs/output/<suite>/<job>/<seed>/` — Phase 3 reads these for `retained_value` and `pricing_event_stream.sha256` aggregation. New `.planning/realism-tests/<test>/<cell>.json` artefacts mirror this structure conceptually but live outside the sim-rs/output tree (different lifecycle: results.md is checked in; sim-rs/output/ is not).
- **3-layer determinism regime** (unit-test goldens, `experiment-suite verify`, suite-level goldens per `tests/determinism.rs`) — Phase 3 does NOT extend this regime. Phase 3 outputs land in `.planning/realism-tests/`, not in `.goldens/`. The hash-diversity gate is a Phase-3-specific assertion on per-seed `sha256` distinctness, not a golden-hash pin.

### Integration Points

- **Phase 2's `coverage-check.md` `CLM-NN` rows** — Phase 3 reads `UNBACKED` rows to prioritise test work (which CLM rows reference which test); writes `BACKED` / `WEAK` / `re-run-needed` status + populates `confidence-method`, `seeds-cited`, `backing-job`, `golden-sha256` per row as each wave lands. Same-commit pattern (D-27).
- **Phase 1's register `EXP-NN` slugs** — Phase 3 reads the LIVE entries' `scope-of-resolution` field for test hypotheses; writes nothing to the register (Phase 4 / DOC-01 will flip verdicts based on Phase 3 results, not Phase 3 itself).
- **Phase 4 inputs** — every `re-run-needed` row whose re-run still fails the hash-diversity gate becomes a Phase 4 disclosure (LIVE → DISCLOSED). Every `WEAK` row similarly enters Phase 4's refresh either as accepted-WEAK evidence or as a disclosure. Phase 3's `results.md` summary tables become Phase 4's input.
- **Existing experiment-suite parallelism cap** (`min(available_parallelism(), 8)`) — Phase 3's W2 has 5 parallel suite runs across 5 sub-tasks; if the user runs all five concurrently via `scripts/run-parallel-suites.sh`, the cross-suite K × intra-suite P product can exceed available cores. Planner notes this in W2 task description (recommend running W2 suites sequentially within one shell at `-P 8`, NOT cross-suite-parallel, on the dev machine; raise on larger hardware).

</code_context>

<specifics>
## Specific Ideas

- **Same seed set across the four TEST-03 sign-flip cells.** Pairing requires shared seeds; sequential `[1..N]` is the canonical choice. If TEST-02's wall-clock calibration lands at N=20, the four sign-flip cells use seeds `[1..20]` each.
- **TEST-04's 5 canonical cells share a seed set across the 4 menu-option arms.** The single-lane EIP-1559 control uses the same seed-set for paired comparison against each menu option. N=10 by default; TEST-02 may revise.
- **TEST-05's 5 demand profiles** (`sundaeswap_moderate` + 4 `paper_like_*` variants) at 33 jobs each × 2 pool counts (100, 150) = 330 (job, pool-count) pairs. The existing 100-pool 33-job sundaeswap-smoke is in tree; the four `paper_like_*` 100-pool variants need a 33-job smoke generated. The 150-pool side of all 5 demand profiles is fresh. Per CLAUDE.md operational sizing, this is the heaviest compute step in Phase 3 and should run on a machine with N ≥ 8 cores; expected wall-clock per profile is "a few hours" (rough — TEST-02's scoping is the only authoritative wall-clock number).
- **TEST-06's 4 canonical jobs** are one per CIP menu option (NOT the single-lane control — TEST-06 is about menu options' steady-state behaviour). One canonical job per menu option × 3 slot lengths × N seeds = 12 (job, length, seed-set) triples. N=10 by default unless TEST-02 dictates lower.
- **TEST-07a's 2 jobs** carry like-for-like `multiplier_floor` override; everything else holds. Two cells; no paired bootstrap needed since this is a re-run of a known cell at a different floor — comparison is between the existing 100-pool single-seed run at floor=4 and the new floor=16 result. Verdict is qualitative: "the existing finding replicates at 16" (MITIGATED) or "the finding inverts at 16" (LIVE → DISCLOSED per Phase 1 plan-02 SUMMARY).
- **The COV-05 hash-diversity gate** runs in Wave 3. By that point all per-seed `sha256` values are in `.planning/realism-tests/<test>/<cell>.json`; the wave 3 task aggregates and checks distinct counts. Any BACKED row whose count < N gets downgraded to WEAK with annotation `"hash collision detected: distinct count = K < N"`; the re-run alternative (different seed values) is recorded as a deferred-to-Phase-4 item if encountered.
- **`paired_bca_ci` API hint.** The simplest API has `CiResult { point: f64, lower: f64, upper: f64, alpha: f64, n_bootstrap: u32, bootstrap_seed: u64 }` (serde-derive Serialize). `paired_bca_ci(samples_a, samples_b, alpha, bootstrap_seed)` panics if `samples_a.len() != samples_b.len()` (paired requires equal lengths) and if either is empty. Use `f64::is_finite` guards on input. ~150 LoC total.

</specifics>

<deferred>
## Deferred Ideas

- **TEST-07 sub-requirements beyond TEST-07a.** REQUIREMENTS.md TEST-07 reserves space for 3–5 additional cheap tests; only TEST-07a has surfaced from Phase 1 plan-02. If Phase 3 execution surfaces a new LIVE-entry-driven test that doesn't fit TEST-03/04/05/06/07a, the planner adds a TEST-07b row in REQUIREMENTS.md and an additional sub-wave in Plan 03-02 — but the default assumption is no further TEST-07x sub-requirements emerge.
- **Promoting Phase 3 suites to goldens-pinned.** Out of scope per PROJECT.md "Out of Scope" §"Promoting unpinned demand-regime suites to goldens-pinned" and per D-25. Phase 3 suites stay unpinned for this milestone.
- **CIP-0164 600-pool topology migration.** Out of scope per PROJECT.md; superseded by TEST-05. If TEST-05 surfaces a real (100 vs 150) sensitivity, the existing `docs/phase-2/m6-implementation-plan.md` (deferred) becomes a future-milestone contingency.
- **Cross-architecture CI verification.** Out of scope per PROJECT.md and per `.planning/codebase/CONCERNS.md`. Phase 3's `paired_bca_ci` is bit-deterministic on the same arch (deterministic seed + integer-percentile cuts) but cross-arch verification is not in this milestone.
- **Adversarial-actor regime.** Out of scope per PROJECT.md; deferred to a future milestone after CIP publication. Phase 3 actors remain utility-maximising via `LanePolicy::UtilityMaximising`.
- **Cross-reference index automation.** Phase 1 / Phase 2 deferred. Still deferred.
- **Refresh of `cardano-realism-audit.md` and `validity-threats.md`.** Phase 4 work (DOC-01, DOC-02). Phase 3 leaves both documents as Phase 2 left them.
- **Anchoring of the four un-anchored controller knobs.** Phase 4 / DOC-03 work; the four `EXP-*-anchor` slugs map to that work, not to Phase 3 tests.
- **CIP author summary.** Phase 5 / HAND-01 work.

No reviewed-todo deferrals (the `cross_reference_todos` step returned an empty matches set).

</deferred>

---

*Phase: 3-Targeted Cheap Tests*
*Context gathered: 2026-05-15*
