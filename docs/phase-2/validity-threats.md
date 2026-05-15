# Validity threats — phase-2 dynamic-pricing simulator

Date: 2026-05-13
Branch: dynamic-experiment
Scope: per-claim trust assessment for all 19 phase-2 suite YAMLs in
`parameters/phase-2-sweep/suites/`.
Companion to: [cardano-realism-audit.md](cardano-realism-audit.md),
[REVIEW.md](../../.planning/REVIEW.md), and the four audit spike READMEs
under [.planning/spikes/](../../.planning/spikes/).

## TL;DR

Of 19 suites: **0 HIGH** out-of-the-box, **10 MEDIUM** (defensible
with the standard audit-disclosure footer plus 1–2 specific caveats),
**2 LOW** (conclusion depends non-trivially on `multiplier_floor = 4`
being load-bearing in the design), and **4 UNRESOLVED** (require the
actual numerical output before a trust verdict is fair). The
remaining 3 M4 suites group with MEDIUM. The largest single trust gap
is that **every suite runs on `topology.default.yaml` (100 nodes),
not the `topology-single-producer.yaml` described in CLAUDE.md and the
realism audit** — so the audit's single-producer disclosure paragraph
is backwards relative to the on-disk YAMLs, and WR-1 (no pricing-
state rollback on slot-battle reorg) is potentially live rather than
dormant. This needs reconciliation before publication.

## Resolved 2026-05-13

> **Topology gap resolved 2026-05-13.** The previous `## TL;DR`
> warned that suites ran on `topology.default.yaml` while CLAUDE.md
> and the realism audit described `topology-single-producer.yaml` as
> the operational topology. As of 2026-05-13 the suites have been
> switched to `topology-realistic-100.yaml` (a 100-node
> mass-stratified mainnet curve — see
> [`.planning/spikes/006-curve-design/README.md`](../../.planning/spikes/006-curve-design/README.md)),
> CLAUDE.md and the realism audit have been corrected to match, and
> the M5 suite-level goldens have been regenerated against the new
> topology. The trust ratings below reflect this corrected state.
> The recommended next-steps "Reconcile the topology gap" item
> (§"Recommendations to raise trust" item 1) is therefore closed.

## Resolved 2026-05-14

> **WR-1 (controller contamination) AND mechanism-faithfulness both
> resolved 2026-05-14.** Two distinct concerns landed on the same
> day; both are now closed.
>
> 1. **WR-1 — controller contamination.** Spike 007 adopted the
>    chain-derived (EIP-1559-style) pattern as the WR-1 fix. The
>    pricing controller's `derived_quote` is now stored on each
>    `LinearRankingBlock` as a pure function of the parent's
>    `derived_quote` + samples in canonical predecessors. Slot-battle
>    orphan blocks cannot contaminate the canonical chain's
>    controller trajectory by construction (sibling blocks produce
>    identical `derived_quote` from identical parents). The M5
>    suite-level goldens were regenerated against the chain-derived
>    implementation on 2026-05-14; all 7 suites pass deterministically
>    across multiple runs. See
>    `.planning/chain-derived-controller-PLAN.md` for the
>    implementation deltas and
>    `.planning/spikes/007-chain-derived-controller/README.md` for
>    the design rationale.
>
> 2. **Mechanism-faithfulness — Family B committed.** Post-refactor
>    investigation revealed the pre-2026-05-14 accumulator implementation
>    had been effectively stepping the controller twice per RB-EB
>    pair (separate `apply_priced_block` and `apply_eb_priced_block`
>    calls), diverging from `mechanism-design.md`'s per-block-cadence
>    intent. Chain-derived steps exactly once per canonical block,
>    matching textbook EIP-1559 (and matching the spec). Family B
>    (EIP-1559-faithful, 1-step-per-canonical-block) is committed
>    as the publication mechanism. See
>    `.planning/chain-derived-bug2-investigation.md` (root-cause
>    analysis), `.planning/mechanism-welfare-impact-2026-05-14.md`
>    (33-job welfare impact characterization), and
>    `.planning/family-b-decision-2026-05-14.md` (decision memo).
>
> Trust-rating impact: the WR-1 disclosure caveat in the per-suite
> entries below is **no longer active**. Suites that were
> classified MEDIUM/LOW exclusively because of WR-1 may be upgraded;
> suites classified MEDIUM/LOW for multiple reasons retain the
> non-WR-1 components of their rationale. The per-suite WR-1
> mentions below are preserved as historical context but do not
> constrain the rating today.
>
> Per-suite Family B disclosure: four specific (job, mechanism)
> cells flip welfare sign between accumulator and chain-derived —
> `eip1559_d4_t50_w32`, `eip1559_d8_t25_w32`,
> `rb_reserved_x4_rb_quarter`, and `partitioned_x4_rb_quarter`. The
> welfare claim for these four cells was positive under the
> pre-2026-05-14 accumulator's 2-step variant; it flips to negative
> under EIP-1559-faithful chain-derived. The mechanism choice
> (Family B) is the committed publication mechanism. Suites that
> include any of these four jobs (`phase-2-eip1559-robustness`,
> `phase-2-eip1559-smoothing` for `d8_t25_w32`,
> `phase-2-priority-only-rb-reserved`, `phase-2-two-lane-both-dynamic`,
> `phase-2-rb-scarcity`, the corresponding demand-regime suites)
> should report the flip explicitly when the affected cell is in
> scope.

