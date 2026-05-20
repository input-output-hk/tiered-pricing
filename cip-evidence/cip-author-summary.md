# Cardano Improvement Proposal (CIP) Author Summary — Phase-2 Evidence Base

**Status:** Post-Phase-5 close; citable git tag `phase-2-cip-evidence-v1` (tag pending: user-applied per Plan 05-03 Task 3 — see §"Pinned references" below).
**Scope:** Single paste-guide artefact for the Cardano Improvement Proposal (CIP) responding to Cardano Problem Statement (CPS)-0023 ("Urgency Signaling"). Maps each CIP section to its source artefact and the specific paragraphs / rows / values to paste.
**Identifier convention:** Realism Risk identifier (RSK)-NN identifiers in `audit-documents/realism-risks-register.md` (24 entries, all DISCLOSED at Phase 5 close). Claim identifier (CLM)-NN identifiers in `audit-documents/coverage-check.md` (55 rows). Experiment identifier (EXP)-NN identifiers in the register's EXP-NN column. All three append-only — never renumbered.
**Verdict vocabulary:** Register verdicts: LIVE / DORMANT / MITIGATED / DISCLOSED (post-Phase-5: **0 LIVE + 24 DISCLOSED + 0 MITIGATED + 0 DORMANT**). Coverage-check verdicts: BACKED / WEAK / UNBACKED / OUT-OF-SCOPE. The two vocabularies are intentionally distinct.

**Abbreviations on first use** (per `CLAUDE.md` §"Conventions / gotchas"): Cardano Improvement Proposal (CIP), Cardano Problem Statement (CPS), Overview Design-concepts Details (ODD), Realism Risk identifier (RSK), Claim identifier (CLM), Experiment identifier (EXP), Bias-corrected and accelerated (BCa) bootstrap, Confidence Interval (CI), Inter-Quartile Range (IQR), Ranking Block (RB), Endorser Block (EB), Ethereum Improvement Proposal 1559 (EIP-1559), Secure Hash Algorithm 256-bit (SHA-256), Advances in Financial Technologies (AFT), Conference on Computer and Communications Security (CCS), Symposium on Discrete Algorithms (SODA), Maximum Extractable Value (MEV), Uniform Resource Locator (URL), Yet Another Markup Language (YAML), Stake-Pool Operator (SPO), Continuous Integration (CI-pipeline), Institute of Electrical and Electronics Engineers (IEEE), Advanced RISC Machine (ARM), Number of seeds (N).

## Reading guide

The summary is hybrid per `.planning/phases/05-handoff/05-CONTEXT.md` decision D-44:

1. **Top — Paste-target table.** One-page map: CIP section → source artefact → paste content + inline-vs-reference treatment.
2. **Middle — Per-CIP-section recommendations.** Five subsections, one per CIP section (Methodology, Calibration, Trust matrix, Evidence, Limitations). Per D-45, the Evidence section inlines 4–8 headline claims verbatim; the Limitations section inlines the substrate-scope umbrella + 3–4 highest-priority disclosure-paragraphs verbatim. All other content is reference-only with RSK-NN / CLM-NN / path / line range pointers.
3. **Bottom — Pinned references.** Tag name, commit Secure Hash Algorithm 256-bit (SHA-256), Cardano mainnet epoch-582 stake snapshot reference, consistency-verification script + report paths, and the embedded tag-message draft for the user-executed `git tag` step.

The CIP author may freely edit inline content. Reference-only items should be pasted from their source-of-truth without modification (the source artefacts are stable at the `phase-2-cip-evidence-v1` tag; subsequent edits require a new tagged version).

## Paste-target table

| CIP Section | Source Artefact | Paste Content | Inline / Reference |
|---|---|---|---|
| Methodology | `audit-documents/methodology-overview.md` | Full document by repo Uniform Resource Locator (URL) — Overview, Design concepts, Details (ODD) seven-element index + per-element prose + worked example tracing `menu_unreserved_priority_only_static_x4` seed=1 through the seven ODD elements | Reference |
| Calibration | `audit-documents/cardano-realism-audit.md` | Seventeen `(value, source, date-retrieved)` triples in §"Topology and actor model", §"Pricing-controller calibration", §"What lines up with mainnet", §"Ranking-block (RB) cadence" | Reference (CIP names specific values; cites the audit by repo URL) |
| Trust matrix | `audit-documents/validity-threats.md` | Aggregate trust matrix — **2 HIGH + 13 MEDIUM + 4 LOW + 0 UNRESOLVED** across 19 per-suite blocks; each block carries `Related RSK:` + `Related CLM:` cross-references | Reference (CIP cites the aggregate + per-suite breakdown by name) |
| Evidence | `audit-documents/coverage-check.md` | Headline CIP claims backed by specific CLM-NN rows (see §"CIP Section: Evidence" §"Headline CIP claim list" below for the row-to-claim mapping); supporting CLM rows referenced by identifier + line range | Mixed — 4–8 headline claims inline; long-tail CLM rows reference-only |
| Limitations | `audit-documents/realism-risks-register.md` | 24 DISCLOSED `disclosure-paragraph` blocks pastable verbatim; substrate-scope umbrella + top-4 paste-order paragraphs inlined below in §"CIP Section: Limitations" §"Limitations paste order"; long-tail paragraphs referenced by RSK-NN identifier + path + line range | Mixed |
| Evidence — user experience | `audit-documents/latency-by-urgency.md` | Per-mechanism observed inclusion latency and inclusion rate across 11 urgency-tagged user classes at N=20 seeds on `sundaeswap_moderate × multiplier_floor=4`; refines the welfare-delta findings with the user-class attribution single-lane EIP-1559 vs un-reserved vs RB-reserved mechanisms produce | Reference (one anomaly flagged pending team review; see source doc §"Anomaly to flag") |

