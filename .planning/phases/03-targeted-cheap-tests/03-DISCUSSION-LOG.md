# Phase 3: Targeted Cheap Tests - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-15
**Phase:** 3-Targeted Cheap Tests
**Areas discussed:** Bootstrap module shape, Multi-seed orchestration, 150-pool topology, Wave decomposition & verdict criteria

---

## Bootstrap module shape (TEST-01)

### Q1: API surface for `paired_bootstrap.rs`

| Option | Description | Selected |
|--------|-------------|----------|
| Pure library | Module exports `paired_bca_ci(...)`; multi-seed runner orchestration calls it after collecting per-seed scalars. No new CLI sub-command. | ✓ |
| Library + thin CLI | Same library + an `experiment-suite bootstrap-ci <input.json>` sub-command. Useful if orchestration is bash/python. | |
| Library + integrated | Library + a hook into `experiment-suite run` that auto-computes CIs when N>1 seeds are present. Largest surface. | |

**User's choice:** Pure library (Recommended)
**Notes:** Smallest surface; easiest to unit-test. Locked as D-22.

### Q2: Bootstrap-seed determinism

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed seed per call | Caller passes a `u64 bootstrap_seed`; same inputs + same seed ⇒ same CI bit-for-bit. | ✓ |
| System entropy | Bootstrap uses `thread_rng`; CI varies slightly between re-runs. | |

**User's choice:** Fixed seed (Recommended)
**Notes:** Bit-reproducibility on `.planning/realism-tests/` outputs preserved. Bootstrap seed namespace separate from simulator seed namespace. Locked as D-23.

### Q3: Per-seed scalar input

| Option | Description | Selected |
|--------|-------------|----------|
| Per-seed `retained_value` | One scalar per (job, seed); paired bootstrap of arm A vs arm B. Simplest. | ✓ |
| Per-seed welfare-tuple | Multi-dimensional sample: `(retained_value, net_utility, retained_value_ratio)`. Bootstrap CI per dimension. | |
| Per-seed welfare-delta only | Pre-computed per-seed delta; 1-D bootstrap. Equivalent to (1) but the caller pre-computes pairing. | |

**User's choice:** Per-seed `retained_value` (Recommended)
**Notes:** Subsidiary metrics may be reported alongside but don't gate verdict. Locked as D-24.

---

## Multi-seed orchestration (TEST-02..07a)

### Q1: Run packaging

| Option | Description | Selected |
|--------|-------------|----------|
| New phase-3 suite YAMLs | Add new suite YAMLs under `parameters/phase-2-sweep/suites/` (not goldens-pinned). Existing `experiment-suite run` iterates seeds. | ✓ |
| Override seeds on existing suites | Re-use existing suites with a wrapper script. Couples Phase 3 to whatever existing suites encode. | |
| Bespoke per-cell mini-YAMLs | One tiny YAML per (cell, N). Maximally explicit but heavier maintenance. | |

**User's choice:** New phase-3 suite YAMLs (Recommended)
**Notes:** Six new suite YAMLs (phase-3-scoping, sign-flip-variance, canonical-variance, pool-number-sensitivity, run-length, multiplier-floor-16-companion). Not goldens-pinned. Locked as D-25.

### Q2: Seed selection methodology

| Option | Description | Selected |
|--------|-------------|----------|
| Sequential `1..N` | Seeds = `[1, 2, …, N]`. Trivially reproducible; matches existing M3/M4 convention. | ✓ |
| Sampled-but-pinned | Seeds = first N draws from a documented PRNG. Adds bookkeeping; no evidence of seed=1 contamination. | |
| Disjoint per-cell seed sets | Each cell uses non-overlapping range. Incompatible with paired-bootstrap unless cells share a set. | |

**User's choice:** Sequential `1..N` (Recommended)
**Notes:** Same seed-set across the four TEST-03 sign-flip cells (paired requirement). Locked as D-26.

### Q3: When to update `coverage-check.md`

| Option | Description | Selected |
|--------|-------------|----------|
| Incrementally as each test lands | Each commit drops `.planning/realism-tests/<name>/` results AND flips the relevant CLM-NN rows in the same commit. | ✓ |
| Batch at end of Phase 3 | All results land first; a single final wave updates `coverage-check.md`. Cleaner diff but coverage stays stale. | |
| Mixed — incremental for BACKED, batch for WEAK | BACKED rows flip commit-by-commit; WEAK / re-run-needed batch in closeout. | |

**User's choice:** Incrementally (Recommended)
**Notes:** Reviewer can grep `git log -p docs/phase-2/coverage-check.md` to see Phase 3 evidence accumulating. Locked as D-27.

---

## 150-pool topology (TEST-05 prerequisite)

### Q1: Source mainnet snapshot

| Option | Description | Selected |
|--------|-------------|----------|
| Same epoch-582 snapshot | Re-parameterise generator with `N=150`. Δ% isolates pool count as the only variable. | ✓ |
| Fresh epoch (re-query Koios) | Pulls fresher data but introduces simultaneous epoch-drift + pool-count change. | |
| Stake-resample 150 from existing 100 | Doesn't actually exercise pool-count sensitivity; dilutes per-pool stake instead. | |

**User's choice:** Same epoch-582 snapshot (Recommended)
**Notes:** Test design discipline. Locked as D-28.

### Q2: Network properties for nodes 100-149