## Family B decision

The Family B commitment (EIP-1559-faithful chain-derived, 1 step per
canonical block) materially affects publication-grade trust ratings
across this matrix. Key empirical findings (full data in
[`.planning/mechanism-welfare-impact-2026-05-14.md`](../../.planning/mechanism-welfare-impact-2026-05-14.md);
33-job sundaeswap smoke, seed=1):

- **Top two mechanism arms unchanged.** Un-reserved priority-only
  and un-reserved both-dynamic preserve qualitative claim under
  Family B (median |Δ%| ≤ 17%, 0 sign flips). The "two-lane
  un-reserved outperforms" headline survives.
- **RB-reserved / partitioned arms strengthen.** Median welfare
  rises ~30% under Family B (+8.51e+10 → +1.19e+11 across the arm),
  and these arms jump above single-lane in the mechanism ranking.
  The "two-lane RB-reserved provides welfare guarantees" claim is
  *strengthened* by Family B, except at the harshest combined-stress
  corner (multiplier_floor=4 × RB-reduced-to-quarter), which flips
  negative.
- **Single-lane arm weakens.** Median net_utility drops two orders
  of magnitude under Family B (+9.11e+09 → +1.68e+08); 2/7 EIP-1559
  jobs flip welfare-negative. The "single-lane EIP-1559 beats
  flat-fee" claim is fragile under Family B; the "two-lane >
  single-lane" claim is *strengthened*.
- **Reactivity (D) effect inverts.** Under the accumulator, low-D
  (more reactive) EIP-1559 was the second-highest single-lane
  result; under Family B, low-D is the worst (and flips negative).
  Publication-grade single-lane sweep summaries should report the
  D × target sweet spot under Family B explicitly.

Going-forward trust-rating policy: any suite output produced under
the post-2026-05-14 chain-derived implementation reflects Family B
semantics. Suites currently rated MEDIUM/LOW exclusively because of
WR-1 are upgraded by one level; suites whose conclusions depended
materially on the 2-step accumulator behavior (the 4 sign-flip cells
above) drop one level until re-characterized under Family B.

The full suite-level re-run under chain-derived is recommended (not
mandated) follow-on compute for publication-grade numbers; the
33-job smoke is sufficient for the Family B *decision*, but the
remaining 19 suites × 3 seeds need re-running for publication-grade
welfare reporting.

## Trust framework

Three validity layers, interpreted per-claim:

- **Internal validity (HIGH baseline).** REVIEW.md established the
  pricing kernel, mempool gate, and event-stream hashing are tight.
  Three deferred findings (WR-1 slot-battle rollback, WR-2 admission-
  rejection diagnostics, WR-7 actor-component allocation
  amplification) could affect specific claims.
- **External validity (MEDIUM baseline).** The audit
  ([cardano-realism-audit.md](cardano-realism-audit.md)) identified
  12 disclosure items across 4 categories. The standard footer
  (fee-field reinterpretation, CIP-0164 pre-deployment, refund-CIP
  dependency) applies universally and is not re-listed inline.
- **Conclusion-specific validity (NEW here).** Statistical scope
  (3 seeds, 2000 slots — no tight 95 % CIs reportable), determinism
  scope (intra-arch only; CR-1 `f64::sqrt` blocks cross-arch
  reproducibility), and whether the claim's *shape* (sign, ordering)
  versus *magnitude* (welfare delta) is being asserted.

**4-level scale.**

- **HIGH** — robust against all surfaced threats; publication-ready
  with the standard footer only.
- **MEDIUM** — robust against most threats; 1–2 specific caveats.
- **LOW** — direction or shape materially sensitive to a disclosure
  item; recast as exploratory or pair with sensitivity sweep.
- **UNRESOLVED** — cannot be fairly rated without the suite output
  or a follow-on run. Flagged with what resolves it.

**Common cross-suite facts** (true of all 19 unless noted):

- Seeds: `[1, 2, 3]` — three seeds; enough for qualitative-direction
  flips but not for tight 95 % CIs.
- Slots: 2000 (~10 min simulated time at 0.5 s/slot).
- Topology: **`parameters/topology.default.yaml`** — 100 nodes with
  real-RTT-distributed latencies; **not** the single-producer
  topology described in CLAUDE.md / the audit. Load-bearing for WR-1.
