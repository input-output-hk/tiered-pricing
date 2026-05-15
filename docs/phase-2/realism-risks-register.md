# Realism-Risks Register

**Status:** Draft (Phase 1)
**Scope:** Unresolved realism risks affecting the Cardano Improvement Proposal (CIP) responding to CPS-0023 ("Urgency Signaling")
**Identifier convention:** `RSK-NN` for risks (this document) and `EXP-NN` for register-flagged cheap tests, per Leios precedent (`docs/ImpactAnalysis.md`). Append-only — identifiers never renumber.
**Categorisation:** Wohlin four-fold (construct / internal / external / conclusion). Multi-tagging permitted where a risk genuinely straddles categories.
**Verdict vocabulary:** LIVE / DORMANT / MITIGATED / DISCLOSED. Ambiguous cells default to LIVE (most-conservative).

## Reading guide

This register is the single source of truth for unresolved realism risks affecting the CIP. Per-claim trust ratings live in `docs/phase-2/coverage-check.md` (Phase 2). Calibration-provenance triples `(value, source, date-retrieved)` live in the refreshed `docs/phase-2/cardano-realism-audit.md` (Phase 4). Spike READMEs in `.planning/spikes/` are the resolved-or-disclosed audit trail; this register cites them as evidence sources but does not re-express their verdicts.

This file is the Phase 1 inventory skeleton. Descriptive fields (`Description`, `Evidence-for`, `Evidence-against`) are populated in full; judgement fields (`Verdict`, `Scope-of-resolution`, `EXP / Resolution`, `Disclosure-paragraph`) carry `TBD plan 02` placeholders that Plan 01-02 finalises. Plan 01-02 may grep for `TBD plan 02` to find every site requiring judgement work.

The Inter-Quartile Range (IQR) is used as the canonical seed-noise band throughout: a finding is treated as inside seed noise iff its magnitude is below the IQR of the corresponding cell across seeds. The Paired Seed Evaluation (PSE) methodology — comparing two mechanism variants on the same seed to isolate the mechanism effect — is the foundation of Phase 3's Bias-corrected and accelerated (BCa) bootstrap confidence intervals on welfare deltas.

## Index

| RSK | Title | Category (initial) |
|-----|-------|--------------------|
| RSK-pool-count | Pool-count sensitivity above 100 pools | external |
| RSK-single-seed-precision | Single-seed welfare claims at publication precision | conclusion, construct |
| RSK-un-anchored-controller-knobs | Four un-anchored controller knobs (window-length, two multiplier-floors, lane-signal-source) | external, construct |
| RSK-substrate-scope | Inherited substrate scope (`f64` in non-pricing paths, propagation fidelity, utility-maximising actor model) | external, construct |
| RSK-fee-as-maxFee-envelope | Fee-field semantic reinterpretation as maxFee envelope; refund-CIP dependency | construct |
| RSK-mempool-cap-magnitude | Mempool absolute byte cap 133× mainnet (24 MB vs ~180 KB); rule matches | external |
| RSK-max-fee-policy-default | Default actor `max_fee_policy = {4, 1}` is a forecast about wallet behaviour, not an anchor | construct |
| RSK-calibration-stale-stake-snapshot | Epoch-582 stake snapshot freshness over publication horizon | external |
| RSK-demand-mix-bit-calibration | Q1 2026 mainnet demand mix order-of-magnitude correct, not bit-calibrated | external |
| RSK-demand-non-stationarity | Finer-than-2-hour demand patterns (diurnal, NFT drops, governance deadlines) not modelled | external |
| RSK-target-inclusion-blocks-default | `target_inclusion_blocks` defaults are mechanism-induced, not mainnet-anchored | construct |
| RSK-partition-activated-honest-producer | `partition_activated` is a producer claim, not body-derivable; byzantine-producer risk | external |
| RSK-leios-spec-pre-deployment | Linear-Leios spec knobs not cross-checkable to deployed mainnet (CIP-0164 is pre-deployment) | external |
| RSK-multiplier-floor-4-suite-coverage | Two suites condition exclusively on `multiplier_floor = 4` | construct, external |
| RSK-three-seed-statistical-power | Three-seed suite default cannot license tight 95% confidence intervals | conclusion |
| RSK-unresolved-suite-claims | Four UNRESOLVED suite verdicts pending output read | conclusion |
| RSK-standard-user-fee-drift-exposure | Both-dynamic standard-lane drift exposure under realistic / spike demand | external, construct |
| RSK-cross-arch-determinism | Determinism intra-architecture only; cross-architecture not proven (CR-1 `f64::sqrt` residual) | conclusion |
| RSK-admission-rejection-attribution | Gate-reject vs mempool-reject collapsed into one bool; eviction-cause attribution gap | internal |
| RSK-menu-collapse-to-advocacy | Welfare-only evidence collapses 4-way menu into single-option recommendation | conclusion |
| RSK-steady-state-run-length | 2000-slot run length not verified to be steady-state for every menu item | internal, conclusion |
| RSK-hash-diversity-policy | Hash-diversity sanity check policy (strict vs soft gate) unresolved | conclusion |
| RSK-welfare-as-f64-reporting | Welfare aggregates reported as `f64`; precision boundary not surfaced | conclusion |
| RSK-sundaeswap-demand-staleness | SundaeSwap January 2022 launch profile is a 4-year-old retail spike, not steady-state | external |