| Option | Description | Selected |
|--------|-------------|----------|
| Sample-with-jitter from existing 100 | Pick random existing node as template; perturb latencies by ±5-10% Gaussian. Documented PRNG seed. | ✓ |
| Cloned templates (no jitter) | Trivially deterministic but creates suspicious latency-cluster structure. | |
| Sampled from the full distribution | Empirical distribution sampling (KDE/histogram fit). Cleaner but heavier script. | |

**User's choice:** Sample-with-jitter (Recommended)
**Notes:** PRNG seed documented in YAML header. ±5-10% jitter SD at planner discretion (~7% midpoint reasonable). Locked as D-29.

### Q3: Total stake rescale target

| Option | Description | Selected |
|--------|-------------|----------|
| Preserve 3×10^10 | Same total as `topology-realistic-100.yaml`; pool count is the only variable. | ✓ |
| Rescale proportionally | 150-node total = 4.5×10^10. Preserves per-pool average but introduces a second variable. | |

**User's choice:** Preserve 3×10^10 (Recommended)
**Notes:** Per-pool average stake naturally drops from `3e10/100` to `3e10/150`; that's a property of going from 100 to 150 pools at fixed total. Lottery-quantization re-verified at N=150. Locked as D-30.

---

## Wave decomposition & verdict criteria

### Q1: Plan wave shape

| Option | Description | Selected |
|--------|-------------|----------|
| 3-wave: prereqs / tests / closeout | W1 = TEST-01 + TEST-02 + 150-pool topology. W2 = TEST-03/04/05/06/07a. W3 = COV-05 + coverage-check consolidation. | ✓ |
| 5-wave fine-grained | One wave per TEST-NN. Strictly sequential; slows phase. | |
| 2-wave coarse | All code/prep in W1, all runs in W2. Maximal parallelism but huge wave; brittle gating. | |

**User's choice:** 3-wave (Recommended)
**Notes:** Three plans total: 03-01 (W1), 03-02 (W2), 03-03 (W3). Locked as D-31.

### Q2: BACKED gate for TEST-03 / TEST-04

| Option | Description | Selected |
|--------|-------------|----------|
| BCa 95% CI excludes zero AND distinct-hash count = N | Conjunctive: statistical defence + COV-05 diversity. Sign-coherence reported alongside but not gating. | ✓ |
| Sign-coherence ≥ 95% — looser | BACKED if ≥95% of seeds agree on sign; CI informational. Risk: tiny absolute deltas slip through. | |
| Both — OR conjunction | Maximally inclusive; loses discipline. | |

**User's choice:** Conjunctive: CI excludes zero AND hash count = N (Recommended)
**Notes:** WEAK if CI crosses zero; re-run-needed if hashes collapse. Locked as D-32.

### Q3: Run-length / steady-state criterion (TEST-06)

| Option | Description | Selected |
|--------|-------------|----------|
| Per-seed paired comparison, `retained_value`, slots `[N/4, N/2]` vs `[N/2, 3N/4]` | Discards start-up transient; per-seed delta; STEADY iff `|median| < seed-IQR`. | ✓ |
| Pooled across seeds, `retained_value_ratio` | Pool seeds; single delta. | |
| Welfare-per-slot rolling mean over full-run halves | Simpler split (`[0, N/2]` vs `[N/2, N]`); includes start-up transient. | |

**User's choice:** Per-seed paired comparison (Recommended)
**Notes:** Per-(job, length) verdict; raise default to 4000 (or 8000) for any menu option that fails at 2000. Locked as D-33.

---

## Claude's Discretion

The following items were not raised during the discussion; CONTEXT.md `<decisions>` § Claude's Discretion captures the planner / executor defaults with rationale. Summary:

- `CiResult` struct shape (field names, serde derives) — planner picks.
- Bootstrap iteration count — default `n_bootstrap = 9999`.
- TEST-02 canonical job choice — pick a representative non-sign-flip cell.
- TEST-02 target wall-clock — aim `≤ 30 min × parallelism` per cell; N from measurement.
- `.planning/realism-tests/<name>/results.md` shape — markdown table + per-cell JSON.
- 150-pool topology jitter SD — ~7% midpoint.
- Multiplier-floor-16 companion run job mechanics — like-for-like override.
- TEST-05 aggregate verdict — single Δ% > seed-IQR keeps the register entry LIVE-going-to-DISCLOSED.
- Coverage-check `golden-sha256` column for Phase 3 rows — cite seed=1 representative with "(plus N-1 additional `sha256` values; full list in `<cell>.json`)" parenthetical.

## Deferred Ideas

- TEST-07 sub-requirements beyond TEST-07a (only TEST-07a surfaced from Phase 1; new sub-rows added only if Phase 3 execution forces them).
- Promoting Phase 3 suites to goldens-pinned (Out of Scope per PROJECT.md and D-25).
- CIP-0164 600-pool topology migration (Out of Scope per PROJECT.md; superseded by TEST-05).
- Cross-architecture CI verification (Out of Scope; phase-3 bit-deterministic on same arch only).
- Adversarial-actor regime (Out of Scope per PROJECT.md; deferred post-CIP).
- Cross-reference index automation (deferred since Phase 1).
- Refresh of `cardano-realism-audit.md` and `validity-threats.md` (Phase 4 work, DOC-01 / DOC-02).
- Anchoring of the four un-anchored controller knobs (Phase 4 / DOC-03).
- CIP author summary (Phase 5 / HAND-01).
