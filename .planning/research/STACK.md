# Methodology & Tooling Stack — CIP Evidence Base

**Project:** Phase-2 CIP Evidence Audit (subsequent milestone on `dynamic-experiment`)
**Domain:** CIP-grade evidence on top of a custom deterministic protocol simulator
**Scope:** Methodology & tooling for the audit milestone — *not* the underlying Rust simulator stack (already established, frozen).
**Researched:** 2026-05-15
**Overall confidence:** MEDIUM-HIGH (Cardano precedent is sparse but consistent; cross-ecosystem precedent — EIP-1559, ODD, FAIR4RS — is well-established)

---

## Executive recommendation

The audit milestone's deliverables (realism-risks register, multi-seed variance bands, claim-evidence mapping, coverage check) sit at the intersection of three established methodological traditions:

1. **Cardano-ecosystem precedent** — sparse but converging on a "register + targeted experiment" pattern (Leios `Impact Analysis` uses `RSK-*` risk IDs and `EXP-*` experiment IDs; this repo's `validity-threats.md` and `cardano-realism-audit.md` already prototype the pattern). **Adopt and extend these conventions** rather than inventing a new format.
2. **EIP-1559 empirical-analysis precedent** — Roughgarden (2020) for game-theoretic framing; Liu et al. (arxiv:2201.05574, CCS 2022) for the empirical-methodology template (geographically-distributed probing, three-effect breakdown: fees, waiting time, security). **Mirror the structure** for the per-mechanism welfare analysis.
3. **Empirical software engineering** — the four-fold "internal/external/construct/conclusion" validity-threats taxonomy. The repo already uses construct-validity reasoning informally in `cardano-realism-audit.md`; **formalise it** in the realism-risks register.

For statistical methodology specifically: the project's simulator is deterministic (intra-arch goldens), so the right framing is **common-random-numbers (CRN) variance reduction via Paired Seed Evaluation** (Sharma 2025, arxiv:2512.24145) — not classical Monte Carlo CIs. Use `statrs 0.18` (already in the tree) plus a paired-sample bootstrap for the variance bands; do not introduce new external statistical runtimes.

---

## The stack at a glance

| Layer | Recommendation | Rationale (one-line) |
|---|---|---|
| **Risk-register format** | Adopt Leios `RSK-*` + `EXP-*` IDs; extend with verdict column (LIVE / DORMANT / MITIGATED / DISCLOSED) | Cardano-native; the verdict column is your PROJECT.md innovation |
| **Validity-threat taxonomy** | Wohlin-style four-fold (internal/external/construct/conclusion) | Software-engineering canonical; `cardano-realism-audit.md` already uses it informally |
| **Claim-evidence mapping** | Requirements Traceability Matrix (RTM) variant — single Markdown table, columns: claim, mechanism, backing suite, backing job, backing seed, suite golden SHA-256 | Lightweight; auditable; uses artefacts that already exist |
| **Reproducibility regime** | Existing 3-layer determinism (unit goldens, `verify` subcommand, suite goldens) + version pinning via `vergen-gitcl` (already in build) + CIP-0052 machine-readable test-result convention | Builds on what's there; CIP-0052 is the local audit-disclosure standard |
| **Multi-seed variance method** | **Paired Seed Evaluation (PSE)** with N ≥ 30 seeds per (job, mechanism); paired-bootstrap BCa 95% CI on welfare deltas | PSE matches deterministic-simulator structure; BCa is the standard non-symmetric bootstrap CI |
| **Statistical compute** | `statrs 0.18` (already a dependency) for distributions + a small in-tree `paired_bootstrap.rs` module | No new runtime dependencies; Rust-only keeps the determinism contract intact |
| **Significance-test choice** | Paired-sample bootstrap CIs (preferred) or Wilcoxon signed-rank (fallback for non-normal Δ) — **not** unpaired t-tests | Variance reduction from CRN is the whole point of running paired seeds |
| **Sample-size guidance** | N=30 paired seeds as primary target; N≥20 minimum for BCa coverage; report N alongside every CI | BCa coverage degrades below N=20 (Pustejovsky simulations); 30 is the conventional asymptotic-normal threshold |
| **Figure/table conventions** | Match `family-b-results-table-2026-05-14.md` and CIP-0164 (numbered figures, "observed range by simulations" columns); add CI columns | Format already exists in-repo; CI columns are the audit-milestone extension |
| **Disclosure paragraph template** | Adopt `validity-threats.md`'s per-suite trust-rating block, generalised across the register | Pattern is proven on the project |
| **Documentation methodology** | ODD-protocol-inspired sections in the realism-risks register (Purpose / State variables / Process / Design concepts / Initialisation / Input / Submodels) — adapted for protocol-simulator context | ODD is the agent-based-model documentation standard; partial adoption is sufficient |

---

## Detailed recommendations

### 1. Risk-register format

**Adopt:** Two ID namespaces matching Leios precedent — `RSK-<name>` for risks, `EXP-<name>` for resolution experiments. Add a `Verdict` column per PROJECT.md (LIVE / DORMANT / MITIGATED / DISCLOSED).

**Schema (one row per risk):**

```
RSK-<id> | Category | Description | Impact-if-real | EXP-<id> (resolution) |
Evidence status | Verdict | Disclosure framing (1 paragraph)
```

**Categories** (Wohlin four-fold):
- **Construct validity** — does the simulator's measured quantity correspond to the real-world quantity claimed?
  Example: `RSK-net-utility-mismatch` (does simulator `retained_value` track the user-welfare claim in the CIP?)
- **Internal validity** — does the simulator's controlled change cause the observed effect, or is there a confounder?
  Example: `RSK-multiplier-floor-confound` (does `multiplier_floor=4` confound the un-reserved-vs-RB-reserved comparison?)
- **External validity** — does the simulator's result generalise to mainnet? *This is where "realism" risks live.*
  Examples: `RSK-pool-count-100`, `RSK-stake-curve-epoch-582`, `RSK-actor-model-non-adversarial`
- **Conclusion validity** — does the statistical procedure draw correct conclusions from the data?
  Example: `RSK-single-seed-claim` (do single-seed welfare claims in `family-b-results-table-2026-05-14.md` survive multi-seed variance bands?)

**Confidence:** HIGH — `RSK-*` precedent is in Leios `docs/ImpactAnalysis.md`; the four-fold taxonomy is Wohlin's canonical text and is the dominant taxonomy in empirical SE research per Verdecchia et al. (ESEM 2024).

**Anti-pattern:** Single flat list with no category. The 8+ risks anticipated in PROJECT.md will be incomprehensible without grouping. Wohlin's four-fold is the standard grouping for software empirical work; use it.

### 2. Reproducibility regime

**Keep:** The three-layer determinism the repo already operates:

| Layer | Artefact | Source |
|---|---|---|
| Unit-test goldens | `m2_two_lane.rs`, `m3_actors.rs` SHA-256 constants | `sim-rs/sim-core/src/sim/tests/` |
| Per-(job, seed) verify | `pricing_event_stream.sha256` persisted to disk; re-checked by `experiment-suite verify` | `sim-rs/sim-cli/src/runner.rs` |
| Suite-level goldens | `parameters/phase-2-sweep/suites/.goldens/<suite>.sha256` | `sim-rs/sim-cli/tests/determinism.rs` |

**Add for the audit milestone:**

1. **Version pinning per evidence artefact.** `vergen-gitcl` (already in `sim-cli/Cargo.toml`) emits the git SHA at build time. Persist the build's git SHA + simulator semver into every metrics output (`runner.rs` already has `RunSummary`; add a `simulator_version: { git_sha, cargo_version, build_time }` field). Mirrors Leios's pattern of regenerating figures with explicit simulator version (`sim-cli 1.3.0` cited in their news).
2. **Machine-readable evidence index** per CIP-0052. The coverage-check table is itself the index — emit it both as `.md` (for humans) and `.json` (for tooling). CIP-0052 explicitly mandates "machine-readable format to facilitate subsequent analysis."
3. **Reproduction recipe per cell.** Each claim-evidence row must include the exact command to re-run: `cargo run --release --bin experiment-suite -- verify parameters/phase-2-sweep/suites/<suite>.yaml`. Direct quote, not paraphrase.

**Do NOT:**
- Re-architect the determinism story (intra-arch is explicitly accepted in PROJECT.md and `.planning/codebase/CONCERNS.md`).
- Introduce Nix builds. CIP-0052 mentions Nix as "particularly suitable" but PROJECT.md constrains "no new languages or runtimes; minimise new dependencies" — git-SHA + `Cargo.lock` is sufficient.
- Cite the simulator output without simulator-version provenance. **Anti-pattern**: figures regenerated against a moving HEAD with no version trail. Leios technical-report-2 explicitly does the version-vs-version regression comparison (sim 1.3.0 vs 1.2.x); follow suit.

**Confidence:** HIGH — CIP-0052 requirements are explicit; Leios precedent confirms semver-pinning is the deployed pattern.

### 3. Multi-seed variance bands — *the central methodological choice*

PROJECT.md requirement: "re-run canonical menu-item job at ≥ N seeds (N to be calibrated by run cost); produce variance bands; verify single-seed claims … still hold."

**Recommended method:** **Paired Seed Evaluation (PSE)** with paired-bootstrap BCa confidence intervals on welfare deltas.

**Why PSE specifically:**

- The simulator is *deterministic given a seed*. Two mechanism variants run on the same seed share all stochastic confounders (lottery draws, propagation delays, actor sampling realisations). The welfare *delta* between them isolates the mechanism effect — variance reduction via common random numbers (CRN).
- This is the technique formalised by Sharma 2025 (arxiv:2512.24145) for learning-based simulators, but the underlying mathematics is Glasserman & Yao's classical CRN result. "When positive correlation exists, PSE provides strict improvements in statistical properties; when correlation is absent, paired runs can simply be treated as independent without any loss of validity — making PSE a safe default."
- For protocol-mechanism comparisons under congestion, positive correlation is overwhelming (same congestion event, same actor draws, same lottery → outcomes co-move). PSE is strictly better than independent-seed evaluation; there is no downside to choosing it.

**Procedure:**

1. Pick the canonical seed set: `seeds = 1..=N`. N=30 is the recommended primary target (Pustejovsky shows BCa coverage is at-nominal for N≥20; classical asymptotic-normal threshold is 30; suite-runner cost is linear so doubling N doubles wall-clock).
2. For each pair of mechanisms (M_a, M_b) under comparison, run both at every seed `s ∈ 1..=N`. The suite runner already supports this — every existing suite is keyed by (job_name, seed_string).
3. Compute paired deltas `Δ_s = welfare(M_a, s) − welfare(M_b, s)` for each seed.
4. Compute the **paired-sample bootstrap BCa 95% CI** on the mean of `Δ_s`:
   - Resample `Δ_s` values (not raw welfares) with replacement, B=10,000 bootstrap replicates.
   - Compute BCa interval per Efron (1987); use `stats-ci` crate or implement directly against `statrs::distribution::Normal` for the bias/acceleration constants.
5. Report: mean Δ, BCa 95% CI, N, and the bootstrap-replicate count. Reject the "mechanisms differ" claim only if 0 ∉ CI.

**Sample-size guidance:**

| N seeds | Use case | Coverage notes |
|---|---|---|
| ≥ 30 | Primary canonical menu-item claims | At-nominal BCa coverage; conventional asymptotic threshold |
| 20-29 | Secondary comparisons; budget-constrained | BCa coverage acceptable per Pustejovsky simulations |
| 10-19 | Smoke / sanity only — **not for CIP claims** | Coverage degraded; CI too narrow; misleads readers |
| < 10 | Single anecdote; document as such | Anti-pattern for any quantitative claim |

**Tooling:**

| Layer | Choice | Rationale |
|---|---|---|
| Distribution functions, percentile lookups | `statrs 0.18` | Already in `sim-cli/Cargo.toml`; no new dependency |
| Bootstrap mechanics | In-tree `sim-cli/src/metrics/paired_bootstrap.rs` (new module, ~150 LoC) | Avoids `stats-ci`'s pulled-in deps; bootstrap math is trivial; keeps determinism contract under repo control |
| Seed orchestration | Existing `experiment-suite` `--seeds N` (already supports multi-seed) | No new tooling; the runner is the right layer |
| Reporting | New `metrics_ci.json` per (suite, mechanism-pair) | Lives next to existing `metrics_comparison.txt` |

**Confidence:** HIGH for PSE (Sharma 2025 is the recent definitive treatment; Glasserman & Yao is the classical foundation; both unambiguous on the variance-reduction guarantee). MEDIUM for N=30 specifically (the "right" N depends on compute budget; 30 is conventional, not derived from this project's effect sizes).

**Anti-patterns:**

- **Independent-seed t-tests across mechanisms.** Discards the CRN-induced variance reduction. Strictly inferior to paired analysis when the simulator is deterministic. Per Sharma 2025: "standard independent evaluation designs fail to exploit shared sources of randomness across alternatives."
- **Percentile bootstrap on unscaled metrics.** BCa is preferred over plain percentile when the underlying distribution is asymmetric; welfare metrics under congestion are often heavy-tailed. Use BCa as the default; cross-check with percentile only as sanity.
- **N=5 or N=10 with hidden CI width.** Pustejovsky: "Bootstrap CIs are extremely optimistic (too narrow) with sample sizes like n=5 (coverage of a 95% interval is 81-83%) and remain optimistic even at n=20." Report N visibly; refuse claims at N<20.
- **Summary-stats-without-CIs.** Per PROJECT.md quality gate explicitly. Every numeric claim in the evidence base must be accompanied by either (a) a CI, or (b) explicit disclosure that it is a single-seed observation.

### 4. Claim-evidence mapping (the coverage check)

**Format:** Single Markdown table (committed at `.planning/coverage-check.md` or similar), one row per CIP-facing claim.

**Schema:**

```
| Claim ID | Claim text (1 sentence) | Mechanism | Backing suite | Backing job | Seeds | CI/method | Suite golden SHA | Verdict |
```

**Verdict values:** `BACKED` (claim has matching simulator job at sufficient N), `WEAK` (single-seed only or low N), `UNBACKED` (no simulator job covers this claim), `OUT-OF-SCOPE` (claim is not simulator-derivable; CIP must justify another way).

**Convention:**

- Claim IDs are stable across CIP drafts: `CLM-priority-tail-latency`, `CLM-standard-floor-preserved`, etc. The user-author of the CIP can cite `[CLM-priority-tail-latency]` and the evidence base resolves the citation.
- Backing job is the YAML filename + the (job_name, seed_string) tuple that this golden hashes — concrete, unambiguous, reproducible.
- Suite golden SHA is the contents of `parameters/phase-2-sweep/suites/.goldens/<suite>.sha256` at the commit the CIP cites. Re-pinning when goldens regenerate.

**Confidence:** HIGH — RTM is the canonical compliance-traceability format (DO-178C, ISO 26262, FDA); Markdown table is the right human-readable shape. The Leios CIP-0164 / technical-report-2 separation (claims in CIP, evidence in technical report) demonstrates the precedent of an external evidence base feeding a CIP.

**Anti-pattern:** Burying claim-evidence mapping in prose ("see Figure 8 and Table 5"). Forces every reader to re-derive the mapping. Make it a table; expose `UNBACKED` rows as audit findings.

### 5. Disclosure-paragraph template

**Adopt:** Per-suite disclosure block from `docs/phase-2/validity-threats.md` — already exists, already trusted internally.

**Adapt for the realism-risks register:**

```markdown
### RSK-<id>: <name>
**Category:** External validity (Wohlin)
**What goes wrong:** <consequence-if-real>
**Current evidence:** <observed simulator behaviour or upstream constraint>
**Verdict:** LIVE | DORMANT | MITIGATED | DISCLOSED
**Resolution:** <EXP-<id> reference, OR disclosure paragraph for the CIP>
**Disclosure framing** (CIP-ready paragraph):

> The phase-2 simulator [does X / assumes Y / lacks Z]. Empirical
> validation [against ... / via EXP-<id>] establishes [bound /
> sensitivity]. Readers should interpret [welfare claims / latency
> claims / etc] accordingly.
```

The "Disclosure framing" sub-section is **CIP-ready prose**: the user-author copies it into the CIP's Limitations / Methodology section verbatim if they choose. This is the audit milestone's deliverable shape — paragraph-level, not bullet-level, because CIPs are prose documents.

**Confidence:** HIGH — `validity-threats.md` validates the template on this exact project.

### 6. Documentation methodology (ODD-inspired sections)

Partial adoption of the **ODD protocol** (Grimm et al. 2020 update, JASSS 23(2):7) for the simulator's methodology disclosure. ODD's seven elements map naturally to the audit deliverables:

| ODD element | Maps to | Existing artefact |
|---|---|---|
| Purpose | "What this is" in PROJECT.md | Already present |
| State variables and scales | `CLAUDE.md` mechanism abstractions section | Already present |
| Process overview and scheduling | `mechanism-design.md` controller cadence + linear-Leios flow | Already present |
| Design concepts | Chain-derivation rationale, EIP-1559 faithfulness | `family-b-decision-2026-05-14.md`, spike 007 |
| Initialization | Topology + demand YAMLs | `parameters/phase-2-sweep/` |
| Input | Per-suite YAML + demand profiles | Same |
| Submodels | `MempoolGate`, `ActorComponent`, `Eip1559Pricing`, etc. | `tx_pricing/`, `tx_actors.rs`, `mempool_gate.rs` |

**Action:** Audit milestone should produce a single "Methodology Overview" doc (`.planning/methodology-overview.md` or as an appendix to the realism-risks register) that ODD-indexes the existing artefacts. ~1-page table mapping ODD elements to in-repo files. **Do not rewrite** the existing artefacts — index them.

**Confidence:** MEDIUM — ODD is the agent-based-modelling community standard but is broader than necessary. The point of citing it is not to follow it religiously but to give readers a recognisable structural cue. ResearchGate review (Grimm et al. 2020): "Limitations of the ODD protocol include the limited availability of guidance on how to use it, the length of ODD documents." Use it as a structural reference, not a checklist.

### 7. CIP-author-facing artefact list

The user authors the CIP. The audit milestone's evidence base produces these artefacts the CIP cites:

| Artefact | Path | CIP citation form |
|---|---|---|
| Realism-risks register | `.planning/realism-risks-register.md` | "See [RSK-<id>] in the project realism-risks register" |
| Coverage-check table | `.planning/coverage-check.md` | "See [CLM-<id>] in the project coverage check" |
| Multi-seed variance results | `.planning/multi-seed-variance.md` + per-suite JSON | Inline 95% CIs with N reported |
| Methodology overview | `.planning/methodology-overview.md` | "Methodology summary: …; full ODD-indexed methodology at …" |
| Updated `cardano-realism-audit.md` | `docs/phase-2/cardano-realism-audit.md` | "Cardano-realism audit at …" — already in the CIP-able set |
| Suite goldens (existing) | `parameters/phase-2-sweep/suites/.goldens/` | "Reproducible against commit `<git-sha>`; suite goldens at …" |
| Disclosure paragraphs | Inline in realism-risks register | Paste into CIP Limitations section verbatim |

**Confidence:** HIGH — the list mirrors PROJECT.md's Active requirements directly.

---

## Anti-patterns (explicit)

| Anti-pattern | Why bad | Instead |
|---|---|---|
| Single-seed claims without explicit `N=1` flag | Misleads CIP readers; violates conclusion validity | Always report N; refuse quantitative claims at N<20 |
| Independent-seed unpaired t-tests | Discards CRN variance reduction; statistically suboptimal | Paired bootstrap on Δ; PSE per Sharma 2025 |
| Bootstrap CI without bias/skew correction | Plain percentile under-covers asymmetric distributions | BCa per Efron 1987; cross-check with percentile |
| Risk register as unstructured prose | Cannot be audited; cannot be CIP-cited | RSK-* IDs + RTM table + verdict column |
| Mapping claims to figures, not jobs | Figures regenerate; jobs+seeds+SHAs do not | Cite (suite, job, seed, golden SHA) tuple, not figure number |
| Re-running figures against moving HEAD | Loses provenance; non-reproducible | Pin every figure to simulator-version `vergen-gitcl` SHA |
| "We tried X seeds and it looks the same" | Cannot be audited; epistemic claim with no quantified backing | Report mean Δ, 95% BCa CI, N, bootstrap replicate count |
| Disclosure as bullet list of caveats | CIP reader cannot copy-paste into CIP | Disclosure paragraphs, prose, CIP-ready |
| Coverage check as narrative ("most claims are backed") | Forces re-derivation; hides gaps | Table with BACKED/WEAK/UNBACKED/OUT-OF-SCOPE verdict per claim |
| New external runtime (Python stats notebook for analysis) | Splits the determinism contract; new dependency | In-tree Rust analysis module; same `cargo test` regime |
| ODD-protocol slavish adoption | ODD is verbose; overkill for protocol-simulator audit | Use ODD as a structural cue only; index existing artefacts |

---

## Installation / dependencies

**No new Rust dependencies needed.** Everything maps to the existing dep tree:

| Need | Crate | Already in tree? |
|---|---|---|
| Distribution functions | `statrs 0.18` | ✓ `sim-cli/Cargo.toml:38` |
| Hashing (golden SHA-256) | `sha2 0.10` | ✓ `sim-cli/Cargo.toml:36` |
| Git-SHA at build time | `vergen-gitcl 1` | ✓ `sim-cli/Cargo.toml:49` |
| YAML config | `serde_yaml 0.9` | ✓ `sim-cli/Cargo.toml:35` |
| JSON output | `serde_json 1` | ✓ `sim-cli/Cargo.toml:34` |
| Random number generation | `rand 0.9` | ✓ `sim-cli/Cargo.toml:32` |
| Bootstrap implementation | — (new in-tree module, ~150 LoC) | To add: `sim-cli/src/metrics/paired_bootstrap.rs` |

**Optional new dep** (consider rejecting): `stats-ci` crate for bootstrap CIs. Pros: bias-correction code is non-trivial; reusing tested code is sensible. Cons: pulls in additional transitive deps; harder to audit; project convention is minimal-deps. **Recommend:** implement BCa in-tree; ~50-150 LoC; bootstrap math is genuinely simple; matches the project's "no new deps" preference.

**Confidence:** HIGH — every requirement maps to an existing dep or trivial in-tree code.

---

## Alternatives considered (and rejected)

| Choice | Recommended | Alternative | Why not |
|---|---|---|---|
| Variance method | Paired-bootstrap BCa | Independent-seed t-test | Discards CRN variance reduction; suboptimal for deterministic simulator |
| Variance method | Paired-bootstrap BCa | Wilcoxon signed-rank | Acceptable fallback; loses magnitude information; use only if Δ distribution is pathological |
| Sample-size N | 30 (primary), 20 (minimum) | N=5–10 | BCa coverage well below nominal; misleads readers; standard threshold for asymptotic claims is 30 |
| Risk-register taxonomy | Wohlin four-fold | Flat list or ad-hoc categories | 8+ risks anticipated; flat list unreadable; Wohlin is the SE-research canonical |
| Claim-evidence format | RTM table | Prose in CIP | Cannot be audited; cannot be machine-checked; CIP-0052 demands machine-readable |
| Methodology doc | ODD-indexed appendix | Full ODD-protocol rewrite | ODD's verbosity is acknowledged limitation; partial adoption preserves recognisability without bloat |
| Build-time provenance | `vergen-gitcl` (already integrated) | Nix flake | "No new runtimes" constraint in PROJECT.md; `vergen` is sufficient and already there |
| Statistical compute | In-tree Rust module + `statrs` | Python notebook with `scipy.stats.bootstrap` | Splits the determinism contract; introduces new runtime; project is Rust-only by mandate |
| Statistical compute | In-tree Rust module + `statrs` | `stats-ci` crate dependency | Acceptable but introduces transitive deps; in-tree keeps audit surface minimal |
| Disclosure format | Per-RSK paragraph block, CIP-ready | Bullet caveats | User-author needs prose to paste; bullets force rewrites |

---

## Sources

Cardano ecosystem:

- [CIP-0001 — CIP Process](https://cips.cardano.org/cip/CIP-0001) — CIP structure: Preamble + required sections; "complete and unambiguous design" requirement.
- [CIP-0052 — Audit best practice guidelines](https://cips.cardano.org/cip/CIP-0052) — Reproducibility requirements: "machine-readable format", "identity, configuration and version of all test components", "checksum and version".
- [CIP-0164 — Ouroboros Linear Leios](https://cips.cardano.org/cip/CIP-0164) — Numbered figures, performance metrics table, "Observed Range by Simulations" columns, version pinning to `V1.0` git tags, GitHub-hosted SVG figures.
- [Ouroboros Leios CIP publication news](https://leios.cardano-scaling.org/news/tags/cip-publication/) — Cross-implementation validation (Haskell ↔ Rust simulators), regression experiment patterns, version-pinned regeneration (`sim-cli 1.3.0`).
- [Ouroboros Leios `docs/technical-report-2.md`](https://github.com/input-output-hk/ouroboros-leios/blob/main/docs/technical-report-2.md) — Six-scenario progression structure; comparative analysis; provisional-evidence framing. *Notable absence:* no confidence intervals or multi-seed bands — gap your audit milestone fills.
- [Ouroboros Leios `docs/ImpactAnalysis.md`](https://github.com/input-output-hk/ouroboros-leios/blob/main/docs/ImpactAnalysis.md) — `RSK-*` risk IDs (LeiosPraosContentionGC, LeiosDiskBandwidth, etc.); `EXP-*` experiment IDs; "prototypes rather than simulations" forward-looking framing.

EIP-1559 / blockchain mechanism precedent:

- [Roughgarden 2020 — Transaction Fee Mechanism Design for Ethereum (arxiv:2012.00854)](https://arxiv.org/abs/2012.00854) — Game-theoretic framing: MMIC, OCA-proofness, DSIC properties. Sets the standard for "what a fee mechanism paper looks like."
- [Liu et al. 2022 — Empirical Analysis of EIP-1559 (arxiv:2201.05574, CCS 2022)](https://arxiv.org/abs/2201.05574) — Three-effect breakdown (fees, waiting time, security); geographically-distributed probing methodology; modified Geth clients connecting to 1,000 peers.
- [abm1559 EIP-1559 simulation notebook](https://ethereum.github.io/abm1559/notebooks/eip1559.html) — Agent-based simulation pedagogy: simple-demand → iteratively-richer scenarios.

Statistical methodology:

- [Sharma 2025 — Paired Seed Evaluation: Statistical Reliability for Learning-Based Simulators (arxiv:2512.24145)](https://arxiv.org/abs/2512.24145) — PSE formalisation; "matched realisations of stochastic components", strict variance reduction under positive correlation, no-worse-than-independent guarantee.
- [Glasserman & Yao — Some Guidelines and Guarantees for Common Random Numbers](https://business.columbia.edu/sites/default/files-efs/pubfiles/4261/glasserman_yao_guidelines.pdf) — Classical CRN theory; foundation of PSE.
- [Pustejovsky — Bootstrap CI variations](https://jepusto.com/posts/Bootstrap-CI-variations/) — BCa coverage simulations across N; "BCa CIs with paired samples are also nominal across a wide range of values except at very small sample sizes (n = 10 pairs)."
- [Bootstrap CIs: A comparative simulation study (arxiv:2404.12967)](https://arxiv.org/html/2404.12967v1) — Method comparison; recommends BCa when distribution is skewed.

Empirical software engineering / risk frameworks:

- [Wohlin et al. — Threats to Validity in Empirical SE Research (Feldt & Magazinius 2010 survey)](https://www.cse.chalmers.se/~feldt/publications/feldt_2010_validity_threats_in_ese_initial_survey.pdf) — Four-fold internal/external/construct/conclusion taxonomy.
- [Verdecchia et al. — Threats to Validity in SE Research: A Critical Reflection (ESEM 2024)](https://robertoverdecchia.github.io/papers/ESEM_2024.pdf) — Current state of TTV practice in SE; argues against the "afterthought" treatment that's currently dominant.
- [RFC 3552 — Guidelines for Writing RFC Text on Security Considerations](https://datatracker.ietf.org/doc/html/rfc3552) — IETF convention for threat-model framing; due-diligence framing for known/foreseeable risks. Useful as cross-ecosystem reference.

Documentation conventions:

- [Grimm et al. 2020 — ODD Protocol Second Update (JASSS 23(2):7)](https://www.jasss.org/23/2/7.html) — Overview / Design concepts / Details structure for agent-based models; seven-element schema (Purpose, State variables, Process, Design concepts, Initialisation, Input, Submodels).
- [FAIR4RS Principles (Zenodo 6623556)](https://zenodo.org/records/6623556) — FAIR principles adapted for research software: findable, accessible, interoperable, reusable. Frames reproducibility-via-versioning as a community standard.
- [Requirements Traceability Matrix overview (Perforce ALM)](https://www.perforce.com/resources/alm/requirements-traceability-matrix) — Forward/backward/bidirectional/horizontal traceability; format for claim-evidence mapping.

Rust tooling:

- [`statrs` documentation](https://docs.rs/statrs/) — Distributions and statistical functions for Rust.
- [`stats-ci` crate (xdefago/stats-ci)](https://github.com/xdefago/stats-ci) — Pure-Rust bootstrap and confidence-interval implementation. Considered and not adopted (prefer in-tree module per project's minimal-deps norm).

In-repo references:

- `/home/will/git/arc-tiered-pricing/CLAUDE.md` — Determinism scope, numeric-representation contract, suite structure.
- `/home/will/git/arc-tiered-pricing/.planning/PROJECT.md` — Audit milestone scope, deliverables, constraints.
- `/home/will/git/arc-tiered-pricing/docs/phase-2/mechanism-design.md` — Mechanism spec.
- `/home/will/git/arc-tiered-pricing/docs/phase-2/validity-threats.md` — Per-claim trust-rating precedent already in the repo.
- `/home/will/git/arc-tiered-pricing/docs/phase-2/cardano-realism-audit.md` — Existing realism audit; refresh target per PROJECT.md.
- `/home/will/git/arc-tiered-pricing/.planning/family-b-decision-2026-05-14.md` — Mechanism-decision audit-trail precedent.
- `/home/will/git/arc-tiered-pricing/.planning/family-b-results-table-2026-05-14.md` — Single-seed claims that multi-seed variance bands must verify.
- `/home/will/git/arc-tiered-pricing/.planning/REVIEW.md` — Fix Status table format (good precedent for the verdict-column shape).
- `/home/will/git/arc-tiered-pricing/sim-rs/sim-cli/Cargo.toml` — Current dep tree (no new Rust deps needed).