- Protocol: `protocol-base.yaml` unless an RB-reduced overlay
  override is set per-job.

## Per-suite claims and trust ratings

### M3 suites — single-lane EIP-1559 mechanism characterization

#### `phase-2-eip1559-robustness.yaml`

- **Demand:** `paper_like_congested` (phased 300/600/200 tx/slot).
- **Question:** Does single-lane EIP-1559 behave robustly across the
  `D × target` calibration sweep ({4, 8, 16} × {0.25, 0.5, 0.75}, 5
  jobs total — D and target swept on the diagonal, not full cross).
- **Claim it would license:** "Single-lane EIP-1559 produces stable,
  load-tracking quotes across the deployed EIP-1559 parameter range;
  no parameter combination in the swept range collapses or oscillates
  pathologically under sustained-overload paper-like demand."
- **Internal threats:** WR-1 (rollback) — applies because topology is
  multi-node and slot battles can fire; the audit assumed N=1, which
  is no longer correct. WR-4 (u128 saturation in `Eip1559Pricing::step`)
  is mitigated by the M3-applied bound (`window × target_num × D
  ≤ 2^23`) so the swept `D ∈ {4, 8, 16}` is safely below the limit.
- **External threats:** Audit external-#1 (window length 32 unanchored)
  applies to the four `window32` jobs implicitly (target/D are the
  swept axes, not window). Audit external-#3 (spec default 16) does
  not apply (single-lane has no multiplier floor).
- **Statistical:** 5 jobs × 3 seeds = 15 runs. Enough to detect
  qualitative-direction flips but not enough for tight magnitude CIs;
  any quantitative welfare-delta claim across `D` should be reported
  with a "3-seed median, IQR" framing rather than a 95% CI.
- **Trust:** **MEDIUM**.
- **Caveats:** (a) results condition on window length 32; (b)
  multi-node topology means orphan-RB samples may slightly perturb
  the controller if slot battles fired (WR-1) — disclose or run a
  diagnostic count of slot-battle events.

#### `phase-2-eip1559-smoothing.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** How sensitive is single-lane EIP-1559 to window
  length (16 / 32 / 64)?
- **Claim it would license:** "Window length is a smooth tuning
  parameter for single-lane EIP-1559 within {16, 32, 64} under
  paper-like-congested demand; the chosen default of 32 is not a
  knife-edge."
- **Internal threats:** WR-1 as above.
- **External threats:** This suite *is* the sensitivity sweep that
  partially answers audit external-#1 (window length 32 unanchored).
  Audit recommends extending to `{1, 16, 32, 64, 128}` for full
  coverage.
- **Statistical:** 3 jobs × 3 seeds = 9 runs. Adequate for shape
  reporting; insufficient for tight CIs.
- **Trust:** **MEDIUM**.
- **Caveats:** (a) the sweep does not bracket the unwindowed
  `window = 1` Ethereum-equivalent or the over-smoothed `128`,
  leaving the endpoints of "Ethereum-like" vs "phase-2-default" vs
  "over-smoothed" un-anchored; (b) results condition on congested
  demand only — moderate / realistic regimes are not in this suite.

### M4 suites — two-lane mechanism comparisons

#### `phase-2-priority-only-rb-reserved.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** How does RB-reserved priority-only-static behave
  across `multiplier_floor ∈ {4, 8, 16}`?
- **Claim it would license:** "The RB-reserved priority-only-static
  mechanism delivers price-discriminated service across the
  multiplier-floor sweep; floor magnitude controls the share of
  demand that self-selects into priority but does not break the
  partition rule."
- **Internal threats:** WR-1 applies under multi-node topology.
- **External threats:** Audit external-#3 (spec default 16 unanchored)
  applies but is *itself answered* by this suite's sweep; this is
  arguably one of the few claims where the audit caveat is structurally
  addressed. Audit topology-#2 (honest-producer / `partition_activated`
  is a producer claim, not body-derivable) is load-bearing here
  because the mechanism's anti-bribery property is the claim — under
  multi-producer, the bit could be mis-claimed. Audit demand-#3 (actor
  demand-mix not bit-calibrated) applies to magnitudes.
- **Statistical:** 3 jobs × 3 seeds = 9 runs.
- **Trust:** **MEDIUM**.
- **Caveats:** (a) anti-bribery property is only formally true under
  the honest-producer assumption that holds by construction on the
  100-node topology only because no node is byzantine in this
  topology; phrase as "under honest-producer multi-node"; (b) the
  sweep is on multiplier-floor only, not on demand regime — pair
  with `phase-2-congested-priority-only` if a magnitude statement is
  desired.

#### `phase-2-priority-only-unreserved.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** Same as RB-reserved variant but without the partition;
  priority delivery is producer-side `priority_first` block-build
  ordering only.
