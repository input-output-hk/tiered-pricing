# Spike 003 — Pricing controller calibration
Date: 2026-05-13
Verdict: NEEDS-DISCLOSURE

> **Correction added 2026-05-14:** This spike audited phase-2's
> `mechanism-design.md` spec and concluded "matches EIP-1559's
> per-block update cadence exactly." That verdict was correct **for
> the spec**. However, post-audit empirical investigation (see
> [`.planning/chain-derived-bug2-investigation.md`](../../chain-derived-bug2-investigation.md)
> and
> [`.planning/mechanism-welfare-impact-2026-05-14.md`](../../mechanism-welfare-impact-2026-05-14.md))
> revealed that the **pre-2026-05-14 accumulator implementation**
> effectively stepped twice per RB-EB pair (via separate
> `apply_priced_block` and `apply_eb_priced_block` calls). This was
> unintentional implementation behavior, not what the spec specified.
> The chain-derived refactor (spike 007) brings the implementation
> in line with the spec's per-block-cadence intent. Family B
> (EIP-1559-faithful, 1-step-per-canonical-block) is the committed
> publication mechanism as of 2026-05-14; see
> [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md).
> The original audit's other findings (window=32,
> multiplier_floor variants, signal-source choices, etc.) remain
> valid.

## Spike Question

- **Given** (a) Ethereum's deployed EIP-1559 controller (D=8
  max-change-denominator, target = half-block-gas, per-block updates)
  and the body of Ethereum gas-pricing research and (b) the open
  calibration choices Cardano's Leios CIP / `mechanism-design.md`
  explicitly leaves to implementation,
- **When** phase-2 uses `window-length=32` for capacity-varying
  signals (length 1 for RB-reserved priority), per-priced-block
  update cadence, `multiplier_floor=4` in two of the seven suites
  (`phase-2-rb-scarcity`, `phase-2-urgency-inversion`) versus spec
  default 16, and EIP-1559 parameters from the suite-tuning YAMLs
  under `parameters/phase-2-sweep/pricing/`,
- **Then** the controller's drift behavior under these calibrations
  should reflect defensible economic assumptions consistent with
  Ethereum's evidence base and the `mechanism-design.md` spec — or,
  if not, deviations must be acknowledged.

## Research

### Ethereum EIP-1559 — canonical deployed parameters

