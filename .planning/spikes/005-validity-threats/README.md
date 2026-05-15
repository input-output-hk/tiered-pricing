# Spike 005 — Validity threats (per-claim trust matrix)
Date: 2026-05-13 (opened) / 2026-05-14 (RESOLVED)
Verdict: **RESOLVED** — chain-derived adopted; topology + curve set;
WR-1 closed; mechanism Family B (EIP-1559-faithful) committed
2026-05-14. See [`MANIFEST.md`](../MANIFEST.md) spike-005 row for the
authoritative verdict line.

> This README is a pointer. The full per-claim trust matrix and the
> structured discussion of each of the 19 phase-2 suite YAMLs lives
> at [`docs/phase-2/validity-threats.md`](../../../docs/phase-2/validity-threats.md).
> That document is the canonical content; this file exists so spike
> 005 has a discoverable entry point under `.planning/spikes/`
> matching the other spikes' (001, 002, 003, 004, 006, 007) layout.

## Spike Question

- **Given** that phase-2 publishes welfare conclusions from 19 suite
  YAMLs under `parameters/phase-2-sweep/suites/`, each driven by a
  combination of mechanism choice, demand profile, topology, and
  controller calibration,
- **When** a paper reviewer or external auditor asks "which of these
  conclusions are robust, which depend on calibration choices the
  spec leaves open, and which would change under a different
  reasonable parameterisation?",
- **Then** the simulator owes a per-claim trust matrix mapping each
  suite to its supporting evidence (spike 001–004 findings,
  CONCERNS.md fragilities, mechanism-design.md spec text) and its
  undermining evidence (REVIEW.md findings, known calibration
  fragilities, methodology gaps).

## Verdict (summary — see MANIFEST.md row for the full line)

**RESOLVED 2026-05-14.** The three trust gaps that motivated this
spike at opening have all closed:

1. **Topology gap** — closed 2026-05-13. Suites now run on
   `topology-realistic-100.yaml` (multi-producer, 100-node,
   mass-stratified mainnet curve via spike 006); CLAUDE.md, the
   realism audit, and the suite YAMLs all agree.
2. **Stake-curve design** — closed 2026-05-13 via spike 006's
   `RECOMMENDED option-1-stratified-mainnet` verdict.
3. **WR-1 (controller contamination on slot-battle reorg)** —
   closed 2026-05-14 via the chain-derived controller refactor
   (spike 007 ADOPT). The empirical re-validation arc produced a
   secondary finding (the pre-refactor accumulator effectively
   stepped twice per RB-EB pair); that finding was characterised
   in `.planning/mechanism-welfare-impact-2026-05-14.md` and
   resolved by committing Family B (EIP-1559-faithful, one
   controller step per canonical block) as the publication
   mechanism. See
   [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md).

The per-suite trust ratings in `docs/phase-2/validity-threats.md`
have been updated through both passes (2026-05-13 topology
resolution, 2026-05-14 Family B commitment).

## Audit trail

The discoverability chain that arrived at this spike's resolution:

1. **2026-05-13**: validity-threats.md first draft surfaced two
   open trust gaps — topology mismatch (`topology.default.yaml` on
   disk vs `topology-single-producer.yaml` in docs) and an unsettled
   stake-curve choice for the 100-node multi-producer pivot.
2. **2026-05-13**: spike 006 (curve design) opened to resolve the
   stake-curve gap → `RECOMMENDED option-1-stratified-mainnet` →
   `topology-realistic-100.yaml` adopted across all phase-2 suites.
3. **2026-05-13**: with multi-producer topology now live, WR-1
   (pricing-state contamination on slot-battle reorg, REVIEW.md)
   reclassified from *dormant* to *LIVE / disclosure-required* —
   the realistic-100 smoke comparison observed 41 slot battles
   across 33 (job, seed) pairs.
4. **2026-05-14**: spike 007 (chain-derived controller) opened to
   resolve WR-1 by design rather than by snapshot/restore →
   `ADOPT` verdict → chain-derived controller implemented per
   `.planning/chain-derived-controller-PLAN.md`.
5. **2026-05-14**: post-refactor empirical revalidation identified
   bug #1 (one-step `current_chain_tip_quote` lag), fixed; and
   characterised a mechanism-cadence difference between the
   pre-refactor accumulator (effective 2-step per RB-EB pair) and
   the chain-derived design (1 step per canonical block, EIP-1559
   faithful). The mechanism choice was committed to Family B in
   [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md).
6. **2026-05-14**: WR-1 marked **RESOLVED** in `.planning/REVIEW.md`;
   this spike's verdict updated from `NEEDS-RESOLUTION` to
   **RESOLVED** to reflect all three opening trust gaps now closed.

## Pointers

- Canonical content: [`docs/phase-2/validity-threats.md`](../../../docs/phase-2/validity-threats.md)
- Authoritative mechanism decision: [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md)
- Empirical welfare-impact analysis: [`.planning/mechanism-welfare-impact-2026-05-14.md`](../../mechanism-welfare-impact-2026-05-14.md)
- WR-1 row (RESOLVED): [`.planning/REVIEW.md`](../../REVIEW.md#fix-status)
- Sibling spikes: [001](../001-rb-cadence-and-capacity/README.md),
  [002](../002-fee-structure-and-mempool-sizing/README.md),
  [003](../003-pricing-controller-calibration/README.md),
  [004](../004-topology-and-actor-model/README.md),
  [006](../006-curve-design/README.md),
  [007](../007-chain-derived-controller/README.md)