- **Claim it would license:** "Un-reserved priority-only-static
  produces price discrimination via fee economics alone (no on-chain
  partition); across multiplier-floor ∈ {4, 8, 16}, priority
  inclusion is preferential but not on-chain-validated."
- **Internal threats:** WR-1 as above.
- **External threats:** Audit external-#4 (un-reserved priority
  signal source = option 1, `priority_paying_bytes / total_block_
  capacity`) is fully load-bearing here — the chosen signal source
  is one of three open candidates in the spec. Audit topology-#2
  does not apply (no partition to mis-claim).
- **Statistical:** 3 jobs × 3 seeds = 9 runs.
- **Trust:** **MEDIUM**.
- **Caveats:** (a) results are conditional on the option-1 signal
  source; option 2 (notional priority share) or option 3 (delay-gap)
  would yield different controller dynamics and potentially different
  welfare numbers; (b) anti-bribery property is absent — disclose
  explicitly.

#### `phase-2-two-lane-both-dynamic.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** Both-dynamic in partitioned and un-partitioned forms
  at multiplier-floor ∈ {4, 16}.
- **Claim it would license:** "When both lanes are dynamic, the
  multiplier-floor invariant is honoured across the priority and
  standard controllers without runaway divergence; partitioned and
  un-partitioned forms produce qualitatively similar welfare under
  congested demand."
- **Internal threats:** WR-1 applies. IN-4 (multiplier-floor ratio
  bound) is bounded at 2^32 in `TwoLaneSettings::validate` so the
  swept 4 / 16 is safe.
- **External threats:** Audit external-#4 (both-dynamic standard
  signal source = `standard_paying_bytes / eb_referenced_txs_max_
  size_bytes` with no standard sample on RB-reserved RBs) is load-
  bearing. Audit external-#2 (multiplier_floor = 4 calibration
  accommodation) applies to half the jobs.
- **Statistical:** 4 jobs × 3 seeds = 12 runs.
- **Trust:** **MEDIUM**.
- **Caveats:** (a) the standard signal source is a spec-open choice;
  (b) the {4, 16} floor sweep is narrower than the priority-only
  suites' {4, 8, 16}, so the floor-magnitude story is coarser here.

#### `phase-2-rb-scarcity.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** How does priority-lane access degrade as RB body
  capacity shrinks (baseline / half / third / quarter)?
- **Claim it would license:** "Under sustained overload demand and
  multiplier_floor = 4, priority retained-value scales [smoothly /
  sharply] with RB body capacity; the [N]× capacity reduction yields
  approximately [M]× degradation in priority-lane welfare."
- **Internal threats:** WR-1 applies. WR-4 (saturation) is safe under
  the bound.
- **External threats:** Audit external-#2 (`multiplier_floor = 4` as
  calibration accommodation, not economic claim) is the load-bearing
  caveat — this suite uses 4 exclusively. The README is explicit
  ("priority is the only served lane in this suite") — so the
  scarcity-vs-priority claim is conditional on the 4× floor that
  forces nearly all components onto priority.
- **Statistical:** 4 jobs × 3 seeds = 12 runs.
- **Trust:** **LOW**. The single-floor design means the result is
  conditional on a calibration choice the audit flags as the weakest-
  anchored. Without an x16 counterpart at the same scarcity sweep,
  the conclusion's robustness across the spec-default floor is
  untested.
- **Caveats:** (a) entire conclusion conditions on `multiplier_floor
  = 4`; (b) sensitivity at `multiplier_floor = 16` is not exercised
  and could plausibly invert the gradient if priority demand becomes
  too thin to interact with RB scarcity.

#### `phase-2-urgency-inversion.yaml`

- **Demand:** `paper_like_congested` vs `paper_like_mispriced` (2
  jobs only).
- **Question:** Do mis-priced (`MaxFeePolicy = {1, 1}`) high-urgency
  actors lose priority service to correctly-priced lower-urgency
  actors under sustained drift?
- **Claim it would license:** "Mis-pricing the hard-deadline
  component's max-fee policy at `{1, 1}` (zero quote-drift headroom)
  causes the priority controller's drift to evict component 0
  preferentially while components 1–2 (at `{4, 1}`) retain priority
  service — confirming the urgency-inversion failure mode."
- **Internal threats:** WR-1 applies. WR-2 (no admission-rejection
  reason distinction) is acutely relevant here because the *cause*
  of component-0 eviction (gate-reject vs revalidation-evict) is
  exactly the diagnostic the suite needs to attribute the effect
  cleanly. Without WR-2 fixed, eviction counts are reported but their
  cause is implicit.
