# Spike Manifest

## Idea

Audit Cardano-realism of phase-2 dynamic-pricing simulator calibration choices
against current Cardano mainnet parameters and observed behavior. Surface
deviations between the simulation's modeling assumptions and production
Cardano, classify each by plausible impact on phase-2 conclusions, and
recommend whether each deviation is defensible, requires disclosure, or
should drive recalibration.

Source of calibration choices: [CLAUDE.md "Calibration choices"
section](../../CLAUDE.md) and `sim-rs/parameters/phase-2-sweep/`.
Phase-2 spec: [`docs/phase-2/mechanism-design.md`](../../docs/phase-2/mechanism-design.md).

## Requirements

- Each spike produces a `README.md` with Research, Comparison Table (sim ↔
  mainnet ↔ delta ↔ impact), Findings, and Verdict.
- Final synthesis lives at `docs/phase-2/cardano-realism-audit.md` and links
  back to each spike for evidence.
- No code changes from this audit — recalibration is a separate phase if
  warranted.
- All mainnet parameter values cited must include a date stamp and source
  (cexplorer / on-chain query / CIP / protocol-params endpoint) so the audit
  remains auditable as Cardano parameters evolve.

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | rb-cadence-and-capacity | standard | Cardano slot-1s + activeSlotsCoeff=0.05 cadence and current rbBodyMaxSize match phase-2 protocol-base.yaml knobs | VALIDATED | cadence, capacity, protocol-params |
| 002 | fee-structure-and-mempool-sizing | standard | minFeeA/minFeeB and mempool sizing on mainnet vs phase-2 fee formula and mempool cap | NEEDS-DISCLOSURE | fees, mempool, protocol-params |
| 003 | pricing-controller-calibration | standard | EIP-1559 calibration evidence vs phase-2 window-length, multiplier_floor, update cadence | NEEDS-DISCLOSURE | controller, calibration, eip1559 |
| 004 | topology-and-actor-model | standard | Mainnet SPO count + stake distribution vs phase-2 single-producer topology and actor model | NEEDS-DISCLOSURE | topology, actors, stake |
| 005 | validity-threats | standard | Per-claim trust matrix mapping each phase-2 experimental claim to supporting + undermining evidence from REVIEW.md and spikes 001–004 | RESOLVED (chain-derived adopted; topology + curve set; WR-1 closed; mechanism Family B committed 2026-05-14) | trust, validity, publication |
| 006 | curve-design | comparison | Evaluate 3-4 stake-distribution curve options against mainnet data, pick one for the 100-node multi-producer topology, with full rationale | RECOMMENDED option-1-stratified-mainnet | topology, stake-distribution, curve-fit |
| 007 | chain-derived-controller | standard | Lock in the chain-derived (EIP-1559-style) controller design pattern as the WR-1 fix; document rationale, edge cases, comparison to accumulator design. Empirical revalidation arc: bug #1 (`current_chain_tip_quote` one-step lag) identified and fixed; bug #2 was actually a mechanism choice (1-step-per-canonical-block vs the prior accumulator's effective 2-step-per-RB-EB pair), characterized via the 33-job welfare-impact analysis and resolved by committing Family B (EIP-1559-faithful) as the publication mechanism on 2026-05-14. | ADOPT | wr-1, controller, eip-1559, chain-derivation, family-b |