## RSK-pool-count: Pool-count sensitivity above 100 pools

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The default phase-2 topology (`topology-realistic-100.yaml`) is a mass-stratified 100-node downsample of the Cardano mainnet (epoch 582) ~3,000 stake-pool-operator (SPO) population. Welfare conclusions derived at 100 pools may not transfer to deployed-mainnet pool counts, and the Cardano Improvement Proposal (CIP) drafted from this evidence base may be challenged by a reviewer asking "does this welfare claim replicate at 600 pools (CIP-0164 calibration) or at ~3,000 pools (current mainnet)?". The prototype-pattern cheap test in PROJECT.md Active item 2 (pool-number sensitivity smoke at 100 vs 150 pools across `sundaeswap_moderate` plus the four `paper_like_*` variants) is the canonical method to bound this risk; the failure-mode hypothesis is that Δ% on welfare metrics falls inside the seed-Inter-Quartile-Range (seed-IQR) of the same job at 100 pools.
**Evidence-for:**
- `.planning/research/PITFALLS.md` §"CRIT-5: Calibration-stale parameters cited as current" — calibration-drift framing, with pool-count sensitivity as the prototype cheap test
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" — mainnet ~3,000 SPOs vs the simulator's downsampled 100; multi-producer slot-battle dynamics
- `.planning/spikes/006-curve-design/README.md` — calibration-provenance for the epoch-582 stake snapshot; reproduction recipe via `sim-rs/scripts/generate-realistic-100-topology.py`
- `.planning/PROJECT.md` Active item 2 — pool-number sensitivity prototype-pattern test naming 100 vs 150 pools across five demand profiles
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` §"What lines up with mainnet" — RB cadence, fee floor, and Leios-specific knobs match deployed values bit-exact; topology is the *abstraction*, not the *parameter set*, so calibrated values themselves transfer
- `.planning/spikes/006-curve-design/README.md` — the mass-stratified curve preserves Nakamoto coefficient (35) and top-stake share (1.97%); not a uniform downsample
**Scope-of-resolution:** Δ% < seed-IQR (Inter-Quartile Range) of same job at 100 pools establishes MITIGATED.
**EXP / Resolution:** EXP-pool-number (→ TEST-05)
**Disclosure-paragraph:** TBD plan 02

## RSK-single-seed-precision: Single-seed welfare claims at publication precision

**Category:** conclusion, construct
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The 33-job sundaeswap smoke run that drove the Family B mechanism commitment used seed=1 only. Four cells in that smoke run produced welfare sign-flips between accumulator and chain-derived: `d4_t50_w32` and `d8_t25_w32` under single-lane Ethereum Improvement Proposal 1559 (EIP-1559), and `x4_rb_quarter` under both the `priority-only-rb-reserved` and `partitioned-both-dynamic` arms (4 cells total). Each sits close to zero in absolute welfare and could plausibly invert at a different seed. A reviewer asking "what is the 95% confidence interval (CI) on this sign-flip?" gets no answer today. Phase 3's Paired Seed Evaluation (PSE) with Bias-corrected and accelerated (BCa) bootstrap intervals is the canonical resolution path; the broader 19-suite × 3-seed sweep cannot license tight CIs either, and PSE re-runs at N ≥ 10 seeds on the flip cells (and the canonical menu-item welfare cells) are the smallest set of additional runs that turn the welfare narrative into a publishable claim.
**Evidence-for:**
- `.planning/research/PITFALLS.md` §"CRIT-1: Single-seed welfare claims at publication-grade precision"
- `.planning/mechanism-welfare-impact-2026-05-14.md` §TL;DR + §"Per-job table" — the four sign-flip cells named explicitly (seed=1 only)
- `.planning/family-b-decision-2026-05-14.md` §"Empirical welfare-impact characterisation" — Family B decision sourced from the seed=1 smoke
- `.planning/family-b-results-table-2026-05-14.md` — broader 19-suite × 3-seed table, but 3 seeds is below the seed count where Bias-corrected and accelerated (BCa) bootstrap coverage is nominal
**Evidence-against:**
- `.planning/mechanism-welfare-impact-2026-05-14.md` §TL;DR — the headline two-lane un-reserved arms preserve qualitative claim under Family B with median |Δ%| ≤ 17% and zero sign-flips, suggesting the mechanism-robust subset of the menu does not depend on the flip cells
- `docs/phase-2/validity-threats.md` §"Trust framework" — three seeds is "enough to detect qualitative-direction flips" for ordering claims that do not depend on the four flip cells
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** EXP-sign-flip-variance (→ TEST-03); EXP-canonical-variance (→ TEST-04)
**Disclosure-paragraph:** TBD plan 02

## RSK-un-anchored-controller-knobs: Four un-anchored controller knobs (window-length, two multiplier-floors, lane-signal-source)

**Category:** external, construct
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Four phase-2 controller knobs have no deployed-system anchor and must each either find an external citation or carry an explicit Cardano Improvement Proposal (CIP) disclosure paragraph of the form "we chose X; alternative Y exists; we did not exercise Y; the qualitative finding is conditional on X":

1. **Window length 32** for capacity-varying signals (single-lane Ethereum Improvement Proposal 1559 (EIP-1559), both-dynamic standard, un-reserved priority). The audit notes this is "a round-number choice, not an empirical anchor"; the `phase-2-eip1559-smoothing` suite sweeps {16, 32, 64} for sensitivity but does not bracket the unwindowed Ethereum-equivalent `window = 1` or an over-smoothed `128`.
2. **Multiplier-floor 4** in `phase-2-rb-scarcity` and `phase-2-urgency-inversion`. At the spec default 16, priority demand stays too thin to surface controller drift in these two suites; at 4, the controller does drift. The CLAUDE.md project context flags this explicitly as "a calibration accommodation, not an economic claim."
3. **Multiplier-floor 16** — the spec default itself. Has no empirical anchor: `mechanism-design.md` lists it without citing calibration data, and Ethereum has no comparable second-lane multiplier.
4. **Lane-signal-source choices**: (a) un-reserved priority defaults to option 1 (`priority_paying_bytes / total_block_capacity`) — one of three open candidates in the spec; (b) both-dynamic standard defaults to `standard_paying_bytes / eb_referenced_txs_max_size_bytes` for endorser blocks (EBs) with no standard sample on ranking-block-reserved (RB-reserved) RBs.

This umbrella entry covers all four sub-points because they share a single resolution path (the two-hour literature search at Phase 4 open, then anchor or disclose). Grouping prevents the "looks anchored on 3 of 4" partial-resolution failure mode.

**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Pricing-controller calibration" — disclosure items 1–4 enumerate the four knobs in the same order as above
- `.planning/spikes/003-pricing-controller-calibration/README.md` — NEEDS-DISCLOSURE verdict; per-knob comparison-table evidence
- `.planning/research/PITFALLS.md` §"CRIT-3: Reviewer-anticipated-question gaps" — the four knobs are named explicitly as the canonical cite-without-rationale risk
- `docs/phase-2/mechanism-design.md` §"Open calibration choices" and lines 207–211 (un-reserved priority signal-source options 1–3)
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` §"What lines up with mainnet" — the EIP-1559 *core* parameters (`D = 8`, `target = 0.5`, per-priced-block update cadence) match Ethereum mainnet bit-exact, bounding the un-anchored scope to non-core knobs
- `docs/phase-2/cardano-realism-audit.md` disclosure-item 2 — the priority-only suites sweep multiplier-floor at {4, 8, 16} and the both-dynamic suite sweeps {4, 16}, so phase-2 findings are reported *across* the floor sweep at five of seven suites
- External literature candidates for Phase 4 search: Liu et al. Conference on Computer and Communications Security (CCS) 2022, Reijsbergen et al. Advances in Financial Technologies (AFT) 2021 — Ethereum window-length and step-size data may anchor window=32 if comparable
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** EXP-window-length-anchor; EXP-multiplier-floor-4-anchor; EXP-multiplier-floor-16-anchor; EXP-lane-signal-source-anchor (all TBD plan 02 — also see DOC-03)
**Disclosure-paragraph:** TBD plan 02

