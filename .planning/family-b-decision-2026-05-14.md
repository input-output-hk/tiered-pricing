# Family B decision: EIP-1559-faithful chain-derived mechanism

Date: 2026-05-14
Status: COMMITTED for publication

## Decision

Phase-2 publishes welfare results under Family B: the chain-derived
controller pattern that steps `quote_per_byte` exactly once per
canonical Ranking Block, matching EIP-1559's per-block update
semantics. The pre-2026-05-14 accumulator implementation effectively
stepped twice per RB-EB pair (via separate `apply_priced_block` and
`apply_eb_priced_block` calls) — this was unintentional
implementation behavior diverging from `mechanism-design.md`'s
per-block-cadence intent, and has been corrected.

## What changed

- **Implementation:** chain-derived refactor (spike 007, applied
  2026-05-14) replaced node-local mutable controller state with
  pure-function-of-chain semantics. Every `LinearRankingBlock`
  carries `derived_quote: PerLaneQuote` as a pure function of the
  parent block's chain-derived state plus the samples emitted by
  canonical predecessors within the window. There is no node-local
  accumulator; the canonical chain itself carries the controller
  state.
- **Bug #1 fixed:** post-refactor, a one-step lag in
  `current_chain_tip_quote` was identified (the representative
  observed the parent's stored `derived_quote` rather than the
  prediction for the next child) and fixed. See
  `.planning/chain-derived-bug-investigation.md`.
- **"Bug #2" reframed:** a residual empirical divergence between
  accumulator and chain-derived persisted on zero-slot-battle
  multi-node runs even after bug #1's fix. Root-cause analysis (see
  `.planning/chain-derived-bug2-investigation.md`) established that
  this was not a chain-derived bug but rather an architectural
  difference: the accumulator was effectively performing two
  controller steps per RB-EB pair (one at RB publish via
  `apply_priced_block`, one at deferred EB validation via
  `apply_eb_priced_block`); chain-derived fires exactly one step
  per canonical block, which is the EIP-1559-faithful cadence.

## Welfare-impact summary

(33-job sundaeswap smoke, seed=1, 2000 slots; per
[`.planning/mechanism-welfare-impact-2026-05-14.md`](mechanism-welfare-impact-2026-05-14.md).
A = accumulator, 2-step variant; B = chain-derived, 1-step
EIP-1559-faithful.)

| Arm | A → B effect | Sign flips | Verdict |
|---|---|---|---|
| Un-reserved priority-only | ≈ unchanged (median \|Δ%\| 15%) | 0/3 | robust |
| Un-reserved both-dynamic | ≈ unchanged (median \|Δ%\| 15%) | 0/2 | robust |
| RB-reserved priority-only | +30% median welfare | 1/12 (`x4_rb_quarter`) | gets better |
| Partitioned both-dynamic | +30% median welfare | 1/8 (`x4_rb_quarter`) | gets better |
| Single-lane EIP-1559 | Median drops 2 orders of magnitude | 2/7 (`d4_t50_w32`, `d8_t25_w32`) | collapses |

Sign flips occur at the corners of the parameter space (most
reactive controllers × harshest scarcity × tightest multiplier
floor). The middle of the space — the un-reserved arms,
moderate-D / moderate-floor / default-capacity configurations — is
robust to the mechanism choice.

## Why Family B is the right call