## CIP Section: Methodology

**Source-of-truth:** `audit-documents/methodology-overview.md`.

**Paste order:** The methodology overview is a one-page Overview, Design concepts, Details (ODD) index with per-element prose and a worked example. The CIP cites it by repo Uniform Resource Locator (URL) rather than pasting content. The CIP's Methodology section should consist of: (a) a one-paragraph reference to the repo URL of `methodology-overview.md`; (b) a one-paragraph high-level summary of the seven ODD elements (Purpose, State variables, Process overview, Design concepts, Initialisation, Input data, Submodels); (c) a one-sentence reference to the worked example tracing the canonical job-seed pair (`menu_unreserved_priority_only_static_x4` seed=1) through the seven elements.

**Why ODD:** the Overview, Design concepts, Details protocol (Grimm et al. 2006/2010, revised 2020) is the standard reporting template for agent-based simulation models in ecology and economics. Adopting ODD for the phase-2 simulator's methodology lets a Cardano Improvement Proposal (CIP) reviewer with simulation-modelling background map the present work onto an established methodology rather than re-deriving the simulator's structure from prose.

**Reference-only:** the full methodology document at `audit-documents/methodology-overview.md` (260 lines). The CIP should cite this document by repo URL at the post-`phase-2-cip-evidence-v1`-tag commit.

## CIP Section: Calibration

**Source-of-truth:** `audit-documents/cardano-realism-audit.md`.

**Paste order:** The audit carries seventeen `(value, source, date-retrieved)` triples in §"Topology and actor model" (calibration of the realistic-100 topology against epoch-582 mainnet), §"Pricing-controller calibration" (the four controller knobs: window-length 32 ANCHORED via Reijsbergen / Leonardos / Liu citations; multiplier-floor 4, multiplier-floor 16, lane-signal-source DISCLOSED per Plan 04-01), §"What lines up with mainnet" (the Ethereum Improvement Proposal 1559 (EIP-1559) core parameters that match Ethereum mainnet bit-exact: D=8, target=0.5, per-priced-block update cadence), and §"RB cadence" (ranking-block (RB) cadence calibration). The CIP cites specific calibration values by triple but does not paste the full audit; it links to the audit by repo Uniform Resource Locator (URL) and the §"Recommended disclosure statements" subsection.

**What "anchored" means here:** a calibration value is *anchored* when it cites a deployed-system datum or peer-reviewed academic source. Of the seventeen triples in the audit, the ranking-block (RB) cadence + the EIP-1559 core parameters (D=8, target=0.5, per-priced-block update cadence) + window length 32 are anchored; the multiplier floor 4, multiplier floor 16, and lane-signal-source choices carry "conditional on X" disclosure paragraphs per `RSK-un-anchored-controller-knobs`. The CIP should report the anchored-vs-disclosed boundary explicitly — `audit-documents/cardano-realism-audit.md` §"Pricing-controller calibration" is the source-of-truth for which knobs lie on which side of the boundary.

**Reference-only:** the calibration-source-of-truth at `audit-documents/cardano-realism-audit.md` (500 lines). The CIP should cite this document by repo URL + name specific values by triple from §"Topology and actor model" and §"Pricing-controller calibration".

## CIP Section: Trust matrix

**Source-of-truth:** `audit-documents/validity-threats.md`.

**Paste order:** The validity-threats document carries 19 per-suite trust blocks plus an aggregate summary. The CIP's Trust matrix section should consist of: (a) one paragraph citing the aggregate **2 HIGH + 13 MEDIUM + 4 LOW + 0 UNRESOLVED**; (b) a one-row-per-suite table naming each goldens-pinned suite (the seven canonical suites: `phase-2-eip1559-robustness`, `phase-2-eip1559-smoothing`, `phase-2-priority-only-rb-reserved`, `phase-2-priority-only-unreserved`, `phase-2-rb-scarcity`, `phase-2-two-lane-both-dynamic`, `phase-2-urgency-inversion`) with its trust verdict and one-sentence rationale; (c) a reference to the validity-threats document for the full 19-block per-suite breakdown.