## RSK-substrate-scope: Inherited substrate scope (`f64` in non-pricing paths, propagation fidelity, utility-maximising actor model)

**Category:** external, construct
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The phase-2 work hardens the pricing kernel and mempool gate to integer/rational/u128 discipline but inherits the upstream Leios simulator substrate, which carries three categories of unresolved realism limitation that the Cardano Improvement Proposal (CIP) must disclose explicitly:

(a) **Upstream `f64` in non-pricing hot paths** — slot lottery (`sim-rs/sim-core/src/sim/lottery.rs`), propagation timing (`sim/driver.rs`, `sim/slot.rs`), distribution sampling, and the residual `f64::sqrt` site in `endorsement_window_priced_blocks` (review finding CR-1) put a small but nonzero asterisk on cross-architecture reproducibility. Intra-architecture determinism on x86_64 / glibc is pinned by goldens; cross-arch is not yet proven.

(b) **Propagation-model fidelity** — the simulator's round-trip-time-driven (RTT-driven) topology with default real-world-derived latencies stands in for the production Cardano mainnet propagation reality (geographically distributed pools, varying RTTs, dynamic peer selection). The model is reasonable but not validated against mainnet propagation traces.

(c) **Utility-maximising actor model** — the actor model has no adversarial / strategic bidders. Chung and Shi's Symposium on Discrete Algorithms (SODA) 2023 impossibility result for user-incentive-compatibility, miner-incentive-compatibility, and side-contract-proofness in transaction-fee mechanisms is the canonical formal frame; the CIP must lead its caveat list with this scope, citing Chung and Shi for the formal impossibility frame and Roughgarden's foundational Transaction Fee Mechanism Design work for the strategic-bidder regime definition.

The three sub-points share a single mitigation path (none — they are inherited substrate and out-of-scope for re-audit per PROJECT.md Out of Scope items 2 and 3); the disclosure paragraph in Plan 01-02 enumerates them under one umbrella entry. Each sub-point is individually citable from the CIP via the disclosure-paragraph anchor.

**Evidence-for:**
- `CLAUDE.md` §"Numeric representation contract" and §"Determinism scope" — the `f64` boundary is precisely documented; cross-arch CI verification is named as not-yet-built
- `.planning/REVIEW.md` §"Critical Findings: CR-1" — `f64::sqrt` in `endorsement_window_priced_blocks` is the named residual
- `.planning/codebase/CONCERNS.md` §"Historical f64 inheritance from upstream non-pricing code paths" — full list of inherited f64 sites
- `.planning/spikes/004-topology-and-actor-model/README.md` — NEEDS-DISCLOSURE verdict; substrate-scope sub-points (b) and (c)
- `.planning/research/PITFALLS.md` §"CRIT-4: Inherited-substrate limitations not disclosed" — frames the substrate-scope-paragraph as a top-three CIP disclosure
- `.planning/PROJECT.md` §"Out of Scope" items 2 (adversarial actors) and 3 (upstream re-audit)
**Evidence-against:**
- `CLAUDE.md` §"Determinism scope" — intra-arch determinism is asserted with pinned golden hashes at three layers (unit-test goldens, `experiment-suite verify`, suite-level goldens), so reproducibility on x86_64 / glibc is robust
- `.planning/REVIEW.md` §"Cross-cutting observations" item 1 — the integer/rational discipline is "honoured pervasively in the pricing kernel and mempool gate"; the substrate scope is bounded to non-pricing paths
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" disclosure-item 3 — Cardano's extended unspent-transaction-output (eUTxO) model is structurally Maximum Extractable Value (MEV)-resistant by construction (no global mempool), so the utility-maximising actor model is "mainnet-faithful in shape" for the non-adversarial regime
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely disclose-only; no cheap test resolves an inherited-substrate limitation)
**Disclosure-paragraph:** TBD plan 02

## RSK-fee-as-maxFee-envelope: Fee-field semantic reinterpretation as maxFee envelope; refund-CIP dependency