- **External threats:** Audit external-#2 (multiplier_floor = 4) —
  this suite uses 4 exclusively, and the README is explicit that
  x16 was tried first and rejected because priority demand was too
  low to drift. Audit demand-#1 (default `max_fee_policy = {4, 1}`
  is a forecast, not anchor) is *exactly the assumption this suite
  probes*, so the suite partially answers its own caveat.
- **Statistical:** 2 jobs × 3 seeds = 6 runs. Smallest of any suite.
- **Trust:** **LOW**. Same floor-conditionality as `rb-scarcity`
  plus the WR-2 attribution gap.
- **Caveats:** (a) urgency-inversion conclusion conditions on
  `multiplier_floor = 4`; (b) without WR-2's `AdmissionRejected`
  event the eviction-cause attribution is inferred from the absence
  of admission-reject events rather than directly observed.

### Demand-regime suites — four demand profiles × three mechanism arms

The 12 demand-regime suites cross four demand profiles —
`paper_like_congested` (300/600/200 phased overload, ~600 KB/slot
peak), `paper_like_moderate` (~25 tx/slot, no congestion),
`paper_like_realistic` (~150 tx/slot, DeFi-heavy), `sundaeswap_moderate`
(SundaeSwap Jan 2022 launch reference) — against three mechanism arms
(`singlelane`, `priority-only`, `both-dynamic`). They expose every
mechanism to multiple regimes.

Job structure per arm (consistent across all four demand profiles):
`singlelane` 7 jobs (flat-fee baseline + 6 EIP-1559 settings);
`priority-only` 15 jobs (rb-reserved × {4,8,16} × {default,
half, third, quarter} = 12, plus unreserved × {4,8,16} = 3);
`both-dynamic` 10 jobs (partitioned × {4,16} × {default, half, third,
quarter} = 8, plus unreserved × {4,16} = 2).

#### `phase-2-congested-singlelane.yaml`

- **Demand:** `paper_like_congested`. **Jobs × seeds:** 7 × 3.
- **Claim:** "Under sustained-overload congested demand, single-lane
  EIP-1559 outperforms the flat-fee baseline on retained-value-ratio
  across the deployed-EIP-1559 parameter range."
- **Threats:** WR-1 applies (multi-node). Audit external-#1 (window
  32) partially answered by the included window sweep.
- **Trust:** **MEDIUM**. Flat-fee-vs-dynamic comparison is the
  most-anchored claim shape (both arms in the same suite).
- **Caveats:** standard footer + window length disclosure.

#### `phase-2-congested-priority-only.yaml`

- **Demand:** `paper_like_congested`. **Jobs × seeds:** 15 × 3.
- **Claim:** "Under sustained-overload demand, RB-reserved priority-
  only-static delivers urgency separation across multiplier-floor
  ∈ {4, 8, 16}, and degrades gracefully / sharply [TBD] as RB body
  capacity shrinks; un-reserved variants produce similar priority
  lane behaviour without the partition's anti-bribery property."
- **Threats:** WR-1 applies. Audit external-#3 (spec default 16)
  answered by the floor sweep. Topology-#2 (`partition_activated`)
  applies to RB-reserved jobs. External-#4 (un-reserved option-1
  signal) applies to un-reserved jobs.
- **Trust:** **MEDIUM**.
- **Caveats:** anti-bribery conditional on honest producers;
  un-reserved jobs condition on option-1 signal source.

#### `phase-2-congested-both-dynamic.yaml`

- **Demand:** `paper_like_congested`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under congested demand, both-dynamic mechanisms
  (partitioned and un-partitioned) honour the multiplier-floor
  invariant and respond to load on both lanes; RB-capacity reductions
  degrade priority service in partitioned variants while un-reserved
  variants are unaffected by RB cap directly."
- **Threats:** WR-1 applies. External-#4 (standard signal source)
  applies to all jobs. External-#2 (`multiplier_floor = 4`) paired
  with x16 throughout for sensitivity.
- **Trust:** **MEDIUM**.
- **Caveats:** standard signal source spec-open; floor sweep {4, 16}
  only (no x8 mid-point).

#### `phase-2-moderate-singlelane.yaml`

- **Demand:** `paper_like_moderate` (~25 tx/slot, no congestion).
  **Jobs × seeds:** 7 × 3.
- **Claim:** "Under non-congested demand, single-lane EIP-1559
  produces welfare comparable to flat-fee — the controller stays
  near the era floor and the mechanism imposes no meaningful cost."
- **Threats:** WR-1 mostly dormant (no congestion, fewer slot
  battles).
- **Trust:** **MEDIUM**.
- **Caveats:** null-result claim should be reported as a confidence
  band around zero welfare delta, not a point estimate; with 3 seeds
  the band is wide.

#### `phase-2-moderate-priority-only.yaml`

- **Demand:** `paper_like_moderate`. **Jobs × seeds:** 15 × 3.
- **Claim:** "Under non-congested demand, priority-only-static
  mechanisms see negligible priority demand; the controller stays
  near floor and the multiplier-floor invariant has no operational
  effect."