**What "trust" means here:** each suite's trust verdict reflects the conjunction of (i) the demand-profile's empirical anchoring (sundaeswap is the most-anchored, `paper_like_*` profiles are stylised reference loads), (ii) the calibration knobs the suite exercises (multiplier-floor sweep coverage, window-length sweep coverage), (iii) the conclusion-validity bound from the three-seed default, and (iv) any Realism Risk identifier (RSK)-NN entries that scope the suite's findings. The trust framework is documented in `audit-documents/validity-threats.md` §"Trust framework". Each per-suite block carries `Related RSK:` + `Related CLM:` cross-references introduced by Plan 04-05; the CIP can derive any suite's load-bearing scope by following those cross-references.

**Reference-only:** the trust-matrix-source-of-truth at `audit-documents/validity-threats.md` (850 lines). The CIP should cite the aggregate verdict + name suites by slug + reference the per-suite cross-references rather than pasting the full document.

## CIP Section: Evidence

**Source-of-truth:** `audit-documents/coverage-check.md` + the Phase 3 evidence consolidations at `.planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md` + `test-results/multi-seed-variance/results.md` + `test-results/multiplier-floor-16-companion/results.md` + `test-results/hash-diversity-gate/results.md`.

**Paste order:** The CIP's Evidence section should consist of: (a) the headline-claim list (inline below; 6 claims at this writing); (b) a one-paragraph reference to `audit-documents/coverage-check.md` for the full 55-row coverage matrix; (c) per-claim citations of the specific CLM-NN rows backing each headline.

### Headline CIP claim list

The headline claims below are derived from Phase 3 / Phase 4 evidence per `.planning/phases/05-handoff/05-CONTEXT.md` decision D-46, with each claim's backing CLM-NN row(s) named and its Bias-corrected and accelerated (BCa) 95% Confidence Interval (CI) numerics quoted. The CIP author may add or remove headlines before submission; the minimum bar is the four mechanism-ordering claims (Headlines 1, 2, 3, 4).

> **Headline Claim 1:** "Un-reserved two-lane mechanisms materially outperform single-lane Ethereum Improvement Proposal 1559 (EIP-1559) on welfare at `multiplier_floor = 4` under `sundaeswap_moderate` demand at Number of seeds (N) = 20 seeds, with Bias-corrected and accelerated (BCa) 95% Confidence Interval (CI) excluding zero."
>
> **Backed by:**
> - **CLM-07** (`priority-only-un-reserved`): BCa 95% CI = `[+4.28e+09, +8.49e+09]`; median Δ retained_value = `+6.66e+09`; sign-coherence = `0.90`; distinct-hash = `20/20`; backing-job = `menu_unreserved_priority_only_static_x4` in `sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml`.
> - **CLM-09** (`both-dynamic-un-partitioned`): BCa 95% CI = `[+5.65e+09, +1.09e+10]`; median Δ = `+7.95e+09`; sign-coherence = `0.90`; distinct-hash = `20/20`; backing-job = `menu_unreserved_both_dynamic_x4` in `sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml`.
> - **Paired baseline:** single-lane EIP-1559 control `control_eip1559_d8_t50_w32` (via CLM-05; zero by construction; 20/20 distinct hashes).
>
> **Source-of-truth:** `test-results/multi-seed-variance/results.md` §"TEST-04 canonical menu-item variance bands" + `audit-documents/coverage-check.md` rows CLM-05, CLM-07, CLM-09.

> **Headline Claim 2:** "Ranking-block-reserved (RB-reserved) two-lane mechanisms underperform single-lane EIP-1559 under the same calibration; this REFUTES the pre-Phase-3 single-seed framing that 'two-lane mechanisms outperform single-lane EIP-1559', which holds only for the un-reserved variants."
>
> **Backed by:**
> - **CLM-06** (`priority-only-RB-reserved`): BCa 95% CI = `[-6.02e+09, -1.00e+09]`; median Δ = `-4.15e+09`; sign-coherence = `0.65`; distinct-hash = `20/20`; backing-job = `menu_rb_reserved_priority_only_static_x4` in `sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml`. (Coverage-check verdict `WEAK` predates Phase 3; the Phase 3 N=20 CI excludes zero — the CIP author should cite the Phase 3 evidence at face value.)
> - **CLM-08** (`both-dynamic-partitioned`): BCa 95% CI = `[-5.95e+09, -8.87e+08]`; median Δ = `-4.15e+09`; sign-coherence = `0.65`; distinct-hash = `20/20`; backing-job = `menu_rb_reserved_both_dynamic_x4` in `sim-rs/parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml`. (Same coverage-check-vs-Phase-3 caveat as CLM-06.)
>
> **Source-of-truth:** `test-results/multi-seed-variance/results.md` §"TEST-04 canonical menu-item variance bands" + `audit-documents/coverage-check.md` rows CLM-06, CLM-08.