**Category:** construct
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Phase-2 reinterprets the existing Cardano transaction `fee` field as a `max_fee_lovelace` envelope: at admission the wallet posts a maximum fee; at inclusion the (possibly-lower) current quote is charged and the gap is refunded. Deployed mainnet today has no maxFee envelope and no refund path — wallets ship the exact deterministic fee via `cardano-serialization-lib`. The refund path depends on Polina's separate fee-change-return CIP being adopted; the phase-2 welfare conclusions assume this refund mechanism exists. This is a hard external dependency, not a soft one, and is the single most user-visible mechanism-level deviation from the world Cardano users have today.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Fee structure and mempool sizing" disclosure-item 1 — full framing as mechanism-level change, not calibration drift
- `docs/phase-2/mechanism-design.md` lines 39-51 — the reinterpretation is documented in the spec
- `.planning/spikes/002-fee-structure-and-mempool-sizing/README.md` — NEEDS-DISCLOSURE verdict
- `docs/phase-2/cardano-realism-audit.md` §"Recommended next steps" — refund-CIP dependency named as a "hard dependency to flag in any publication"
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` §"What lines up with mainnet" — the fee *floor* (`min-fee-a = 44`, `min-fee-b = 155381`) is bit-equal to Conway-era mainnet; the deviation is in the *envelope semantics*, not the floor itself
- `docs/phase-2/cardano-realism-audit.md` "Defensible because" rationale — the welfare claims explicitly assume the refund mechanism exists; the spec is transparent about the reinterpretation
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — no cheap test resolves a separate-CIP dependency)
**Disclosure-paragraph:** TBD plan 02

## RSK-mempool-cap-magnitude: Mempool absolute byte cap 133× mainnet (24 MB vs ~180 KB); rule matches

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The phase-2 mempool default absolute byte cap is 24 megabytes (MB) — 133× larger than Cardano mainnet's current ~180 kilobytes (KB). The cap *rule* matches mainnet exactly (`2 × one-bearer-block-bytes` with reject-on-full overflow), but the absolute number is a downstream consequence of CIP-0164's 12 MB endorser-block (EB) target replacing Praos's 90 KB ranking-block (RB) as the bearer block. A reviewer familiar with mainnet's current ~180 KB mempool may treat the 133× difference as a sizing-philosophy divergence when in fact it is a rule-conserving consequence of the Leios bearer-block size.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Fee structure and mempool sizing" disclosure-item 2
- `.planning/spikes/002-fee-structure-and-mempool-sizing/README.md` §"Comparison Table" row "mempool-max-total-size-bytes"
- `CLAUDE.md` §"Calibration choices" — `mempool-max-total-size-bytes = 2 × eb_referenced_txs_max_size_bytes`
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` §"Fee structure and mempool sizing" disclosure-item 2 "Defensible because" — the rule shape and overflow policy match exactly; the absolute number is a downstream consequence of CIP-0164's 12 MB EB target, not a different sizing philosophy
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — pre-Leios-deployment mempool sizing is not anchorable against mainnet operational data)
**Disclosure-paragraph:** TBD plan 02

## RSK-max-fee-policy-default: Default actor `max_fee_policy = {4, 1}` is a forecast about wallet behaviour, not an anchor

**Category:** construct
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The default actor `max_fee_policy = ScaledOverLaneQuote { numerator: 4, denominator: 1 }` gives 4× quote-drift headroom on the `max_fee_lovelace` envelope. This is a forecast about post-deployment Cardano wallet conventions analogous to Ethereum's ~2× `maxFeePerGas` default; it is not measured against Cardano user behaviour because no such behaviour exists today (mainnet wallets ship at exact min-fee via `cardano-serialization-lib`). The mispriced demand profile (`paper_like_mispriced.yaml`) uses `{1, 1}` (zero headroom) for the hard-deadline component to bound the worst case where users treat phase-2 like mainnet and ship at exact min-fee.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Fee structure and mempool sizing" disclosure-item 3
- `.planning/spikes/002-fee-structure-and-mempool-sizing/README.md` §Findings + §Verdict item 3
- `CLAUDE.md` §"Calibration choices" — `max_fee_policy = ScaledOverLaneQuote { numerator: 4, denominator: 1 }` documented as 4× quote-drift headroom forecast
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` §"Fee structure and mempool sizing" disclosure-item 3 "Defensible because" — `paper_like_mispriced.yaml` already bounds the worst case with `{1, 1}` headroom
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02
**Disclosure-paragraph:** TBD plan 02

## RSK-calibration-stale-stake-snapshot: Epoch-582 stake snapshot freshness over publication horizon

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The `topology-realistic-100.yaml` stakes are a mass-stratified downsample of the 1,510 Cardano mainnet pools with ≥ 1k ADA active stake at epoch 582 (retrieved 2026-05-14). Over a 6-month CIP review cycle, the snapshot will be 1-2 epochs stale at submission and 4-5 epochs stale at reviewer deep-read. The calibration-fix postmortem and PROJECT.md Active item 2 (pool-number sensitivity) are the mechanism by which the snapshot's load-bearing-ness can be bounded. This is the freshness-of-snapshot risk; it is logically separate from `RSK-pool-count` (the count-sensitivity risk) and from `RSK-demand-mix-bit-calibration` (the demand-side calibration risk).
**Evidence-for:**
- `.planning/research/PITFALLS.md` §"CRIT-5: Calibration-stale parameters cited as current" — calibration drift over publication horizon
- `CLAUDE.md` §"Calibration choices" — epoch 582 retrieval date 2026-05-14
- `.planning/spikes/006-curve-design/README.md` — calibration-provenance for the snapshot
**Evidence-against:**
- `.planning/spikes/006-curve-design/README.md` — top-1 stake share 1.97 %, Nakamoto coefficient 35, Gini 0.253; these summary statistics are slow-moving and unlikely to drift materially over 6 months
- `docs/phase-2/cardano-realism-audit.md` §"What lines up with mainnet" — the rest of the calibration (RB cadence, fee floor, EB knobs from CIP-0164 Table 7) is anchored against deployed values that update on protocol-update cadence, not epoch cadence
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely overlaps with EXP-pool-number — if 100 ≈ 150 and the bound also holds across snapshot-epoch range, freshness is bounded)
**Disclosure-paragraph:** TBD plan 02

## RSK-demand-mix-bit-calibration: Q1 2026 mainnet demand mix order-of-magnitude correct, not bit-calibrated

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The actor model uses three weighted components per profile (hard-deadline arbitrage / active decentralised finance (DeFi) / patient traffic) with fixed urgency families and Poisson arrivals per phase. Demand shares are order-of-magnitude correct against the Q1 2026 mainnet transaction mix (~35 % smart-contract, ~65 % transfer; total ~30 transactions per second) but the shares are not bit-calibrated. A reviewer asking "what fraction of the welfare claim is attributable to the demand-mix calibration vs the mechanism choice?" needs the explicit "under this stylised demand mix" framing before the welfare numbers.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" disclosure-item 3
- `.planning/spikes/004-topology-and-actor-model/README.md` §Findings item 4 + §Verdict ranking item 3
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` "Defensible because" rationale — the M4 / M5 sweeps probe demand-shape sensitivity via mispriced overlays and phased congestion variants; welfare claims should be reported "under this stylised demand mix"
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02
**Disclosure-paragraph:** TBD plan 02