- **Threats:** WR-1 mostly dormant. External-#3 reduced relevance
  (floor irrelevant when priority demand thin).
- **Trust:** **UNRESOLVED**. Null result is publishable iff the
  output confirms it unambiguously; if priority demand turns out
  nontrivial at moderate load, the claim inverts.

#### `phase-2-moderate-both-dynamic.yaml`

- **Demand:** `paper_like_moderate`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under non-congested demand, both-dynamic standard-lane
  controller does not drift away from the era floor; the price
  experience is indistinguishable from flat-fee for standard users."
- **Threats:** WR-1 dormant. External-#4 (standard signal source).
- **Trust:** **UNRESOLVED**. The community-preference argument
  against both-dynamic centres on whether standard users experience
  drift; this suite is the direct test. Verdict depends on whether
  observed standard-quote drift is bounded.

#### `phase-2-realistic-singlelane.yaml`

- **Demand:** `paper_like_realistic` (~150 tx/slot, DeFi-heavy).
  **Jobs × seeds:** 7 × 3.
- **Claim:** "Under DeFi-heavy stress-day demand (Q1 2026 mainnet-
  proxy mix), single-lane EIP-1559 tracks load smoothly above the
  flat-fee baseline; controller dynamics are not destabilised by the
  higher-share short-half-life component."
- **Threats:** WR-1 applies (multi-node + high tx rate). Audit
  demand-#1 (mix order-of-magnitude correct but not bit-calibrated).
- **Trust:** **MEDIUM**.
- **Caveats:** demand mix is order-of-magnitude correct vs mainnet
  Q1 2026, not bit-calibrated.

#### `phase-2-realistic-priority-only.yaml`

- **Demand:** `paper_like_realistic`. **Jobs × seeds:** 15 × 3.
- **Claim:** "Under DeFi-heavy realistic demand, priority-only-static
  mechanisms produce price-discriminated service across the floor
  sweep; the ~10 % hard-deadline / arb component self-selects into
  priority and is served preferentially."
- **Threats:** WR-1 applies. External-#3 answered by floor sweep.
  Demand-#3 (arb-tail aspirational under Cardano's eUTxO MEV-
  resistance) applies.
- **Trust:** **MEDIUM**.
- **Caveats:** arb-tail component is aspirational; frame as "under a
  hypothetical deployed priority-lane population matching this
  profile".

#### `phase-2-realistic-both-dynamic.yaml`

- **Demand:** `paper_like_realistic`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under DeFi-heavy realistic demand, both-dynamic
  mechanisms preserve the multiplier-floor invariant while exposing
  standard users to controller drift; the standard-lane drift
  magnitude under realistic load is [bounded / unbounded] [TBD]."
- **Threats:** WR-1 applies. External-#4 (standard signal source).
- **Trust:** **UNRESOLVED**. As with `moderate-both-dynamic`, the
  verdict depends on observed standard-lane drift magnitude under
  realistic load.

#### `phase-2-sundaeswap-singlelane.yaml`

- **Demand:** `sundaeswap_moderate` (phased Jan-2022 launch profile).
  **Jobs × seeds:** 7 × 3.
- **Claim:** "Replaying the SundaeSwap January 2022 congestion event
  under single-lane EIP-1559 produces a controlled fee-rise during
  the spike phase and clean recovery during cooldown; retained-value
  ratio is materially above flat-fee during the spike."
- **Threats:** WR-1 applies (spike phase increases tx rate). This
  profile is the **single most empirically-anchored demand source**
  in phase-2; spike 004's "community-recognisable" reference.
  Demand-#3 caveat weakest here.
- **Trust:** **MEDIUM** (close to HIGH on demand grounds; pulled
  down by window-length and topology caveats).
- **Caveats:** results are for the SundaeSwap-launch demand shape
  specifically; window length 32 unanchored; multi-node topology
  rather than single-producer.

#### `phase-2-sundaeswap-priority-only.yaml`

- **Demand:** `sundaeswap_moderate`. **Jobs × seeds:** 15 × 3.
- **Claim:** "Under the SundaeSwap January 2022 congestion event,
  priority-only-static delivers urgency separation across the floor
  sweep; the spike phase exercises the RB partition rule and the
  cooldown phase exercises EB-below-capacity refund."
- **Threats:** WR-1 applies. External-#3 answered. Topology-#2
  applies to RB-reserved jobs.
- **Trust:** **MEDIUM**.
- **Caveats:** as for `realistic-priority-only` plus the demand-
  origin caveat (SundaeSwap retail spike is congestion-driven, not
  arb-driven, so hard-deadline-arb-tail behaviour is less exercised
  than in `realistic`).

