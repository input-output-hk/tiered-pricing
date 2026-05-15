# Phase-2 CIP Evidence Audit

## What This Is

A confidence-base milestone on top of the phase-2 dynamic-pricing simulator (the `dynamic-experiment` branch). The deliverables are a **realism-risks register** with cheap-test resolutions and a **coverage check** mapping every menu-item trade-off claim to specific jobs in goldens-pinned suites — enough evidence base for the user to author the CIP responding to [`docs/phase-2/CPS-0023`](../docs/phase-2/CPS-0023) ("Urgency Signaling") with epistemic justification.

The user authors the CIP itself; this milestone produces the evidence the CIP cites.

## Core Value

A reader of the CIP should be able to verify each claim about each menu option against a specific simulator job, and should be able to inspect the realism-risks register to see what the simulator does and does not faithfully model — so the CIP stands on documented evidence rather than asserted authority.

## Requirements

### Validated

<!-- Capabilities present in the codebase as of 2026-05-15, inferred from .planning/codebase/ and CLAUDE.md -->

- ✓ Five mechanism variants implemented: single-lane EIP-1559 (control), priority-only-RB-reserved, priority-only-un-reserved, both-dynamic-partitioned, both-dynamic-un-partitioned — phase-2 M1–M3
- ✓ Chain-derived (Family B) controller, EIP-1559-faithful per-block cadence, committed for publication — 2026-05-14
- ✓ Seven goldens-pinned suites characterising the variant matrix (M3/M4) — phase-2 M5
- ✓ 100-node mainnet-derived topology (`topology-realistic-100.yaml`, epoch-582 stake snapshot, mass-stratified) — phase-2 M5
- ✓ Intra-architectural determinism via `pricing_event_stream.sha256` (3-layer regime: unit goldens, `verify` subcommand, suite-level goldens) — phase-2 M5
- ✓ Cardano-calibration audit covering 6 categories ([`docs/phase-2/cardano-realism-audit.md`](../docs/phase-2/cardano-realism-audit.md)) — phase-2 audit spikes 001–006
- ✓ Family B welfare impact characterised across 33 sundaeswap-smoke jobs ([`.planning/mechanism-welfare-impact-2026-05-14.md`](mechanism-welfare-impact-2026-05-14.md)) — 2026-05-14
- ✓ Parallel suite runner (`experiment-suite run --parallelism N`)
- ✓ WR-1 (controller contamination on slot-battle reorg) resolved by construction under Family B — see [`.planning/REVIEW.md`](REVIEW.md) Fix Status

### Active

<!-- Hypotheses to validate this milestone. Each is a requirement to deliver evidence for the CIP. -->

- [ ] **Realism-risks register** — `.planning/realism-risks-register.md` (or equivalent), listing every realism risk with category, impact-if-real, current evidence, verdict (LIVE / DORMANT / MITIGATED / DISCLOSED), and disclosure framing. Pattern: register-plus-cheap-targeted-test per LIVE risk.
- [ ] **Pool-number sensitivity test (prototype-pattern test)** — 33-job smoke × {100 pools, 150 pools} × {sundaeswap_moderate + all 4 `paper_like_*` variants}. Verdict threshold: Δ% on welfare metrics tight enough to mark CIP-0164 600-pool migration as DISCLOSED-NOT-LIMITATION.
- [ ] **Multi-seed variance bands** — re-run canonical menu-item job at ≥ N seeds (N to be calibrated by run cost); produce variance bands; verify single-seed claims in `family-b-results-table-2026-05-14.md` and `family-b-full-sweep-analysis-2026-05-14.md` still hold.
- [ ] **Coverage check** — mapping table: each menu trade-off claim → backing job(s) in goldens-pinned suites. Surfaces gaps where a CIP claim has no specific simulator job behind it.
- [ ] **Anchoring or disclosure for 4 unanchored controller knobs** — window-length 32, the two multiplier-floor choices (4 and 16), lane-signal-source choices for both-dynamic and un-reserved-priority. Either anchor against deployed-system data, or write the CIP disclosure paragraph.
- [ ] **Refreshed cardano-realism-audit.md** — its 2026-05-13 annotation already acknowledges the `topology-realistic-100.yaml` reality; finish the rewrite so the document reads as authoritative rather than annotated.
- [ ] **Run-length / steady-state validation** — verify 2000 slots suffices for steady-state behaviour under each menu item; if not, raise default for affected jobs.
- [ ] **Additional targeted cheap tests** — one per other LIVE risk surfaced by the register (3–5 tests anticipated; concrete list emerges from the register itself).

### Out of Scope

- **Writing the CIP itself** — user authors the CIP from this evidence base.
- **Adversarial / strategic-bidder modelling** — current actor model is utility-maximising; adversarial regime named as a known gap and disclosed in the CIP as future work. Adding this would double milestone length without proportionate CIP value.
- **Re-auditing upstream simulator code paths** (`sim/lottery.rs`, `sim/driver.rs`, propagation, vote diffusion, the f64 in non-pricing hot paths) — inherited limitations are disclosed; the standard Leios simulator is the substrate the CIP builds on, and re-auditing it is out-of-scope for this milestone.
- **Cross-architecture CI verification** — already deferred in `.planning/codebase/CONCERNS.md`; intra-arch determinism is sufficient for CIP evidence; disclosure paragraph covers the scope.
- **600-pool CIP-0164 topology migration** ([`docs/phase-2/m6-implementation-plan.md`](../docs/phase-2/m6-implementation-plan.md) as drafted) — superseded by the pool-number sensitivity test. If 100 ≈ 150 on welfare metrics, `topology-realistic-100.yaml` is sufficient for CIP-grade evidence. Plan stays in tree as a contingency in case the sensitivity test surfaces a real gap.
- **Cutting the EIP-1559 suites** — retained as control evidence; the CIP references them as baseline ("here's what a single-lane deployment would look like for comparison").