- **Spec faithfulness.** `mechanism-design.md` specifies the
  controller in per-block terms ("the controller updates `c_priority`
  per the EIP-1559 rule … per priced block"), matching textbook
  EIP-1559. Family B implements this faithfully; Family A's 2-step
  behavior was an unintentional implementation artifact, not a
  spec-mandated variant. Publishing the spec-matched variant
  forecloses an obvious reviewer objection: "your controller doesn't
  match the spec you describe."
- **Headline claims survive.** The top two mechanism arms
  (un-reserved priority-only, un-reserved both-dynamic) are
  mechanism-robust. The "two-lane outperforms single-lane" headline
  *strengthens* under Family B — RB-reserved and partitioned
  variants jump above single-lane in the welfare ranking.
- **Single-lane was never a load-bearing claim.** The publication's
  primary contribution is the two-lane mechanism family. Single-lane
  EIP-1559 is presented as a baseline and a calibration sweep, not
  as a deployment recommendation. The single-lane welfare collapse
  under Family B is therefore not a publication-blocker; it is a
  more honest characterization of the single-lane mechanism's
  narrower welfare regime.
- **The `x4_rb_quarter` stress corner is publication-useful as a
  robustness limit.** The two cells that flip negative under
  Family B sit at the most aggressive multiplier-floor (4) combined
  with the harshest RB capacity reduction (quarter). Reporting that
  these specific corner-stress configurations fail under
  faithful-EIP-1559 cadence is a credible boundary-of-applicability
  result rather than a hidden weakness.
- **Auditability.** Chain-derivation makes the controller's
  trajectory a pure function of the canonical chain — any third
  party can re-derive it from published block contents and
  controller settings, no node-local accumulator to inspect. This
  is a publication-grade property.

## Publication framing (ready-to-paste)

> **Mechanism (controller cadence).** The dynamic-pricing controller
> is implemented in the EIP-1559 tradition: a deterministic
> per-block update of the per-byte coefficient `c` (or, in two-lane
> variants, the lane-specific coefficients `c_standard` and
> `c_priority`) bounded by `±1/D` per step and floored at the era
> minimum. Following Ethereum's deployed cadence, the controller
> advances exactly once per canonical block. In linear-Leios the
> canonical block type is the Ranking Block; Endorser Block content
> contributes to the controller's window aggregate via its
> certifying RB's sample emission, but does not trigger a separate
> controller step on deferred validation. This per-block stepping
> guarantees a single auditable trajectory: any third party can
> re-derive the controller's full history from the canonical chain
> alone, with no hidden node-local state.

> **Mechanism (chain-derivation).** The controller's state lives on
> the canonical chain: every Ranking Block carries its own
> `derived_quote` field, computed at production as a pure function
> of the parent block's `derived_quote`, the parent's window
> aggregate, and the samples emitted by canonical predecessors
> within the smoothing window. This pattern matches Ethereum's
> EIP-1559 (the controller is stateless at the node level) and
> trivially survives short-range reorgs: a slot-battle-losing block
> is discarded along with its `derived_quote`, leaving no
> contamination on the canonical chain's pricing trajectory. The
> spec-level controller math (EIP-1559 step, multiplier-floor
> invariant, capacity-weighted window aggregate) is unchanged from
> a node-local-accumulator implementation; only the location of the
> state moves from per-node memory to canonical-chain block fields.

> **Why two-lane mechanisms outperform single-lane under the
> corrected semantics.** Under EIP-1559-faithful single-step
> cadence, the single-lane controller is more sensitive to
> short-window demand spikes because the demand signal cannot
> "average out" within an RB-EB pair via a doubled controller step.
> The two-lane priority/standard split provides an additional
> degree of freedom: under the multiplier-floor invariant
> `c_priority ≥ multiplier_floor × c_standard`, priority demand can
> drive `c_priority` upward without dragging `c_standard` with it,
> preserving the standard lane as a stable-fee path for
> non-priority users while still extracting urgency-based welfare
> from the priority lane. This separation is structurally robust to
> the cadence question: the single-lane arm's welfare changes by
> orders of magnitude between cadence variants; the two-lane
> un-reserved arms' welfare changes by ≤ 17% (median |Δ%|).

## Follow-on work

- **Required for publication-grade numbers:** re-run all 19 phase-2
  suites under chain-derived. The 33-job smoke is sufficient for
  the Family B decision and for the welfare-impact
  characterization, but suite-level publication numbers should
  come from the full sweep × 3 seeds. Compute is incremental — the
  suite runner is resumable; only the chain-derived runs need to
  be (re-)generated. Tracked as "Required follow-on" rather than
  blocking this docs cascade.
- **Recommended:** property-based regression test ensuring future
  implementations match the EIP-1559-faithful semantics. One unit
  test was added during the bug-1 fix; a broader property test
  asserting "controller steps exactly N times over a canonical
  chain of length N" (or equivalently, "deferred-EB validation
  fires zero controller steps") would lock in the Family B
  commitment against future regressions.
- **Optional:** spike 003 follow-up explicitly characterizing the
  1-step vs 2-step welfare difference theoretically (not just
  empirically), for any paper reviewer who asks "why does going
  from 2-step to 1-step change rankings?" The current
  characterization is empirical (33-job sundaeswap smoke); a
  theoretical model of the controller dynamics under each cadence
  would harden the publication's account of the mechanism choice.
- **Optional:** explicit cross-architecture determinism CI. The
  chain-derived implementation is bit-stable across architectures
  by construction (u128 rationals + `libm` math), but the wider
  simulator inherits f64 from `main` in non-pricing paths. CR-1's
  `f64::sqrt` resolution would close this last gap.

## Audit trail

References to every artifact involved in arriving at this decision:

- [`.planning/REVIEW.md`](REVIEW.md) — WR-1 row, status RESOLVED 2026-05-14
- [`.planning/spikes/007-chain-derived-controller/README.md`](spikes/007-chain-derived-controller/README.md) — ADOPT verdict, design spec
- [`.planning/chain-derived-controller-PLAN.md`](chain-derived-controller-PLAN.md) — implementation deltas
- [`.planning/chain-derived-bug-investigation.md`](chain-derived-bug-investigation.md) — bug #1 (`current_chain_tip_quote` one-step lag), fixed
- [`.planning/chain-derived-fix-revalidation-2026-05-14.md`](chain-derived-fix-revalidation-2026-05-14.md) — bug #1 fix revalidation
- [`.planning/chain-derived-bug2-investigation.md`](chain-derived-bug2-investigation.md) — bug #2 root cause: mechanism cadence difference, not a bug
- [`.planning/mechanism-welfare-impact-2026-05-14.md`](mechanism-welfare-impact-2026-05-14.md) — 33-job welfare-impact characterization
- [`.planning/spikes/MANIFEST.md`](spikes/MANIFEST.md) — spike 005 RESOLVED, spike 007 ADOPT with empirical revalidation note
- [`docs/phase-2/mechanism-design.md`](../docs/phase-2/mechanism-design.md) — chain-derived controller section, cadence note, simulator-approximations table row
- [`docs/phase-2/cardano-realism-audit.md`](../docs/phase-2/cardano-realism-audit.md) — pricing-controller-calibration correction note
- [`docs/phase-2/validity-threats.md`](../docs/phase-2/validity-threats.md) — Resolved 2026-05-14 block + Family B decision subsection + per-suite reclassifications
- [`.planning/spikes/003-pricing-controller-calibration/README.md`](spikes/003-pricing-controller-calibration/README.md) — correction note (audited the spec, not the implementation)