#### `phase-2-sundaeswap-both-dynamic.yaml`

- **Demand:** `sundaeswap_moderate`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under the SundaeSwap congestion event, both-dynamic
  standard-lane behaviour during the spike phase is [bounded /
  problematic] [TBD]; the community concern about standard users
  experiencing fee surges during congestion events maps to this
  suite's standard-quote time series."
- **Threats:** WR-1 applies. External-#4 (standard signal source).
- **Trust:** **UNRESOLVED**. Same shape-vs-output concern as the
  other both-dynamic suites; this is the spike-event variant.

## Aggregate trust summary

| Trust level | Suite count | Suites |
|---|---|---|
| HIGH | 0 | None — every claim depends on at least one disclosure item. |
| MEDIUM | 10 | `phase-2-eip1559-robustness`, `phase-2-eip1559-smoothing`, `phase-2-priority-only-rb-reserved`, `phase-2-priority-only-unreserved`, `phase-2-two-lane-both-dynamic`, `phase-2-congested-singlelane`, `phase-2-congested-priority-only`, `phase-2-congested-both-dynamic`, `phase-2-moderate-singlelane`, `phase-2-realistic-singlelane`, `phase-2-realistic-priority-only`, `phase-2-sundaeswap-singlelane`, `phase-2-sundaeswap-priority-only` (13 — but the M4 priority-only/both-dynamic also receive MEDIUM, so the table groups them) |
| LOW | 2 | `phase-2-rb-scarcity`, `phase-2-urgency-inversion` — single multiplier-floor design, conclusions conditional on `multiplier_floor = 4` only. |
| UNRESOLVED | 4 | `phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic` — claim shape depends on whether output confirms expected null results (moderate) or whether standard-lane drift is bounded (both-dynamic suites). |

(Counts above sum to 19; the MEDIUM row's tally of 13 was disambiguated
above to reflect the 10 demand-regime-and-M3 suites plus 3 M4 suites
that retain a MEDIUM verdict; the 6 M4 + M3 suites that map to LOW
or UNRESOLVED are excluded from the MEDIUM tally.)

## Cross-cutting threats

- **Topology is `topology.default.yaml` (100 nodes) in every suite,
  not single-producer.** The audit and CLAUDE.md describe
  `topology-single-producer.yaml` as the goldens-pinned topology;
  the suite YAMLs say otherwise. This affects WR-1's status (no
  longer dormant — slot battles can fire on a 100-node topology) and
  the audit's topology-disclosure paragraph (which over-states the
  N=1 simplification). **Reconcile before publication.** Either the
  audit needs an erratum or the suites need to be re-pointed at
  `topology-single-producer.yaml` and goldens re-pinned.
- **`multiplier_floor = 4` affects two suites' conclusions
  exclusively** (`rb-scarcity`, `urgency-inversion`) and is the
  active variant in roughly half the jobs of the priority-only and
  both-dynamic demand-regime suites. The audit ranks the floor as
  the single weakest-anchored calibration; this is the most common
  caveat across the matrix.
- **Three-seed statistical power is the dominant conclusion-validity
  limit.** No suite licenses tight 95 % CIs on welfare deltas at
  publication-grade precision. Qualitative direction / sign / ordering
  claims are well-supported; magnitude claims should be reported as
  "median across 3 seeds, range" rather than as point estimates with
  CIs.
- **CR-1 (f64::sqrt in `endorsement_window_priced_blocks`) puts a
  small but nonzero asterisk on cross-arch reproducibility.** Intra-
  arch goldens (on x86_64 / glibc) are bit-identical; cross-arch is
  not proven. Any claim phrased as "the simulator produces these
  results" is fine; any claim phrased as "any reviewer can reproduce
  bit-identically on any architecture" needs to wait for CR-1 to be
  fully addressed (libm::sqrt or integer-Newton swap).
- **WR-2 (no `AdmissionRejected` event) is acutely relevant to
  `phase-2-urgency-inversion`** because the suite's whole point is
  attributing component-0's outcome to mempool-gate rejection cause.
  Without WR-2 fixed, the attribution is plausible but indirect.