> **Headline Claim 3:** "The `multiplier_floor = 4` calibration is regime-dependent: at the spec default `multiplier_floor = 16` the `phase-2-rb-scarcity` welfare finding inverts (`standard dominates welfare; RB scarcity mostly invisible` → `priority captures everything; total welfare collapses 93–98%`) and the `phase-2-urgency-inversion` finding weakly reverses (`mispriced > correctly priced` → `correctly priced > mispriced by ~13%`)."
>
> **Backed by:**
> - The TEST-07a multiplier-floor-16 companion run at Number of seeds (N) = 5 per `test-results/multiplier-floor-16-companion/results.md` produced these per-cell comparisons: `rb_baseline` Δ% = −93%; `rb_reduced_half` Δ% = −96%; `rb_reduced_third` Δ% = −97%; `rb_reduced_quarter` Δ% = −98%; `urgency_mispriced_high_urgency` floor=4 = 3.3e+09 → floor=16 = 5.4e+09 (with `urgency_correctly_priced` rising more, weakly reversing the floor=4 ordering).
> - **`RSK-multiplier-floor-4-suite-coverage`** disclosure-paragraph (lines ~282–295 in `audit-documents/realism-risks-register.md`) is the CIP-pasteable narrative for the regime-dependence finding.
>
> **Source-of-truth:** `test-results/multiplier-floor-16-companion/results.md` + `audit-documents/realism-risks-register.md` §"RSK-multiplier-floor-4-suite-coverage".

> **Headline Claim 4:** "Partitioned (both-dynamic) and RB-reserved (priority-only-static) mechanisms produce indistinguishable welfare at `sundaeswap_moderate × multiplier_floor = 4`; this cross-arm duplicate-job artefact replicates at Number of seeds (N) = 20 because the standard-lane controller never drifts off the multiplier floor under this calibration."
>
> **Backed by:**
> - **CLM-06** and **CLM-08**: identical median Δ = `-4.15e+09` and overlapping BCa CIs (`[-6.02e+09, -1.00e+09]` vs `[-5.95e+09, -8.87e+08]`) at N=20 seeds. The underlying mechanism — standard controller pinned at the floor → partitioned-both-dynamic collapses to priority-only-static — is the same mechanism that produces the floor=16 cross-cell Secure Hash Algorithm 256-bit (SHA-256) identity observed in TEST-07a between `rb_scarcity_x16_baseline` and `urgency_inversion_x16_correctly_priced`.
>
> **Source-of-truth:** `test-results/multi-seed-variance/results.md` §"TEST-04" cross-cell pattern + `test-results/multiplier-floor-16-companion/results.md` §"Cross-cell SHA-256 identity at seeds 1+2".

> **Headline Claim 5:** "Single-lane EIP-1559 sign-flip cells (`d4_t50_w32`, `d8_t25_w32`) under the Family B faithful one-step cadence produce statistically significant positive welfare deltas vs the `(d8, t50, w32)` baseline at Number of seeds (N) = 20 seeds."
>
> **Backed by:**
> - **CLM-10** (`single-lane-EIP-1559-control`, cell `d4_t50_w32`): BCa 95% CI = `[+3.38e+09, +1.35e+10]`; median Δ = `+5.37e+09`; sign-coherence = `0.75`; distinct-hash = `20/20`.
> - **CLM-11** (`single-lane-EIP-1559-control`, cell `d8_t25_w32`): BCa 95% CI = `[+4.68e+08, +5.66e+09]`; median Δ = `+7.81e+07`; sign-coherence = `0.55`; distinct-hash = `20/20`.
> - Companion: the two ranking-block-quarter sign-flip cells (CLM-12, CLM-13; `cell_rb_reserved_x4_rb_quarter` and `cell_partitioned_x4_rb_quarter`) produce real-but-noisy positive medians whose CIs straddle zero and are reported as `WEAK` (ordering-level).
>
> **Source-of-truth:** `test-results/multi-seed-variance/results.md` §"TEST-03 sign-flip variance bands" + `audit-documents/coverage-check.md` rows CLM-10, CLM-11, CLM-12, CLM-13.