## Context

The `dynamic-experiment` branch is a clean-room rebuild on top of the upstream Leios protocol simulator implementing the phase-2 dynamic-pricing mechanisms specified in [`docs/phase-2/mechanism-design.md`](../docs/phase-2/mechanism-design.md). M1–M5 are complete; Family B was committed for publication on 2026-05-14 (see [`.planning/family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md)).

The codebase is large (~21k lines including upstream) and the phase-2 rebuild added ~6k lines covering: the pricing kernel (`sim-core/src/tx_pricing/`), block-production additions in `linear_leios.rs`, the `MempoolGate`, the actor model (`tx_actors.rs`), the suite runner, and the metrics collectors. The user's stated worry is "code changes are large, so it's hard to feel super confident that what we've got is truly representative" — which is the explicit driver for the realism-risks audit shape.

The change-surface (new code) has been audited via the calibration realism audit and the 7 audit spikes; the inherited-from-upstream substrate has not been re-audited and is treated as disclosed-limitation rather than re-audit target.

Prior exploration: 7 numbered spikes under [`.planning/spikes/`](spikes/) (RB cadence, fee structure, controller calibration, topology, validity threats, curve design, chain-derived controller). The chain-derived spike (007) directly drove the Family B commitment. Wrap-up of these spikes into a project-local `spike-findings-*` skill is open ([`.planning/spikes/MANIFEST.md`](spikes/MANIFEST.md)).

Recent audit-trail memos worth keeping in view: [`family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md), [`family-b-full-sweep-analysis-2026-05-14.md`](family-b-full-sweep-analysis-2026-05-14.md), [`mechanism-welfare-impact-2026-05-14.md`](mechanism-welfare-impact-2026-05-14.md), [`family-b-results-table-2026-05-14.md`](family-b-results-table-2026-05-14.md), [`REVIEW.md`](REVIEW.md) (Fix Status table specifically), [`docs/phase-2/validity-threats.md`](../docs/phase-2/validity-threats.md) (spike 005 per-claim trust matrix).

## Constraints

- **Tech stack**: Rust workspace under `sim-rs/` (`sim-core`, `sim-cli`); YAML configuration under `sim-rs/parameters/phase-2-sweep/`. No new languages or runtimes; minimise new dependencies. New code conforms to the CLAUDE.md numeric-representation contract (integer/rational/u128 in simulation-affecting state; f64 only in reporting outputs).
- **Determinism**: All new jobs added to goldens-pinned suites must pass `experiment-suite verify` and pin `pricing_event_stream.sha256` constants. `UPDATE_GOLDENS=1` regenerations require explicit user authorisation per the CLAUDE.md workflow.
- **No CIP authorship**: Milestone outputs feed the user's CIP draft; the user writes the CIP themselves.
- **No upstream re-audit**: Inherited Leios simulator code (lottery, propagation, voting, distribution sampling) is out-of-scope substrate; phase-2 work treats it as a black-box dependency with disclosed limitations.
- **No new mechanism work**: Family B is committed for publication; no new mechanism families, new controller-variant arms, or new lane vocabulary in this milestone.
- **No deadline**: Milestone takes the time it takes; quality of evidence over speed.
- **Compute budget**: Targeted-cheap-test pattern means new experiments are scoped to be runnable within practical session times. Multi-arch / multi-thousand-seed sweeps are off the table.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| CIP shape is single-document menu of mechanism options, with CIP author = user (not Claude) | Lets the user maintain editorial authority over the CIP narrative while delegating evidence-gathering | — Pending |
| Realism-risks audit = register + targeted cheap tests (not just a register, not full re-audit) | The pool-number-test pattern proved cheap-tests resolve doubts; register-only would leave gaps; full re-audit is too expensive | — Pending |
| Pool-number sensitivity test is the prototype-pattern for the realism-risks audit | Concrete first instance establishes the audit method | — Pending |
| Pool-number smoke runs across `sundaeswap_moderate` + all 4 `paper_like_*` variants, at {100, 150 pools} | Dual-load coverage ensures the result is not load-specific | — Pending |
| EIP-1559 demoted from menu to control-only, per research request | Research-stakeholder ask; menu is the 4 two-lane variants; EIP-1559 retained as baseline evidence | — Pending |
| Adversarial / strategic-bidder modelling deferred and disclosed | Doubles milestone length; CIP discloses as future work | — Pending |
| Upstream simulator re-audit out of scope, disclosed | Phase-2 work treats Leios simulator substrate as disclosed-limitation, not re-audit target | — Pending |
| 600-pool CIP-0164 migration ([`m6-implementation-plan.md`](../docs/phase-2/m6-implementation-plan.md)) superseded by the cheaper sensitivity test | If 100 ≈ 150, realistic-100 suffices; M6 plan stays in tree as contingency | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-15 after initialization*