## RSK-demand-non-stationarity: Finer-than-2-hour demand patterns (diurnal, NFT drops, governance deadlines) not modelled

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Phase-2's `Phased` arrival-rate machinery captures order-of-2-hours stress regimes but not finer-grained demand patterns: diurnal Coordinated Universal Time (UTC) working-hours peaks, non-fungible-token (NFT) drops, governance-deadline pile-ons. The controller-drift timescale is window-length × per-block-cadence ≈ 10 minutes, faster than diurnal demand shifts, but finer patterns at the minute scale (e.g. NFT-drop concurrent demand spikes) are not exercised.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" disclosure-item 5
- `.planning/spikes/004-topology-and-actor-model/README.md` §Findings items 5–6
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` "Defensible because" rationale — controller-drift timescale is ~10 minutes (faster than diurnal); the mispriced demand profile bounds the worst-case wallet-behaviour assumptions
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — adding diurnal / drop patterns is non-trivial demand modelling work)
**Disclosure-paragraph:** TBD plan 02

## RSK-target-inclusion-blocks-default: `target_inclusion_blocks` defaults are mechanism-induced, not mainnet-anchored

**Category:** construct
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The default `target_inclusion_blocks` (priority=1, standard=4) seed the actor's `LatencyEstimator` per (component, lane). Standard=4 models the expected wait when a standard transaction might sit behind several priority-only ranking blocks (RBs) — this is internal to the phase-2 mechanism, not measured on mainnet (where no priority lane exists). The observed-latency exponential-moving-average (EMA) overwrites the seed once inclusion events arrive, so the seed only shapes the first ~50 slots of actor lane choice, but the calibration choice still influences early-run lane-choice dynamics.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" disclosure-item 4
- `.planning/spikes/004-topology-and-actor-model/README.md` §"Comparison Table" row "`target_inclusion_blocks` defaults" + §Verdict ranking item 4
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` "Defensible because" rationale — observed inclusion latencies overwrite the seed once events arrive; the seed only shapes the first ~50 slots of actor lane choice
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02
**Disclosure-paragraph:** TBD plan 02

## RSK-partition-activated-honest-producer: `partition_activated` is a producer claim, not body-derivable; byzantine-producer risk

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The `partition_activated` bit on `LinearEndorserBlock` is a producer claim, not a property derivable from the endorser-block (EB) body. Under a multi-producer threat model with a byzantine producer, this could be mis-claimed to obtain priority service for standard-fee transactions in the same EB, undermining the ranking-block-reserved (RB-reserved) priority-only anti-bribery property. The simulator does not exercise this attack: the operational topology is multi-producer (100 nodes) but the producers are all honest by construction. A CIP-grade attacker model write-up would need either (a) to move the trigger to a body-derivable invariant (compute the bit from the priority-paying-bytes count in the EB body rather than carrying it as a producer claim), or (b) to explicitly model "honest producer" as a security assumption.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" disclosure-item 2
- `.planning/codebase/CONCERNS.md` §"Security Considerations: `partition_activated` is a producer claim, not a body-derivable property"
- `.planning/REVIEW.md` §"Cross-cutting observations" item 5
- `.planning/spikes/004-topology-and-actor-model/README.md` §Findings item 3 + §Verdict ranking item 2
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` "Defensible because" rationale — the fix path (body-derivable `partition_activated`) is straightforward and outside phase-2's scope; the simulator's current honest-producer setting bounds the attack surface
- `docs/phase-2/cardano-realism-audit.md` §"Recommended next steps" — body-derivable `partition_activated` is named as "outside phase-2's scope but worth flagging for a follow-on"
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — body-derivable refactor is out of phase-2 scope)
**Disclosure-paragraph:** TBD plan 02

## RSK-leios-spec-pre-deployment: Linear-Leios spec knobs not cross-checkable to deployed mainnet (CIP-0164 is pre-deployment)

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Several Leios-specific knobs cite CIP-0164 Table 7 with in-yet-another-markup-language (YAML) provenance (`linear-vote-stage-length-slots = 4`, `linear-diffuse-stage-length-slots = 7`, `eb-referenced-txs-max-size-bytes = 12000000`, `eb-body-validation-cpu-time-ms-per-byte = 2.15e-5`, `n = 600`, `τ = 75 %`) but Leios is pre-deployment — none of these values are cross-checkable against deployed mainnet. The Leios Frequently Asked Questions document (RB ~20 seconds, EB ~5 seconds) corroborates the cadence shape but the magnitudes lack a deployed-system anchor.
**Evidence-for:**
- `docs/phase-2/cardano-realism-audit.md` §"What lines up with mainnet" final bullet — Leios-specific knobs cite CIP-0164 Table 7; none cross-checkable to deployed mainnet
- `.planning/spikes/001-rb-cadence-and-capacity/README.md` §Findings
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` §"What lines up with mainnet" — the Leios Frequently Asked Questions document and the in-YAML provenance comments provide the closest available anchor (`CIP-0164 Table 7`)
- The pre-deployment status applies to the Leios *substrate*, not the phase-2 *pricing-mechanism* work; phase-2's contribution is mechanism-on-top-of-Leios, and Leios-substrate maturation is out of phase-2 scope
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — pre-deployment limitation is inherent to building on a pre-deployment substrate)
**Disclosure-paragraph:** TBD plan 02