> **Headline Claim 6:** "The COV-05 hash-diversity gate passes 17 of 17 BACKED-eligible cells at distinct count = Number of seeds (N) cited; no cell was downgraded to WEAK from gate failure."
>
> **Backed by:**
> - Phase 3 hash-diversity-gate report at `test-results/hash-diversity-gate/results.md`: TEST-03 (6 jobs × 20 seeds, all 20/20), TEST-04 (5 jobs × 20 seeds, all 20/20), TEST-07a (6 jobs × 5 seeds, all 5/5). Cross-cell Secure Hash Algorithm 256-bit (SHA-256) identity in TEST-07a is across-cell (does not violate within-cell gate per `RSK-hash-diversity-policy`'s disclosure-paragraph).
>
> **Source-of-truth:** `test-results/hash-diversity-gate/results.md` + `audit-documents/realism-risks-register.md` §"RSK-hash-diversity-policy".

**Reference-only:** the full 55-row coverage matrix at `audit-documents/coverage-check.md`; the long-tail CLM rows beyond CLM-13 (calibration anchors at CLM-24..27, reorg-safety claims at CLM-19..23, anti-bribery rows at the appropriate column positions). The CIP author may add reference-only CLM citations to support specific claims; the headline list above is the load-bearing subset for the CIP's menu-option recommendation.

## CIP Section: Limitations

**Source-of-truth:** `audit-documents/realism-risks-register.md` (24 DISCLOSED entries; load-bearing disclosure-paragraphs).

### Limitations paste order

The substrate-scope umbrella leads, then category-grouped highest-priority entries (substrate-scope corollaries → controller calibration → topology / calibration freshness). The four inline paragraphs below are the load-bearing top-4 paste targets per `.planning/phases/05-handoff/05-CONTEXT.md` `<specifics>`. The CIP author may include all 24 paragraphs or a paste-order-trimmed subset; the substrate-scope umbrella is mandatory (it sets the scope boundary against which all other paragraphs read).

#### Inline 1 (umbrella): RSK-substrate-scope

> The Cardano Improvement Proposal (CIP)'s evidence base is generated by a Rust simulator that inherits three categories of substrate limitation from the upstream Leios reference implementation, each disclosed here as a scope boundary rather than a tested-and-resolved property. **(a) Floating-point arithmetic in non-pricing code paths.** The pricing kernel itself (admission, eviction, fee charging, controller coefficient, mempool tracking, multiplier-floor invariant, actor lane choice) is implemented in integer / rational / 128-bit unsigned (`u128`) arithmetic for bit-stability; however, the upstream non-pricing substrate (slot lottery, propagation timing, distribution sampling, plus a residual `f64::sqrt` site in `endorsement_window_priced_blocks`) retains `f64` floating-point arithmetic. Reproducibility is asserted intra-architecturally against pinned golden hashes on x86_64 / glibc (the reference build environment), via three layers: unit-test goldens in `sim-rs/sim-core/src/sim/tests/`, the `experiment-suite verify` subcommand, and suite-level goldens in `sim-rs/parameters/phase-2-sweep/suites/.goldens/`. Cross-architecture continuous integration (CI) verification (e.g. Advanced RISC Machine (ARM) builds, alternative C standard libraries) is disclosed as deferred future work. **(b) Propagation-model fidelity.** The simulator's network propagation is round-trip-time-driven across a 100-node topology with real-world-derived latencies (geographically distributed pools, varying round-trip times); it is not validated against packet-level Cardano mainnet network traces. Welfare claims are conditional on the simulator's propagation regime; the deployed-system propagation regime may shift welfare magnitudes without changing the qualitative menu-item orderings. **(c) Utility-maximising actor model.** The actor model is utility-maximising across multiple urgency-tagged demand components; it does not model strategic-bidder regimes including bribery, side contracts, Maximum Extractable Value (MEV) strategies, or sustained controller gaming. Chung and Shi's *Foundations of Transaction Fee Mechanism Design* (Symposium on Discrete Algorithms (SODA) 2023) is cited as the formal frame for those unmodelled incentive/collusion regimes, not as evidence that this simulator exercises them; adversarial regimes are disclosed as future work outside this evidence base. Cardano's extended unspent-transaction-output (eUTxO) model is structurally MEV-resistant by construction (no global mempool), bounding the practical relevance of this gap for the non-adversarial regime but not eliminating it.

#### Inline 2: RSK-cross-arch-determinism

> The Cardano Improvement Proposal (CIP)'s evidence base asserts intra-architectural determinism on the x86_64 / glibc reference build environment, pinned at three layers of golden hashes (unit-test goldens in `sim-rs/sim-core/src/sim/tests/`, the `experiment-suite verify` subcommand re-running each completed (job, seed) pair, and suite-level goldens in `sim-rs/parameters/phase-2-sweep/suites/.goldens/`). The pricing kernel's arithmetic is integer / rational / 128-bit unsigned (`u128`) and `libm::pow` / `libm::round` / `libm::exp` throughout — bit-stable across architectures by construction given identical inputs. Cross-architecture continuous integration (CI) verification (e.g. Advanced RISC Machine (ARM) builds, alternative C standard libraries) is not yet built; the residual `f64::sqrt` site in `endorsement_window_priced_blocks` (named in the review's critical findings as CR-1) is a small but nonzero asterisk on cross-architecture reproducibility because the Institute of Electrical and Electronics Engineers (IEEE) 754 standard does not mandate bit-exact correctly-rounded square root across implementations. Reviewers building on alternative architectures (e.g. ARM, increasingly common in 2026) may observe non-bit-identical golden hashes; all CIP claims about reproducibility should be qualified as intra-architectural pending the cross-architecture CI build-out and the `libm::sqrt` swap closing CR-1.

#### Inline 3: RSK-leios-spec-pre-deployment

> This Cardano Improvement Proposal (CIP) builds a pricing mechanism on top of the Leios substrate as specified in CIP-0164. The Leios-specific parameters (`linear-vote-stage-length-slots`, `linear-diffuse-stage-length-slots`, `eb-referenced-txs-max-size-bytes`, `eb-body-validation-cpu-time-ms-per-byte`, the cohort size `n`, the quorum threshold `τ`) cite CIP-0164 Table 7 with in-YAML provenance comments. Leios itself is pre-deployment at the time of writing; none of these values are cross-checkable against deployed-mainnet operational data. The Leios Frequently Asked Questions document (ranking-block (RB) ~20 seconds, endorser-block (EB) ~5 seconds) and CIP-0164 itself are the closest available anchors. The pricing-mechanism welfare claims are therefore conditional on the Leios substrate as specified; substrate maturation (Leios deployment, post-deployment calibration of the spec parameters) is out of scope for this evidence base and is the underlying anchor against which the welfare claims should be revisited after Leios deployment.

#### Inline 4: RSK-un-anchored-controller-knobs

> This Cardano Improvement Proposal (CIP) uses four pricing-controller calibration values; one is anchored against the Ethereum Improvement Proposal 1559 (EIP-1559) academic-critique tradition and three carry "conditional on X" disclosures per the anchor-or-disclose work in `.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md`. **(a) Window length 32 priced blocks (ANCHORED)** is motivated by Reijsbergen et al. Advances in Financial Technologies (AFT) 2021 + Leonardos et al. AFT 2021 + Liu et al. Conference on Computer and Communications Security (CCS) 2022; the `phase-2-eip1559-smoothing` suite sweeps {16, 32, 64} for sensitivity. **(b) Multiplier floor 4** in `phase-2-rb-scarcity` and `phase-2-urgency-inversion` is an internal calibration accommodation, not an externally anchored economic claim; TEST-07a confirms regime-dependence at floor=16 (see Headline Claim 3 above). **(c) Multiplier floor 16** (the spec default) has no external anchor; welfare findings from the five suites with an `x16` variant are reported under the spec-stated assumption that 16 gives strong discrimination. **(d) Lane-signal-source choices** for un-reserved priority (option 1: `priority_paying_bytes / total_block_capacity`) and both-dynamic standard (`standard_paying_bytes / eb_referenced_txs_max_size_bytes` over endorser blocks; no standard sample on RB-reserved RBs) have no external anchor — the EIP-1559 academic literature analyses single-lane controllers only. Welfare findings from un-reserved priority + both-dynamic standard sides are conditional on these specific signal-source definitions; alternative signal sources were not exercised.

### Reference-only Limitations table (20 long-tail entries)

The CIP author may paste these entries verbatim from the register or cite them by RSK-NN identifier. All 24 register entries are CIP-pasteable; the table below lists the 20 not already inlined above. Approximate line ranges are at the post-Plan-05-01 register state.

**Recommended paste-order for the long tail** (Claude's-Discretion default per `.planning/phases/05-handoff/05-CONTEXT.md` `<specifics>`): topology / calibration freshness (`RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`) → simulation run-length boundary (`RSK-steady-state-run-length`) → semantic-reinterpretation cluster (`RSK-fee-as-maxFee-envelope`, `RSK-mempool-cap-magnitude`, `RSK-max-fee-policy-default`, `RSK-target-inclusion-blocks-default`) → demand-modelling cluster (`RSK-demand-mix-bit-calibration`, `RSK-demand-non-stationarity`, `RSK-sundaeswap-demand-staleness`) → suite-coverage corollary (`RSK-multiplier-floor-4-suite-coverage`) → honest-producer-assumption boundary (`RSK-partition-activated-honest-producer`) → reporting-precision cluster (`RSK-admission-rejection-attribution`, `RSK-welfare-as-f64-reporting`) → Plan-05-01 prose-promoted entries (`RSK-single-seed-precision`, `RSK-three-seed-statistical-power`, `RSK-unresolved-suite-claims`, `RSK-standard-user-fee-drift-exposure`, `RSK-menu-collapse-to-advocacy`) → gate-semantics closing (`RSK-hash-diversity-policy`). The CIP author may reorder if a different reading flow serves the CIP narrative better; the substrate-scope umbrella must remain first (it sets the scope boundary against which all other paragraphs read).

| RSK-NN | Title | Source-of-truth |
|--------|-------|-----------------|
| RSK-pool-count | Pool-count sensitivity above 100 pools | `audit-documents/realism-risks-register.md` lines 47–62 |
| RSK-single-seed-precision | Single-seed welfare claims at publication precision | `audit-documents/realism-risks-register.md` lines 64–79 |
| RSK-fee-as-maxFee-envelope | Fee-field semantic reinterpretation as maxFee envelope | `audit-documents/realism-risks-register.md` lines ~166–190 |
| RSK-mempool-cap-magnitude | Mempool absolute byte cap 133× mainnet | `audit-documents/realism-risks-register.md` lines ~167–190 |
| RSK-max-fee-policy-default | Default actor `max_fee_policy = {4, 1}` is a forecast about wallet behaviour | `audit-documents/realism-risks-register.md` lines ~167–190 |
| RSK-calibration-stale-stake-snapshot | Epoch-582 stake snapshot freshness | `audit-documents/realism-risks-register.md` lines 191–207 |
| RSK-demand-mix-bit-calibration | Q1 2026 mainnet demand mix order-of-magnitude correct, not bit-calibrated | `audit-documents/realism-risks-register.md` lines ~208–230 |
| RSK-demand-non-stationarity | Finer-than-2-hour demand patterns not modelled | `audit-documents/realism-risks-register.md` lines ~231–250 |
| RSK-target-inclusion-blocks-default | `target_inclusion_blocks` defaults mechanism-induced | `audit-documents/realism-risks-register.md` lines ~251–265 |
| RSK-partition-activated-honest-producer | `partition_activated` is a producer claim, not body-derivable | `audit-documents/realism-risks-register.md` lines ~251–265 |
| RSK-multiplier-floor-4-suite-coverage | Two suites condition exclusively on `multiplier_floor = 4` (regime-dependence at 16) | `audit-documents/realism-risks-register.md` lines 282–295 |
| RSK-three-seed-statistical-power | Three-seed suite default cannot license tight 95% CIs | `audit-documents/realism-risks-register.md` lines 297–311 |
| RSK-unresolved-suite-claims | Four UNRESOLVED suite verdicts resolved via Plan 02-02 output-read pass | `audit-documents/realism-risks-register.md` lines 313–325 |
| RSK-standard-user-fee-drift-exposure | Both-dynamic standard-lane drift bounded by EIP-1559 ±1/D per-block clamp | `audit-documents/realism-risks-register.md` lines 327–339 |
| RSK-admission-rejection-attribution | Gate-reject vs mempool-reject collapsed into one bool (WR-2 deferred) | `audit-documents/realism-risks-register.md` lines 359–373 |
| RSK-menu-collapse-to-advocacy | Welfare-only evidence + 4 non-welfare property columns | `audit-documents/realism-risks-register.md` lines 375–389 |
| RSK-steady-state-run-length | 2000-slot run length partial coverage at 1 of 4 menu arms | `audit-documents/realism-risks-register.md` lines 391–404 |
| RSK-hash-diversity-policy | Hash-diversity strict gate per Phase 2 D-19 + 17/17 BACKED-eligible pass | `audit-documents/realism-risks-register.md` lines 406–419 |
| RSK-welfare-as-f64-reporting | Welfare aggregates reported as `f64`; ≤ 3 significant figures | `audit-documents/realism-risks-register.md` lines 421–433 |
| RSK-sundaeswap-demand-staleness | SundaeSwap January 2022 4-year-old retail spike | `audit-documents/realism-risks-register.md` lines 435–448 |

## Pinned references

This block names the citable references that the Cardano Improvement Proposal (CIP) author quotes in the CIP's footer or methodology section. The git tag is **user-executed** per the project's don't-auto-commit convention — see §"Tag message draft" below for the message to paste into `git tag -a`.

### Citable git tag

**Tag name:** `phase-2-cip-evidence-v1`.

**Status:** (tag pending: applied by the user via `git tag -a phase-2-cip-evidence-v1 -m '<see tag message draft below>'` against commit `7f4595ed264e4be46cb007d82ade402f9c54c833` or against the post-Plan-05-03 commit that lands `cip-author-summary.md`).

After the user applies the tag, replace the placeholder line above with: "Tag applied: `phase-2-cip-evidence-v1` at commit `<full-40-char-SHA>` on `<date-applied>`." (Plan 05-03 Task 3 covers this swap.)

### Milestone-close commit

**Commit Secure Hash Algorithm 256-bit (SHA-256):** `7f4595ed264e4be46cb007d82ade402f9c54c833` (the post-Plan-05-02 commit; the post-Plan-05-03 commit landing this file will supersede it). The `phase-2-cip-evidence-v1` tag should reference the post-Plan-05-03 commit Secure Hash Algorithm 256-bit (SHA-256), which the user can resolve via `git rev-parse HEAD` after committing `cip-author-summary.md` and before applying the tag.

### Cardano mainnet stake snapshot reference

**Snapshot:** Cardano mainnet, epoch 582, retrieved 2026-05-14. Per `audit-documents/cardano-realism-audit.md` §"Topology and actor model" + `.planning/spikes/006-curve-design/README.md` reproduction recipe (`sim-rs/scripts/generate-realistic-100-topology.py` at the snapshot epoch). The 100-node realistic topology used across all goldens-pinned suites is derived from this snapshot via mass-stratified downsampling (top-1 stake share 1.97%; Nakamoto coefficient 35; Gini 0.253). The CIP cites this snapshot as the topology-source-of-truth.

### Consistency audit reproducibility

The consistency of cross-references across the six in-scope documents (the five CIP-cited artefacts plus this summary) was audited by `consistency-audit/verify-consistency.sh` and recorded in `consistency-audit/CONSISTENCY-REPORT.md`. Future CIP peer reviewers may re-run the script independently to verify the consistency claims at the tagged commit or at any later commit:

```
bash cip-evidence/consistency-audit/verify-consistency.sh
```

Expected: exit code 0; OVERALL: PASS across all four checks (Realism Risk identifier (RSK)-NN / Claim identifier (CLM)-NN / Experiment identifier (EXP)-NN dead-reference scan; backing-job path resolution against suite Yet Another Markup Language (YAML) files; golden-sha256 cross-check against the seven `.goldens/<suite>.sha256` files; markdown link + backtick-path resolution).

### Tag message draft

Paste the block below verbatim into the `git tag -a phase-2-cip-evidence-v1 -m '<...>'` command:

```
phase-2-cip-evidence-v1 — milestone-close tag

Citable reference for the Cardano Improvement Proposal (CIP)
responding to CPS-0023 ("Urgency Signaling").

CIP-cited artefacts (under cip-evidence/ at this tag):
- cip-evidence/audit-documents/cardano-realism-audit.md
- cip-evidence/audit-documents/validity-threats.md
- cip-evidence/audit-documents/realism-risks-register.md (24 DISCLOSED + 0 LIVE)
- cip-evidence/audit-documents/coverage-check.md
- cip-evidence/audit-documents/methodology-overview.md
- cip-evidence/cip-author-summary.md (paste guide)

Topology snapshot: Cardano mainnet, epoch 582, retrieved 2026-05-14.
Consistency audit: cip-evidence/consistency-audit/CONSISTENCY-REPORT.md
```

### HAND-03 execution note

Per the project's don't-auto-commit convention, Plan 05-03 drafts only the tag message; **the user runs `git tag`**. After committing this file:

1. Resolve the post-Plan-05-03 commit Secure Hash Algorithm 256-bit (SHA-256) via `git rev-parse HEAD`.
2. Run, from the repo root:
   ```
   git tag -a phase-2-cip-evidence-v1 -m "$(cat <<'EOF'
   <paste the §"Tag message draft" block above verbatim>
   EOF
   )"
   ```
3. Confirm the tag landed: `git tag --list 'phase-2-cip-evidence-v1' && git show phase-2-cip-evidence-v1 | head -20`.
4. (Optional) Push the tag for a remote citable reference: `git push origin phase-2-cip-evidence-v1`.
5. Edit this file's §"Citable git tag" subsection to replace the `(tag pending: ...)` placeholder with the live `Tag applied: ...` annotation.
6. Re-run `consistency-audit/verify-consistency.sh` once more to confirm the placeholder swap did not introduce any dead references (expected: exit 0; no dead refs).
7. Append a final line to `consistency-audit/CONSISTENCY-REPORT.md` §"Post-Plan-05-03 verification" recording the tag-application date and the post-tag commit Secure Hash Algorithm 256-bit (SHA-256).

## What is NOT in this evidence base

The Cardano Improvement Proposal (CIP) author should treat the items below as **out of scope** for the evidence base at the `phase-2-cip-evidence-v1` tag. Each is disclosed in the realism-risks register or in `.planning/PROJECT.md` §"Out of Scope":

- **The Cardano Improvement Proposal (CIP) text itself.** The summary is a paste guide; the CIP author writes the CIP draft.
- **Adversarial / strategic-bidder modelling.** The actor model is utility-maximising; bribery, side contracts, Maximum Extractable Value (MEV) strategies, and sustained controller gaming are disclosed as future work (see `RSK-substrate-scope` umbrella subsection (c)).
- **Cross-architecture continuous integration (CI) verification.** Determinism is intra-architectural on x86_64 / glibc; cross-architecture CI is deferred (see `RSK-cross-arch-determinism`).
- **Pool-count regimes above 100 pools.** The realistic-100 topology approximates Cardano mainnet via a mass-stratified downsample; the CIP-0164 600-pool migration regime and the present-day approximate-3000-pool mainnet regime are not exercised (see `RSK-pool-count`).
- **Re-runs of TEST-05 / TEST-06.** Both Phase 3 tests are at partial coverage at Phase 5 close; the three affected register entries (`RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, `RSK-steady-state-run-length`) remain DISCLOSED. Future work to complete the re-runs would license MITIGATED flips on the three entries if the data lands inside the locked thresholds.
- **Upstream Leios spec maturation.** The pricing-mechanism welfare claims are conditional on the Leios substrate as specified in CIP-0164; substrate maturation is out of scope (see `RSK-leios-spec-pre-deployment`).

## Closing footer

Plan 05-03 closes Phase 5 — the milestone-close phase of the Phase-2 Cardano Improvement Proposal (CIP) Evidence Audit project. The CIP author may now copy from this summary into the CIP draft; the underlying source-of-truth artefacts (`cardano-realism-audit.md`, `validity-threats.md`, `realism-risks-register.md`, `coverage-check.md`, `methodology-overview.md`) are stable at the post-`phase-2-cip-evidence-v1` tag. Subsequent edits to the evidence base require a new tagged version (e.g. `phase-2-cip-evidence-v2`) and a corresponding re-run of `consistency-audit/verify-consistency.sh` to refresh the consistency audit baseline.

See `.planning/phases/05-handoff/05-SUMMARY.md` for the Phase 5 SUMMARY consumed by `gsd-verify-phase`.