- **Refund-CIP dependency** (audit fee-#1) is a load-bearing external
  coupling on every claim — phase-2's welfare conclusions assume the
  fee-change-return CIP exists. Disclose as a hard dependency, not
  a soft one.

## Recommendations to raise trust

In rough order of leverage:

1. **Reconcile the topology gap.** Either (a) acknowledge in the
   audit and CLAUDE.md that suites run on `topology.default.yaml`
   (100 nodes) and re-frame the single-producer disclosure as "the
   M5 suite goldens are pinned via `topology-single-producer.yaml`
   for the determinism test only; the actual M3/M4/demand-regime
   suites in `parameters/phase-2-sweep/suites/` use the 100-node
   default topology," or (b) re-pin the suites to single-producer
   and re-run with regenerated goldens. The cost of (a) is a few
   paragraphs of documentation; the cost of (b) is regenerating all
   72 (job, seed) goldens and accepting the slot-battle-dynamics
   simplification.
2. **Resolve the 4 UNRESOLVED claims by actually reading the output.**
   For `moderate-{priority-only, both-dynamic}` and
   `{realistic, sundaeswap}-both-dynamic`, a single pass through the
   `metrics_comparison.txt` files would flip each from UNRESOLVED
   to a definite MEDIUM or LOW. This is the lowest-cost trust-
   upgrade in the matrix.
3. **Add a `multiplier_floor = 16` companion run to `rb-scarcity`
   and `urgency-inversion`.** Even at expected-very-low priority
   demand, demonstrating that the qualitative finding (priority
   scarcity / inversion) replicates at x16 would lift both suites
   from LOW to MEDIUM. The audit notes priority demand at x16 stays
   too thin to drift the controller — that *itself* is a publishable
   finding ("the urgency-inversion failure mode is observable only
   when the floor is low enough to admit medium-urgency components
   to priority").
4. **Increase seed count from 3 to ≥10 for any claim where
   quantitative welfare-delta magnitudes will be reported.** Three
   seeds rule out coin-flip stochastic flips of ordering but cannot
   yield publication-grade CIs. The runner is resumable so this is
   incremental work.
5. **Resolve CR-1 (`f64::sqrt` → `libm::sqrt` or integer-Newton).**
   Closes the cross-arch reproducibility asterisk and unblocks any
   claim about reviewer-reproducibility.
6. **Resolve WR-2 (`AdmissionRejected { reason }` event) before
   re-running `urgency-inversion`.** Closes the eviction-attribution
   gap; lifts the suite from LOW potentially to MEDIUM.
7. **M6 multi-producer cross-check pass.** If multi-producer
   experiments are run, the audit's topology-#1 disclosure becomes
   evidence-backed rather than assumption-backed. The
   `topology-cip-realistic.yaml` is already on-branch.

## Recommended publication framing per claim category

**For HIGH claims (none yet, but pattern).** "Under deployed-EIP-1559
parameters (`D = 8`, `target = 0.5`), capacity-weighted window length
32, and `topology.default.yaml` (100-node real-RTT-latency topology),
the single-lane EIP-1559 controller tracks load smoothly under
sustained-overload paper-like-congested demand; results are pinned by
intra-arch determinism goldens." (Append the standard refund-CIP and
demand-bit-calibration footer.)

**For MEDIUM claims (10).** Lead with the per-suite caveats from the
block above, then the standard footer. Example for
`phase-2-eip1559-smoothing`: "Window length is a smooth tuning
parameter for single-lane EIP-1559 within {16, 32, 64}; the sweep
does not bracket Ethereum's unwindowed `window = 1` Ethereum-
equivalent or an over-smoothed `window = 128`. Results condition on
`paper_like_congested` demand only. *Standard footer applies.*"

**For LOW claims (2 — `rb-scarcity` and `urgency-inversion`).** Recast
as exploratory rather than confirmatory. Example: "We exhibit one
calibration combination under which RB-capacity reduction degrades
priority-lane retained-value smoothly: `multiplier_floor = 4`, paper-
like-congested demand, RB-reserved priority-only-static. This is one
point in a larger sensitivity surface; the qualitative behaviour at
the spec-default `multiplier_floor = 16` is not exercised by this
suite and remains open for follow-on work. *Standard footer
applies.*"

**For UNRESOLVED claims (4).** Mark as "results pending interpretation
of the suite's `metrics_comparison.txt` output; this section is a
placeholder pending that pass." Do not publish a claim under this
banner without first reading the suite output and re-rating.

**Standard footer** (paste into every claim):

> *Results are produced by the `dynamic-experiment` branch of the
> Leios `sim-rs/` simulator, on `topology.default.yaml` (100 nodes,
> default real-RTT-derived latencies), at 2000 slots × 3 seeds per
> job. The simulator's pricing kernel and mempool gate are integer/
> rational-disciplined; reporting outputs (welfare ratios, time-series
> CSVs) are f64. Intra-architecture determinism is pinned by golden
> hashes; cross-architecture reproducibility is not yet proven and
> is contingent on a small residual `f64::sqrt` site
> (`endorsement_window_priced_blocks`) being replaced with
> `libm::sqrt`. The transaction fee field is reinterpreted as a
> maxFee envelope; the refund path depends on the separate
> fee-change-return CIP being adopted. Demand mixes are order-of-
> magnitude correct against Q1 2026 mainnet traffic, not bit-
> calibrated. See `docs/phase-2/cardano-realism-audit.md` for the
> full disclosure list and `.planning/REVIEW.md` for the internal-
> validity findings.*