From [EIP-1559 specification](https://eips.ethereum.org/EIPS/eip-1559)
(retrieved 2026-05-13), deployed at London hard fork (Aug 5 2021),
unchanged through Dencun and Pectra:

- `BASE_FEE_MAX_CHANGE_DENOMINATOR` = **8** → per-block move bounded
  at `±12.5%` of current base fee.
- `ELASTICITY_MULTIPLIER` = **2** → `target = gas_limit / 2`. Since
  capacity = `2 × target`, the controller's target as a fraction of
  capacity is exactly **`0.5`**.
- `INITIAL_BASE_FEE` = 1,000,000,000 wei (1 gwei) at the London fork.
- **Update cadence: per parent block, no windowing.** The base-fee
  update reads `parent_gas_used` against `parent_gas_target` and
  computes the new fee immediately. There is **no smoothing window,
  EMA, or other multi-block aggregation in the deployed mechanism.**

Mathematically the spec rule reduces to:
- If `parent_gas_used > parent_gas_target`: `base_fee_delta =
  max(1, parent_base_fee × gas_used_delta / parent_gas_target / D)`.
- If below target: symmetric decrease. Note the asymmetric `max(1,
  ...)` floor on the up-step that is *not* present on the down-step
  — a subtle bias not relevant to phase-2's clamp formulation.

### EIP-1559 academic critique — known controller issues

| Concern | Source | Phase-2 relevance |
|---|---|---|
| **Chaotic short-term oscillations** in block-size and base-fee under per-block updates without smoothing. "Short-term behavior is marked by intense, chaotic oscillations in block sizes and slow adjustments during periods of demand bursts." | Reijsbergen, Sridhar, Monnot, Leonardos et al., *"Transaction Fees on a Honeymoon: Ethereum's EIP-1559 One Month Later"* (arXiv:2110.04753, 2021); follow-on *"Dynamics of Ethereum's EIP-1559…"* (DLT'R&P 2025) | **Directly motivates phase-2's smoothing window.** The authors propose AIMD (additive-increase / multiplicative-decrease) with a *variable* learning rate as a fix; phase-2 instead uses a *capacity-weighted aggregate over a fixed-length window* as the smoothing layer. Different architectural answer to the same observed problem. |
| **Theoretical bounded oscillation** under uniform-value bidder model; "under variable demand, the base fee oscillates stably with no drift." | Leonardos, Reijsbergen, Monnot et al., *"Dynamical analysis of the EIP-1559 Ethereum fee market"* (AFT'21) | Bounds the worst case at "stable oscillation, no drift" — useful for phase-2 because it implies the per-block-no-smoothing baseline isn't catastrophically broken, just noisy. |
| **20% minority base-fee manipulation attack** via empty-block-then-half-full strategy; minority producers can lock the controller into a manipulated equilibrium. | Azouvi, Goren, Heimbach, Hicks, *"Base Fee Manipulation in Ethereum's EIP-1559 Transaction Fee Mechanism"* (DISC'23) | **Smoothing window weakens this attack vector** — the attacker must sustain the strategy across the window length, not just the parent block. Phase-2's 32-block window raises the cost of this attack by a factor of ~32 over EIP-1559's effective window of 1. Not the primary motivation cited in `mechanism-design.md`, but a real benefit. |
| Empirically, "concerns raised by Reijsbergen et al. relating to base fee volatility making fee estimation more difficult does not hold in practice." | Liu, Lu, Nayak, Zhang, Zhang, Zhao, *"Empirical Analysis of EIP-1559…"* (CCS'22) | A counter-bound: the theoretical chaotic-oscillation concern has not manifested as a usability disaster on mainnet. **Phase-2's smoothing is therefore a defensive choice against a theoretical risk, not a fix for an observed Ethereum production failure.** |

### Cardano Leios CIP-0164 — fee mechanism scope

Per [CIP-0164](https://cips.cardano.org/cip/CIP-0164) (retrieved
2026-05-13), Leios itself does **not** introduce a dynamic-pricing
controller — it preserves the existing static `minFeeA + minFeeB`
formula and inherits the existing per-block fee-flow incentive
structure. Phase-2 (this branch / `docs/phase-2/mechanism-design.md`)
is a *follow-on* design that *adds* dynamic pricing on top of Leios's
linear-Leios block model. The controller calibration is therefore
entirely a phase-2 question; CIP-0164 provides the linear-Leios block
geometry (RB ~90 KB, EB up to 12 MB) but no calibration evidence.

### Other deployed dynamic-fee mechanisms (for triangulation)

- **Solana priority fees**: no controller. Users post `compute_unit_price`
  bids; the RPC method `getRecentPrioritizationFees` returns recent
  per-account percentile floors. Fees are set by user-side oracle
  lookup against recent slot history, not by an in-protocol PID-style
  controller. **Not directly comparable** to phase-2's EIP-1559-style
  closed-loop controller; the architecture is fundamentally
  different (auction-style local fee markets rather than a network-
  global base fee).
- **Sui reference gas price**: validators vote at *epoch boundaries*
  on a 2/3-stake-weighted percentile reservation price. Reference
  price is fixed within an epoch (~24h). **Update cadence is
  validator-governance-driven, not closed-loop on utilization** —
  again not directly comparable to phase-2.
- **NEAR protocol**: minimum gas price adjusts with NEAR token price
  (denomination-stabilization) plus congestion managed primarily
  through sharding rather than a fee controller. **Not a comparable
  controller.**

**Finding: among deployed dynamic-fee mechanisms, EIP-1559 is the only
one whose controller architecture (closed-loop integer-rational PID
on per-block utilization) matches phase-2's.** Sui and NEAR run on
different paradigms (epoch governance / denomination stabilization);
Solana doesn't run a network-global controller at all. EIP-1559 is
the sole calibration anchor.

### Phase-2 controller parameters as deployed in the simulator

Read from `parameters/phase-2-sweep/pricing/`:

| Pricing YAML | D | target | window | initial quote | multiplier floor |
|---|---|---|---|---|---|
| `eip1559_d8_target0.5_window32` (baseline) | 8 | 1/2 | 32 | 44 | n/a |
| `eip1559_d4_target0.5_window32` (sweep) | 4 | 1/2 | 32 | 44 | n/a |
| `eip1559_d16_target0.5_window32` (sweep) | 16 | 1/2 | 32 | 44 | n/a |
| `eip1559_d8_target0.25_window32` (sweep) | 8 | 1/4 | 32 | 44 | n/a |
| `eip1559_d8_target0.75_window32` (sweep) | 8 | 3/4 | 32 | 44 | n/a |
| `eip1559_d8_target0.5_window16` (smoothing) | 8 | 1/2 | 16 | 44 | n/a |
| `eip1559_d8_target0.5_window64` (smoothing) | 8 | 1/2 | 64 | 44 | n/a |
| `two_lane_priority_only_static_x{4,8,16}` | priority 8 (standard pinned at c=1) | 1/2 | priority 1 (RB-reserved), standard 32 (ignored) | priority 176/352/704; standard 44 | 4 / 8 / 16 |
| `two_lane_priority_only_unreserved_x{4,8,16}` | priority 8 (standard pinned) | 1/2 | priority 32, standard 32 (ignored) | priority 176/352/704 | 4 / 8 / 16 |
| `two_lane_both_dynamic_partitioned_x{4,16}` | both 8 | 1/2 | priority 1, standard 32 | priority 176/704; standard 44 | 4 / 16 |
| `two_lane_both_dynamic_unreserved_x{4,16}` | both 8 | 1/2 | both 32 | priority 176/704; standard 44 | 4 / 16 |

**Baseline values across every YAML match EIP-1559: `D = 8`,
`target = 0.5`.** The sweeps deliberately stress D and target;
the smoothing suite stresses window length.

### Multiplier-floor usage by suite

Read from `parameters/phase-2-sweep/suites/`:

| Suite | multiplier_floor jobs |
|---|---|
| `phase-2-eip1559-robustness` | n/a (single-lane) |
| `phase-2-eip1559-smoothing` | n/a (single-lane) |
| `phase-2-priority-only-rb-reserved` | x4 + x8 + **x16** (spec default) |
| `phase-2-priority-only-unreserved` | x4 + x8 + **x16** (spec default) |
| `phase-2-two-lane-both-dynamic` | x4 + **x16** (each in partitioned + unreserved) |
| `phase-2-rb-scarcity` | **x4 only** (calibration accommodation, all 4 jobs) |
| `phase-2-urgency-inversion` | **x4 only** (calibration accommodation, both jobs) |

Both x4-only suites have explicit READMEs documenting the
calibration choice; the multi-floor suites cover the spec default.
5 of 7 suites include an `x16` (spec-default) variant; 2 of 7 are
exclusively `x4`.

## Comparison Table

| Knob / concept | Phase-2 value | Ethereum EIP-1559 / spec default | Δ | Defensibility |
|---|---|---|---|---|
| `max-change-denominator` D | **8** (baseline + 5 of 7 suites; D∈{4,8,16} sweep in robustness suite) | **8** (BASE_FEE_MAX_CHANGE_DENOMINATOR, London-deployed) | **0 — exact match** | Matches deployed Ethereum. The sweep variants (D=4 and D=16) bracket the deployed value for sensitivity analysis. **Strong defensibility, no disclosure needed.** |
| `target-num / target-den` | **1/2** (baseline + every two-lane suite; 1/4 and 3/4 swept in robustness suite) | **1/2** equivalent (ELASTICITY_MULTIPLIER=2 means `target = gas_limit/2`) | **0 — exact match** | Matches deployed Ethereum. Spec `mechanism-design.md:127` explicitly notes Ethereum's choice of 0.5 makes the clamp non-engaging; phase-2 keeps the clamp unconditional for safety under target ≠ 0.5 sweeps. **Strong defensibility.** |
| Update cadence | **Per priced block** (each RB or EB that fires a sample) | **Per block** (per parent block) | Conceptually identical — per-block closed-loop update | The deviation is in **what counts as a "block"** under linear-Leios's two-block-type ledger (RB ~90KB; EB up to 12MB), not in the cadence concept. Phase-2 fires the controller on every priced block (RB or EB); Ethereum fires on every block (single block type). `mechanism-design.md:105` flags "per priced block / per RB / per epoch" as open. **Matches the closest Ethereum analogue.** |
| Window length (capacity-varying signals) | **32 priced blocks** (single-lane EIP-1559, both-dynamic standard, un-reserved priority) | **1** (EIP-1559 reads parent block only; no window) | **+31** vs Ethereum | **The sole architectural deviation.** Motivated by (a) `mechanism-design.md:97-103` blending heterogeneous block sizes (RB ~90KB vs EB ~12MB ⇒ 133× ratio) which the unwindowed Ethereum approach cannot smooth, and (b) academic critique of EIP-1559 oscillation (Reijsbergen et al., 2021/2025; AIMD proposal). **Defensible: phase-2 picks a *different* smoothing architecture from AIMD but addresses the same problem.** No directly-comparable deployed system uses 32; the smoothing suite (`phase-2-eip1559-smoothing.yaml`) sweeps {16, 32, 64} as a sensitivity probe. **NEEDS-DISCLOSURE** that the choice is theoretical not empirical. |
| Window length (RB-reserved priority controller) | **1** (forced by `TwoLanePricing::new` regardless of YAML) | **1** (EIP-1559 baseline equivalent) | **0 — matches** | The RB-reserved priority signal has uniform per-block capacity (always one RB-worth = 90KB), so windowing buys nothing mathematically — `mechanism-design.md:176` explicitly notes "every priced block offers exactly one RB-worth of priority service capacity, so the per-block utilisation is already normalised to [0, 1] regardless of block type." Phase-2's choice **mathematically reduces to per-block fill rate**, which is exactly EIP-1559's mode. **Strong defensibility.** |
| Multiplier-floor | **4 in 2 of 7 suites; 16 in the spec-default-covering 5 of 7 suites; {4, 8, 16} swept in priority-only suites** | **16** (spec default, `mechanism-design.md:155`/`mechanism-design.md:290`) | x4 deviation in 2 suites | No directly comparable deployed system (EIP-1559 has no second lane and no multiplier floor at all). The spec default of 16 is itself stated without an empirical economic anchor — `mechanism-design.md` does not cite calibration data for 16; it's a round-number choice intended to give a *strong* price-discrimination guarantee. The x4 choice in `phase-2-rb-scarcity` and `phase-2-urgency-inversion` is **a calibration accommodation, not an economic claim**: CLAUDE.md explicitly states "at 16, only urgency≥5 components find priority attractive on the utility-maximising lane choice and priority demand stays too low to surface controller drift; at 4, urgency≥2 picks priority and the controller does drift." **NEEDS-DISCLOSURE: the x4 choice changes the regime studied (urgency≥2 vs urgency≥5 self-selection), and the conclusions of those two suites are conditional on x4. The x16 variants in the other 5 suites cover the spec-default regime.** |
| Both-dynamic standard signal source | **Capacity-weighted aggregate of `standard_paying_bytes` against `eb_referenced_txs_max_size_bytes` for EBs; no standard sample fires on RB-reserved RBs.** | Spec leaves open (`mechanism-design.md:238`: "standard-fee bytes against what denominator?") | n/a — no spec default | The choice has an internally-consistent economic story: in RB-reserved variants, RB capacity is dedicated to priority by validity rule, so RB traffic must not move standard pricing (or else priority demand mechanically inflates the standard quote, defeating the partition). The omission of standard samples on RB-reserved RBs is the *only way* to preserve lane isolation under both-dynamic. **Defensible architectural choice; no comparable deployed system; NEEDS-DISCLOSURE only that no empirical evidence backs the specific normalization (`eb_referenced_txs_max_size_bytes`) over alternatives.** |
| Un-reserved priority signal source | **Option 1: `priority_paying_bytes / total_block_capacity`** (`mechanism-design.md:209`) | Spec leaves open with three candidates: 1 (priority-bytes-share), 2 (notional priority share), 3 (delay-gap signal). | n/a — no deployed analogue | No deployed dynamic-fee system has a comparable un-reserved priority lane. Option 1 is **the natural and simplest of the three** — it requires no additional parameter (option 2 needs a notional share knob) and no separate signal path (option 3 needs delay-EMA infrastructure). Option 1 has the known weakness that **priority demand which fits comfortably below the lane's capacity reads as low utilization even when priority service is materially demanded**, biasing the controller toward keeping `c_priority` near the multiplier floor unless priority demand is genuinely large. **NEEDS-DISCLOSURE: option 1 is a defensible choice without empirical evidence; phase-2 conclusions about un-reserved-priority-only welfare are conditional on this signal definition.** |
| Initial quote = 44 lovelace/byte | 44 (matches mainnet `minFeeA`) | n/a (Ethereum's INITIAL_BASE_FEE = 1 gwei; different unit, different economic anchor) | Phase-2 anchors at mainnet equilibrium | Per spike 002 finding: at controller equilibrium under baseline demand, phase-2 fees reproduce mainnet exactly. **Already addressed in spike 002, not re-litigated here.** |

## Findings

- **Phase-2's two core controller parameters (D=8, target=0.5)
  match deployed EIP-1559 exactly** in every baseline pricing YAML.
  Sweeps in `phase-2-eip1559-robustness.yaml` (D∈{4,8,16}, target∈
  {0.25, 0.5, 0.75}) bracket the Ethereum-deployed values for
  sensitivity analysis. This is the strongest equivalence anchor:
  any deployed-system reader can map phase-2's controller to
  EIP-1559 by name and the numbers line up. **No disclosure needed
  on D or target.**

- **The 32-block smoothing window is the single architectural
  deviation from EIP-1559 and is the most defensible deviation in
  the calibration set.** EIP-1559 deployed without smoothing,
  academic critique (Reijsbergen, Leonardos et al., 2021/2025)
  identifies short-term chaotic oscillation as a real (if not yet
  empirically harmful — Liu et al. CCS'22) problem. Phase-2's
  capacity-weighted-window response addresses the same problem
  with a different architecture from AIMD. **The motivation is
  also mechanically forced by linear-Leios: heterogeneous block
  sizes (90KB RB vs 12MB EB, 133× ratio) cannot be smoothed by
  EIP-1559's parent-block-only update — `mechanism-design.md:97-103`
  walks the alternatives and rules each out.** The smoothing suite's
  {16, 32, 64} sweep gives the sensitivity. **NEEDS-DISCLOSURE
  only** in the sense that the specific length 32 is a defensible
  round number, not an empirically-calibrated value.

- **Update cadence per priced block matches EIP-1559's per-parent-
  block cadence in concept.** The only re-interpretation is what
  counts as "a block" under linear-Leios's two-block-type ledger
  — and phase-2 fires on every RB and every EB, the closest direct
  analogue. **No disclosure needed.**

- **Multiplier-floor: the x4 deviation in `phase-2-rb-scarcity`
  and `phase-2-urgency-inversion` is a calibration accommodation
  documented as such, and the x16 spec default is covered by the
  other 5 suites.** Across the 7-suite design, the multiplier
  floor is explicitly studied at {4, 8, 16} in the priority-only
  suites and at {4, 16} in both-dynamic. The two x4-only suites
  exist because the experiments they run (RB-scarcity stress,
  urgency-inversion stress) depend on priority demand actually
  drifting the controller, and at x16 priority demand is too low
  to drift. **The conclusions of those two suites are conditional
  on x4 and the suite READMEs say so.** No suite makes an x4-vs-
  x16 economic claim — the priority-only suites simply sweep all
  three. **NEEDS-DISCLOSURE**: any write-up that summarises
  `phase-2-rb-scarcity` or `phase-2-urgency-inversion` conclusions
  must lead with "under multiplier_floor=4" rather than reporting
  the conclusion absolutely.

- **The spec default of 16 is itself not anchored to empirical
  data.** `mechanism-design.md:290` declares it the default in the
  Calibration-vs-Invariant table without a citation; the only
  justification in the document is that the floor "gives a price-
  discrimination guarantee" (line 155). Both 4 and 16 are
  round-number choices in the absence of deployed-system evidence
  — Ethereum has no second lane and no multiplier floor at all,
  so there is no Ethereum-analogue to anchor against. **The
  multiplier-floor magnitude is therefore the single weakest-
  anchored calibration in phase-2.** This is a spec-level issue,
  not a simulator-level issue; the simulator faithfully implements
  the spec's open-question framing.

- **Both-dynamic standard signal source (`standard_paying_bytes`
  vs `eb_referenced_txs_max_size_bytes`, no standard samples on
  RB-reserved RBs) has an internally-consistent economic story
  but no empirical anchor.** The lane-isolation argument
  ("priority demand on RBs must not move standard pricing") is
  defensible from the partition rule's logic — if RB capacity is
  priority-only by validity, then RB traffic *cannot* signal
  standard congestion without leaking the partition. But the
  specific choice of `eb_referenced_txs_max_size_bytes` as the
  denominator over alternatives (e.g. notional standard share, or
  capacity minus reserved priority partition) is a phase-2-only
  choice without a comparable deployed analogue. **NEEDS-
  DISCLOSURE** in any external write-up of both-dynamic suite
  results.

- **Un-reserved priority signal source is option 1 of three the
  spec leaves open.** Option 1 (`priority_bytes / total_capacity`)
  is the simplest and the only one that requires no extra
  calibration knob, but it has a known weakness: priority demand
  that fits comfortably below partition-worth still reads as low
  utilization, biasing `c_priority` toward the floor. The
  `phase-2-priority-only-unreserved` suite sweeps the multiplier
  floor {4, 8, 16} which partially compensates by varying how
  much room the priority quote has to drift above the floor, but
  the **signal-source choice itself is uncovered by sensitivity
  analysis**. **NEEDS-DISCLOSURE**: un-reserved-priority-only
  welfare conclusions are conditional on option 1.

- **No CIP-0164 / Leios paper material constrains the controller
  calibration.** CIP-0164 (the Leios CIP) preserves the static
  `minFeeA + minFeeB` formula and does not introduce dynamic
  pricing — phase-2 is a follow-on design layered on top of
  Leios's linear-Leios block geometry. The controller calibration
  choices are entirely phase-2's and have no Leios-side anchor to
  defer to.

## Investigation Trail

- The EIP-1559 deployed parameters (D=8, ELASTICITY_MULTIPLIER=2,
  per-parent-block update, no window) were re-verified by direct
  WebFetch of `https://eips.ethereum.org/EIPS/eip-1559`. The spec
  text confirms the update reads only `parent_gas_used` /
  `parent_gas_target`, with no aggregation. Cross-checked against
  Consensys EIP-1559 documentation and the ethereum.github.io
  ABM1559 notebook.

- Academic critique of EIP-1559's per-block-no-smoothing approach
  was traced through:
  - Reijsbergen, Sridhar, Monnot, Leonardos, *"Transaction Fees
    on a Honeymoon: Ethereum's EIP-1559 One Month Later"*
    (arXiv:2110.04753, IEEE ICBC 2022) — quotes the "intense,
    chaotic oscillations in block sizes and slow adjustments
    during periods of demand bursts" finding and proposes AIMD
    as the fix.
  - Leonardos, Reijsbergen et al., *"Dynamical analysis of the
    EIP-1559 Ethereum fee market"* (AFT'21) — gives the theoretical
    "bounded oscillation under uniform-value bidders, stable
    oscillation under variable demand" result.
  - Liu et al., *"Empirical Analysis of EIP-1559: Transaction
    Fees, Waiting Times, and Consensus Security"* (CCS'22) — the
    empirical counter-bound showing the theoretical oscillation
    has not manifested as a usability problem on mainnet.
  - Azouvi, Goren, Heimbach, Hicks, *"Base Fee Manipulation in
    Ethereum's EIP-1559 Transaction Fee Mechanism"* (DISC'23) —
    the 20%-minority base-fee manipulation attack, weakened by
    window smoothing.

  Phase-2's response (capacity-weighted window length 32) is
  architecturally distinct from AIMD (the literature's preferred
  fix) but addresses the same observed problem. The choice of 32
  vs 16 vs 64 is empirically swept in the smoothing suite.

- The triangulation against Solana / Sui / NEAR confirmed that
  **EIP-1559 is the only deployed system whose controller
  architecture is directly comparable to phase-2's**. Solana uses
  user-side bidding with RPC oracle lookup (no controller); Sui
  uses epoch-boundary validator voting on a percentile reservation
  price (no closed-loop); NEAR adjusts minimum gas with token
  price + sharding for congestion (different paradigm).

- The multiplier-floor calibration choice was cross-referenced
  with `mechanism-design.md:155` ("default 16, configurable"),
  `mechanism-design.md:290` (Calibration-vs-Invariant table
  listing default 16), CLAUDE.md "Calibration choices" section,
  and the two x4-only suite READMEs (`phase-2-rb-scarcity.README.md`
  and `phase-2-urgency-inversion.README.md`). The spec does not
  cite empirical economic evidence for 16; the choice appears to
  be a round-number "guarantee strong price discrimination"
  decision. The x4 choice in two suites is documented as a
  calibration accommodation, not an economic claim.

- The CIP-0164 / Leios scope check was performed via WebSearch
  on `cips.cardano.org/cip/CIP-0164` and the cardano-scaling/CIPs
  repo at `CIP-0164/README.md`. Confirmed: Leios preserves the
  static fee structure and adds *throughput*, not dynamic pricing.
  Phase-2 is a follow-on design.

- The phase-2 pricing-YAML inventory was read directly from
  `sim-rs/parameters/phase-2-sweep/pricing/` (all 18 files, only
  representative ones quoted in the table to keep the read scope
  manageable). The suite multiplier-floor mapping was read from
  the 7 phase-2 suite YAMLs under `sim-rs/parameters/phase-2-sweep/suites/`.

- The integer-rational EIP-1559 update math was confirmed by
  direct reading of
  `sim-rs/sim-core/src/tx_pricing/single_lane.rs` `Eip1559Pricing::step`
  (lines 215-299). The implementation works on `quote_per_byte` as
  `u64` directly, with u128 intermediates; the spec clamp formula
  `±1/D` is applied symmetrically; ceiling rounding is used per
  `implementation-plan.md:175`. **No f64 enters the controller
  hot path.** This is consistent with phase-2's cross-arch
  determinism contract — the controller math is bit-stable across
  architectures given identical input streams.

## Verdict

**NEEDS-DISCLOSURE.** Three of the seven calibration questions
have clean defensibility (D=8, target=0.5, per-priced-block
cadence — all match deployed EIP-1559 exactly); one (RB-reserved
priority window length 1) is mathematically forced by the
mechanism and matches EIP-1559's effective cadence. The remaining
four calibration choices each need explicit disclosure in any
external write-up:

1. **Window length 32 for capacity-varying signals** is a
   defensible response to a real-but-empirically-mild problem
   identified in EIP-1559 academic critique, but the specific
   length (32) is a round-number choice without empirical
   anchoring beyond the smoothing-suite sweep. Phase-2's
   architectural answer (capacity-weighted window) is *different
   from* the literature's preferred answer (AIMD) — worth
   mentioning when discussing prior art.

2. **Multiplier-floor x4 in `phase-2-rb-scarcity` and
   `phase-2-urgency-inversion`** is a calibration accommodation,
   not an economic claim. The conclusions of these two suites
   should be reported "under multiplier_floor=4" rather than
   absolutely. The x16 spec default is independently covered by
   the other 5 suites (priority-only at x4/x8/x16, both-dynamic
   at x4/x16) — so the 7-suite design as a whole is robust
   across the floor sweep.

3. **The spec default of 16 itself has no empirical anchor.**
   Ethereum has no comparable second-lane multiplier; the choice
   of 16 in `mechanism-design.md:155, 290` is a round-number
   "strong price discrimination guarantee" decision. This is a
   spec-level disclosure rather than a simulator-level one, but
   should be flagged: phase-2 is operating without a comparable
   deployed-system anchor for the multiplier-floor magnitude.

4. **Both-dynamic standard signal source and un-reserved priority
   signal source (option 1)** are spec-level open questions that
   the simulator picks concrete answers for. Both choices are
   defensibly motivated (lane isolation; simplest of three
   options) but neither has an empirical anchor. Suite
   conclusions are conditional on these signal-source choices,
   and any external write-up should disclose them.

None of these invalidate phase-2's controller architecture; they
constitute the "calibration-side asterisks" that should appear
alongside the spike 002 fee-semantics asterisks in any write-up.

## Sources

- [EIP-1559: Fee market change for ETH 1.0 chain](https://eips.ethereum.org/EIPS/eip-1559) — retrieved 2026-05-13 (BASE_FEE_MAX_CHANGE_DENOMINATOR=8, ELASTICITY_MULTIPLIER=2, per-parent-block update, no window)
- [Reijsbergen, Sridhar, Monnot, Leonardos et al., "Transaction Fees on a Honeymoon: Ethereum's EIP-1559 One Month Later"](https://arxiv.org/abs/2110.04753) — retrieved 2026-05-13 (chaotic short-term oscillations finding; AIMD proposal)
- [Leonardos, Reijsbergen et al., "Dynamical analysis of the EIP-1559 Ethereum fee market"](https://dl.acm.org/doi/10.1145/3479722.3480993) — retrieved 2026-05-13 (bounded oscillation theoretical result)
- [Liu, Lu, Nayak, Zhang, Zhang, Zhao, "Empirical Analysis of EIP-1559: Transaction Fees, Waiting Times, and Consensus Security"](https://arxiv.org/pdf/2201.05574) — retrieved 2026-05-13 (empirical counter-bound: theoretical oscillation has not manifested as usability problem)
- [Azouvi, Goren, Heimbach, Hicks, "Base Fee Manipulation in Ethereum's EIP-1559 Transaction Fee Mechanism"](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.DISC.2023.6) — retrieved 2026-05-13 (20%-minority manipulation attack; window smoothing as mitigation)
- [Roughgarden, "Transaction Fee Mechanism Design for the Ethereum Blockchain: An Economic Analysis of EIP-1559"](https://timroughgarden.org/papers/eip1559.pdf) — retrieved 2026-05-13 (original economic-analysis paper; MMIC / OCA-proofness / DSIC except under demand spike)
- [Roughgarden, "Transaction Fee Mechanism Design" (EC'21)](https://timroughgarden.org/papers/eip1559exchanges.pdf) — retrieved 2026-05-13 (extended conference version)
- [CIP-0164 — Ouroboros Linear Leios](https://cips.cardano.org/cip/CIP-0164) — retrieved 2026-05-13 (Leios scope: no dynamic-pricing controller; preserves static minFeeA + minFeeB)
- [Sui Documentation — Gas in Sui (Reference Gas Price)](https://docs.sui.io/concepts/tokenomics/gas-in-sui) — retrieved 2026-05-13 (epoch-boundary 2/3-stake percentile vote; not a closed-loop controller)
- [Figment — Deep Dive: Sui Reference Gas Price](https://www.figment.io/insights/deep-dive-sui-reference-gas-price/) — retrieved 2026-05-13 (gas-price-survey mechanics)
- [Solana Docs — Fees](https://solana.com/docs/core/fees) — retrieved 2026-05-13 (priority fees as user-side bids; no controller)
- [Helius — Priority Fees: Understanding Solana's Transaction Fee Mechanics](https://www.helius.dev/blog/priority-fees-understanding-solanas-transaction-fee-mechanics) — retrieved 2026-05-13 (getRecentPrioritizationFees mechanics; local fee markets)
- [NEAR Docs — Gas Advanced](https://docs.near.org/concepts/basics/transactions/gas-advanced) — retrieved 2026-05-13 (NEAR gas pricing model: min gas adjusted with token price; congestion via sharding)
- [Consensys — What is EIP-1559?](https://consensys.io/blog/what-is-eip-1559-how-will-it-change-ethereum) — retrieved 2026-05-13 (deployed parameter summary)
- In-repo provenance: `sim-rs/sim-core/src/tx_pricing/single_lane.rs` (`Eip1559Pricing::step`, lines 215-299 — clamp formula, era floor, u128 rational math); `sim-rs/sim-core/src/tx_pricing/two_lane.rs` (`TwoLanePricing::new`, `enforce_multiplier_floor`, `samples_for_block` — four-variant emission); `sim-rs/sim-core/src/tx_pricing/window.rs` (`CapacityWeightedWindow` — u128 rolling aggregate); `sim-rs/parameters/phase-2-sweep/pricing/` (all 18 pricing YAMLs); `sim-rs/parameters/phase-2-sweep/suites/` (all 7 phase-2 suite YAMLs + the 2 x4-only suite READMEs); `docs/phase-2/mechanism-design.md` lines 97-105 (capacity-weighted-window motivation), 119-133 (single-lane EIP-1559 update rule), 155 (multiplier-floor default 16), 168-180 (RB-reserved priority signal), 207-211 (un-reserved priority three-option open question), 238 (both-dynamic standard signal open question), 290 (multiplier-floor in Calibration-vs-Invariant table); `CLAUDE.md` "Calibration choices" section (window length 32 rationale, multiplier_floor=4 motivation, both-dynamic standard signal source, un-reserved priority option 1).