## RSK-multiplier-floor-4-suite-coverage: Two suites condition exclusively on `multiplier_floor = 4`

**Category:** construct, external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Two of the seven goldens-pinned suites — `phase-2-rb-scarcity.yaml` and `phase-2-urgency-inversion.yaml` — use `multiplier_floor = 4` exclusively. The validity-threats audit assigns both LOW trust. The honest answer at the spec default `multiplier_floor = 16` is that priority demand stays too thin to surface controller drift in these two suites, which is *itself* a publishable finding ("the urgency-inversion failure mode is observable only when the floor is low enough to admit medium-urgency components to priority"). The trap is shipping these two suites' conclusions without the companion x16 run, leaving the reviewer to wonder whether the qualitative finding replicates at the spec default. A `multiplier_floor = 16` companion job per LOW suite is the cheap-test path to lifting both suites to MEDIUM.
**Evidence-for:**
- `docs/phase-2/validity-threats.md` §"phase-2-rb-scarcity.yaml" trust rating LOW + §"phase-2-urgency-inversion.yaml" trust rating LOW
- `.planning/research/PITFALLS.md` §"MOD-1: Defaults-only parameter coverage"
- `docs/phase-2/validity-threats.md` §"Recommendations to raise trust" item 3
- `CLAUDE.md` §"Calibration choices" — `multiplier_floor = 4` explicit rationale
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` disclosure-item 2 "Defensible because" — five of seven suites cover the spec default 16, so the 7-suite design as a whole is robust across the floor sweep
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely an EXP-multiplier-floor-16-companion-run cheap test under TEST-07; see DOC-03)
**Disclosure-paragraph:** TBD plan 02

## RSK-three-seed-statistical-power: Three-seed suite default cannot license tight 95% confidence intervals

**Category:** conclusion
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Every phase-2 suite runs at 3 seeds by default (`[1, 2, 3]`). Three seeds is enough to detect qualitative-direction sign flips but not enough for tight 95 % confidence intervals (CIs) on welfare-delta magnitudes. Quantitative welfare-delta claims at publication grade need either (a) re-runs at higher seed count (Phase 3's TEST-04 calibration of N = 10–20), or (b) reporting as "3-seed median, Inter-Quartile Range (IQR)" rather than as point estimates with CIs. This is the dominant conclusion-validity limit across the matrix.
**Evidence-for:**
- `docs/phase-2/validity-threats.md` §"Cross-cutting threats" — three-seed statistical power as the dominant conclusion-validity limit
- `docs/phase-2/validity-threats.md` §"Trust framework" — three seeds is "enough to detect qualitative-direction flips but not enough for tight magnitude CIs"
- `.planning/research/SUMMARY.md` §"Recommended Stack" — Paired Seed Evaluation (PSE) with Bias-corrected and accelerated (BCa) bootstrap at N = 30 (primary) / N = 20 (minimum) as the resolution path
- `.planning/research/PITFALLS.md` §"CRIT-1" — single-seed framing of the same risk for the sundaeswap smoke
**Evidence-against:**
- `.planning/family-b-results-table-2026-05-14.md` — the 3-seed table does report median and range, which is the appropriate framing for the seed count
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** EXP-canonical-variance (→ TEST-04); overlap with EXP-sign-flip-variance (→ TEST-03) — see RSK-single-seed-precision
**Disclosure-paragraph:** TBD plan 02

## RSK-unresolved-suite-claims: Four UNRESOLVED suite verdicts pending output read

**Category:** conclusion
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The validity-threats audit assigns UNRESOLVED to four suites pending a single pass through their `metrics_comparison.txt` files: `phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`. Each turns on whether the observed null result (moderate-priority-only) or standard-lane drift bound (both-dynamic suites) is confirmed by output. This is the lowest-cost trust-upgrade in the matrix — a single output-read pass flips each from UNRESOLVED to a definite MEDIUM or LOW.
**Evidence-for:**
- `docs/phase-2/validity-threats.md` §"Aggregate trust summary" UNRESOLVED row + per-suite entries
- `docs/phase-2/validity-threats.md` §"Recommendations to raise trust" item 2
**Evidence-against:**
- `docs/phase-2/validity-threats.md` §"Recommendations to raise trust" — the resolution path is explicitly low-cost ("a single pass through the `metrics_comparison.txt` files")
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (resolution is output-read in Phase 2 coverage check, not a cheap test)
**Disclosure-paragraph:** TBD plan 02

## RSK-standard-user-fee-drift-exposure: Both-dynamic standard-lane drift exposure under realistic / spike demand

**Category:** external, construct
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Both-dynamic mechanisms (partitioned and un-partitioned) preserve the multiplier-floor invariant while exposing standard users to controller drift on the standard lane. The validity-threats audit assigns UNRESOLVED to `phase-2-{moderate,realistic,sundaeswap}-both-dynamic` precisely because the verdict turns on whether observed standard-quote drift is bounded under realistic and spike-event demand. The community concern about standard users experiencing fee surges during congestion events maps directly to this risk. The Cardano Improvement Proposal (CIP) framing for both-dynamic must explicitly cite the drift-bound (or its absence) as a load-bearing claim.
**Evidence-for:**
- `docs/phase-2/validity-threats.md` §"phase-2-realistic-both-dynamic.yaml" + §"phase-2-sundaeswap-both-dynamic.yaml" — UNRESOLVED verdicts conditional on drift bound
- `docs/phase-2/validity-threats.md` §"phase-2-moderate-both-dynamic.yaml" — community-preference argument framing
**Evidence-against:**
- `docs/phase-2/cardano-realism-audit.md` §"Pricing-controller calibration" disclosure-item 2 — five of seven suites cover the spec default 16, bounding the drift-magnitude regime
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (output-read pass per RSK-unresolved-suite-claims; if bounded, mitigated; if unbounded, requires CIP disclosure)
**Disclosure-paragraph:** TBD plan 02

## RSK-cross-arch-determinism: Determinism intra-architecture only; cross-architecture not proven

**Category:** conclusion
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Determinism is asserted intra-architecture (x86_64 / glibc as the reference build environment). The repository pins golden hashes at three layers (unit-test goldens, `experiment-suite verify`, suite-level goldens), and the pricing kernel itself is integer/rational/u128 / `libm::pow` / `libm::round` / `libm::exp` throughout — bit-stable across architectures by construction. Cross-architecture continuous integration (CI) verification is **not yet built**; the residual `f64::sqrt` site in `endorsement_window_priced_blocks` (review finding CR-1) is a small but nonzero asterisk on cross-arch reproducibility because Institute of Electrical and Electronics Engineers (IEEE) 754 does not mandate bit-exact correctly-rounded sqrt across implementations. Every CIP appearance of "deterministic" or "reproducible" must be qualified as intra-architecture; a reviewer on an Advanced RISC Machine (ARM) build (increasingly common in 2026) would otherwise get different bits and write an objection.
**Evidence-for:**
- `CLAUDE.md` §"Determinism scope" — cross-architecture CI verification flagged as not yet built
- `.planning/REVIEW.md` §"Critical Findings: CR-1" — `f64::sqrt` named as the residual cross-arch site
- `.planning/codebase/CONCERNS.md` §"Determinism is intra-architecture only; cross-arch CI pipeline unbuilt"
- `.planning/research/PITFALLS.md` §"MOD-6: Determinism claim scope-creep"
- `.planning/PROJECT.md` §"Out of Scope" item 3 (cross-architecture CI deferred)
**Evidence-against:**
- `CLAUDE.md` §"Determinism scope" — intra-arch determinism is pinned at three layers and is robust; the underlying math is bit-stable across architectures given identical inputs
- The CR-1 fix (`libm::sqrt` swap) is a small one-line change that closes the only known residual
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — cross-arch CI is named out of scope in PROJECT.md)
**Disclosure-paragraph:** TBD plan 02

## RSK-admission-rejection-attribution: Gate-reject vs mempool-reject collapsed into one bool; eviction-cause attribution gap

**Category:** internal
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** `try_add_tx_to_mempool` collapses gate-reject and mempool-reject into a single `false`, losing the rich `AdmissionRejection` enum's distinction between `InsufficientMaxFee`, `ByteCapExceeded`, and `FeeOverflow` (review finding WR-2, deferred). The metrics layer cannot distinguish "fee budget exceeded" from "byte cap exceeded" rejections — important for interpreting the sustained-overload calibration regime (~97-99 % rejection rates) and acutely relevant to `phase-2-urgency-inversion` whose whole point is attributing component-0's eviction to a specific rejection cause. The fix is a backwards-compatible addition (`AdmissionRejected { reason }` event) but needs a design pass first.
**Evidence-for:**
- `.planning/REVIEW.md` §"Fix Status" WR-2 row — deferred (design needed)
- `.planning/codebase/CONCERNS.md` §"WR-2: gate-reject vs mempool-reject collapsed into a single bool"
- `docs/phase-2/validity-threats.md` §"phase-2-urgency-inversion.yaml" — WR-2 acutely relevant
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" disclosure-item 5 (~97-99 % rejection rate framing in the broader calibration context)
**Evidence-against:**
- The WR-2 fix path is well-scoped (backwards-compatible event addition); not a blocker for CIP claims that do not rely on rejection-cause attribution
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — WR-2 deferred per REVIEW.md F3; alternatively a Phase 4 cheap follow-on)
**Disclosure-paragraph:** TBD plan 02

## RSK-menu-collapse-to-advocacy: Welfare-only evidence collapses 4-way menu into single-option recommendation

**Category:** conclusion
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The Cardano Improvement Proposal (CIP) is framed as a menu of four mechanism options (priority-only-RB-reserved, priority-only-un-reserved, both-dynamic-partitioned, both-dynamic-un-partitioned) plus a single-lane Ethereum Improvement Proposal 1559 (EIP-1559) control. Un-reserved-both-dynamic dominates on every welfare metric in the published `family-b-results-table-2026-05-14.md`. A coverage check that reports only welfare metrics turns the four-option menu into a single-recommendation CIP in disguise; the non-welfare axes (anti-bribery — formal / informal / absent, signal-source-anchoring — deployed-data / spec-open / unanchored, standard-user-fee-drift exposure — none / bounded-pending-output / unbounded, implementation complexity — chain-derived state required / per-block validity rule / both) must each be surfaced as a coverage-check column so the menu remains a menu.
**Evidence-for:**
- `.planning/research/PITFALLS.md` §"CRIT-2: Menu CIP collapsing into 'this is best, others suck'"
- `.planning/family-b-results-table-2026-05-14.md` Table 2 — un-reserved-both-dynamic dominates net-utility (med +1.70e+10 vs partitioned's +3.02e+09) and inclusion rate
- `docs/phase-2/validity-threats.md` §"Per-suite claims and trust ratings" — anti-bribery framings: RB-reserved formally true under honest-producer; un-reserved variants absent
- Chung and Shi, Symposium on Discrete Algorithms (SODA) 2023 — impossibility result for joint user-incentive-compatibility, miner-incentive-compatibility, side-contract-proofness
**Evidence-against:**
- `.planning/research/FEATURES.md` (project skeleton) — non-welfare property dimensions are named as table-stakes for the coverage check; the coverage check is designed precisely to prevent this collapse (REQ-COV-03)
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (resolution is Phase 2 coverage check design, not a cheap test; cross-reference REQ-COV-03)
**Disclosure-paragraph:** TBD plan 02

## RSK-steady-state-run-length: 2000-slot run length not verified to be steady-state for every menu item

**Category:** internal, conclusion
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** All 19 phase-2 suites run 2000 slots (~10 minutes simulated time at 0.5 seconds per slot). With ranking-block (RB) probability 0.05 (~1 sample per 20 slots), 2000 slots yields ~100 controller updates — enough for transient response but not obviously enough for asymptotic behaviour. The Cardano Improvement Proposal (CIP) may claim long-run welfare behaviour for some menu items; the trap is shipping without verifying steady-state for each. The resolution is the run-length / steady-state validation (PROJECT.md Active item 7): one canonical job per menu option at 2× and 4× run length, comparing the second-half rolling welfare mean to the first-half rolling mean and checking the difference is inside seed-Inter-Quartile-Range (seed-IQR).
**Evidence-for:**
- `.planning/research/PITFALLS.md` §"MOD-2: Steady-state assumption at 2000 slots"
- `.planning/PROJECT.md` Active item 7 — run-length / steady-state validation
- `docs/phase-2/validity-threats.md` §"Cross-cutting threats" — slots: 2000 as a common cross-suite fact
**Evidence-against:**
- The controller-drift timescale is window-length × per-block-cadence ≈ 10 minutes (32 × ~20 seconds), comparable to the 2000-slot run; the cheap-test result is expected to confirm 2000 is sufficient for most menu items
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** EXP-run-length (→ TEST-06)
**Disclosure-paragraph:** TBD plan 02

## RSK-hash-diversity-policy: Hash-diversity sanity check policy (strict vs soft gate) unresolved

**Category:** conclusion
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The `hash-div` column in `family-b-results-table-2026-05-14.md` records the count of distinct `pricing_event_stream.sha256` values across the seeds of a cell. Some cells report `hash-div < N_seeds`, indicating seed correlation — the cell's results reflect a structural artefact (e.g. the same node winning the ranking-block (RB) lottery in slot 0 across seeds), not a genuinely diverse sample. PITFALLS MOD-4 recommends making hash-diversity a publication gate: any cell with `hash-div < N_seeds` cannot be promoted to BACKED in the Phase 2 coverage check without re-running with different seed values (strict policy), or must be flagged inline as WEAK with annotation (soft policy). The strict policy is cleaner; the soft policy is cheaper. The decision must land before Phase 3 begins (per STATE.md open questions).
**Evidence-for:**
- `.planning/research/PITFALLS.md` §"MOD-4: Hash-diversity sanity check skipped"
- `.planning/STATE.md` §"Open questions to resolve during phase execution" — hash-diversity policy decision named explicitly
- `.planning/family-b-results-table-2026-05-14.md` — `hash-div` column present
**Evidence-against:**
- The publication-grade resolution path (PSE BCa CIs at N = 10–20 per TEST-03 / TEST-04) inherently increases hash-diversity by virtue of running more seeds; the strict-vs-soft policy is consequential only for cells that do not get re-run at higher seed counts
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (policy decision before TEST-03 / TEST-04 begin; cross-reference COV-05)
**Disclosure-paragraph:** TBD plan 02

## RSK-welfare-as-f64-reporting: Welfare aggregates reported as `f64`; precision boundary not surfaced

**Category:** conclusion
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** Reporting outputs (`retained_value`, `net_utility`, `retained_value_ratio` and friends in the metrics collector) are computed and stored as 64-bit floating point (`f64`) and are subject to ~15-digit floating-point precision. Reported magnitudes should be interpreted to ≤ 3 significant figures; a reviewer who sees a welfare delta of +1.234e+10 vs +1.235e+10 may treat them as meaningfully different when they are inside `f64` reporting noise. The CLAUDE.md numeric-representation contract is explicit that these reporting outputs are `f64` and never feed back into simulation decisions, but the CIP-grade magnitude-resolution caveat is not currently surfaced in any CIP-pasteable text.
**Evidence-for:**
- `CLAUDE.md` §"Numeric representation contract" — reporting outputs are `f64`
- `.planning/research/PITFALLS.md` §"MIN-2: Welfare-as-f64 reporting boundary not surfaced"
**Evidence-against:**
- `CLAUDE.md` §"Numeric representation contract" — reporting `f64`s never feed back into simulation decisions, so the precision boundary affects only *interpretation*, not the deterministic outputs themselves
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (likely DISCLOSED — refresh of `cardano-realism-audit.md` per DOC-01 adds the ~3-significant-figure addendum)
**Disclosure-paragraph:** TBD plan 02

## RSK-sundaeswap-demand-staleness: SundaeSwap January 2022 launch profile is a 4-year-old retail spike, not steady-state

**Category:** external
**Verdict:** TBD plan 02 (default LIVE if ambiguous)
**Description:** The SundaeSwap January 2022 launch demand profile (`sundaeswap_moderate.yaml`) is the single most empirically-anchored demand source in phase-2 — and the Cardano Improvement Proposal (CIP) is likely to lean on it for empirical credibility. The event is now 4 years old, was a retail-frenzy spike rather than a representative steady-state, and conditioning the CIP narrative on it implicitly claims the spike *shape* is recurring or representative, which is at best unproven. The validity-threats §"phase-2-sundaeswap-singlelane" trust rating already frames this as "spike-event robustness" rather than "general behaviour under realistic demand"; the Phase 2 coverage check (per CIP-claim demand-scope) must enforce the same framing on every CIP claim that cites sundaeswap evidence.
**Evidence-for:**
- `.planning/research/PITFALLS.md` §"MIN-3: SundaeSwap demand-profile origin caveat under-stated"
- `docs/phase-2/validity-threats.md` §"phase-2-sundaeswap-singlelane.yaml" trust rating + caveats
- `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" disclosure-item 5
**Evidence-against:**
- `docs/phase-2/validity-threats.md` — sundaeswap is "close to HIGH on demand grounds; pulled down by window-length and topology caveats", making it among the strongest empirical anchors in the matrix
**Scope-of-resolution:** TBD plan 02
**EXP / Resolution:** TBD plan 02 (resolution is Phase 2 coverage-check framing per CLM-NN demand-scope column, not a cheap test)
**Disclosure-paragraph:** TBD plan 02

---

*Phase 1 inventory skeleton; 24 RSK-NN entries. Plan 01-02 finalises every `TBD plan 02` placeholder.*
