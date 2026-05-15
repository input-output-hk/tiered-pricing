# Domain Pitfalls

**Domain:** CIP-grade evidence on top of a mature mechanism-design simulator (phase-2 dynamic-pricing for Cardano Leios; deliverable feeds the user's CIP responding to CPS-0023)
**Researched:** 2026-05-15
**Confidence:** HIGH (precedents drawn from EIP-1559 empirical literature, software-engineering threats-to-validity scholarship, the phase-2 audit trail, and Cardano CIP-1 process).

## Reading guide

This register is scoped to *evidence-package* pitfalls — not simulator-implementation pitfalls (those have been navigated; see `docs/phase-2/validity-threats.md` and `.planning/REVIEW.md`). Each pitfall is keyed against (a) which phase-2 artefact is at risk and (b) which milestone phase of the CIP-evidence audit should resolve it. The five categories map to the user's question:

- **Coverage gaps** — what reviewers find embarrassing (defaults-only claims, single-load demand, missing variance, stale parameters)
- **Realism-disclosure failures** — inherited limitations not disclosed; calibration anchored to spec defaults rather than data
- **Reproducibility pitfalls** — golden-hash fragility, seed-sensitivity hidden behind small N, scope-creep on "deterministic"
- **CIP-process pitfalls** — menu structure collapsing into advocacy; evidence not aligned with menu items
- **Methodological audit pitfalls** — auditing what's auditable rather than what matters; cheap-tests that don't resolve doubt

The structure is `CRITICAL → MODERATE → MINOR`; criticality reflects publication-grade reviewer-objection risk, not implementation effort.

## Critical Pitfalls

Mistakes that, if shipped, force a CIP revision cycle or invalidate a headline trade-off claim.

### CRIT-1: Single-seed welfare claims at publication-grade precision

**Category:** Coverage gaps / Reproducibility
**At risk:** `family-b-results-table-2026-05-14.md` (per-job net-utility numbers); `mechanism-welfare-impact-2026-05-14.md` (33-job sundaeswap smoke run at seed=1 only); the user's CIP menu-comparison narrative if it cites those tables.

**What goes wrong:** Welfare deltas reported as point numbers (e.g. "RB-reserved priority-only gains ~30% median welfare under Family B") collapse the stochastic envelope into a single sample. Three of the four sign-flip cells in the family-B-vs-accumulator comparison (`d4_t50_w32`, `d8_t25_w32`, `x4_rb_quarter`) sit close to zero in absolute welfare — a seed-2 or seed-3 run could plausibly produce the opposite sign. A reviewer who asks "what's the 95% CI on that flip?" gets no answer.

**Why it happens:** Single-seed runs are the cheap default during development; promoting them to publication evidence without re-running at higher N is the most common bridge-too-far pattern in simulator-derived publications. The 19 phase-2 suites *do* use 3 seeds, but the 33-job sundaeswap smoke (which drove the Family B commitment) used seed=1 only.

**Precedent:** Reijsbergen et al.'s EIP-1559 dynamical analysis (AFT'21) reported instability regimes with formal Lyapunov bounds — but Liu et al.'s CCS'22 empirical follow-up showed those instabilities were not catastrophic in deployment. The lesson: simulator-derived sign-of-effect claims need either (a) confidence intervals over enough seeds, or (b) bounded-region analytic guarantees — neither is currently in hand for the four flip cells.

**Prevention:** Treat any welfare claim with absolute net-utility under, say, 10× the per-seed cell's IQR as conditional and re-run at ≥ N seeds. The Active requirement `Multi-seed variance bands` in `.planning/PROJECT.md` is precisely this; the cheap-test pattern of the realism-risks audit applies — calibrate N by run cost, but pick the N that makes the variance band non-degenerate, not the N that fits the calendar. **Concretely**: re-run the 4 sign-flip cells (`d4_t50_w32`, `d8_t25_w32`, `x4_rb_quarter` under both rb-reserved-priority-only and partitioned-both-dynamic arms) at ≥ 10 seeds before any CIP claim cites them.

**Detection:** Read every comparative-welfare claim in `family-b-*-2026-05-14.md` and ask "would this claim invert if a single seed went the other way?" If yes, that cell needs more seeds.

**Resolution phase:** Multi-seed variance bands (PROJECT.md Active item 3) — earliest phase in the audit milestone after the realism-risks register is drafted.

### CRIT-2: Menu CIP collapsing into "this is best, others suck"

**Category:** CIP-process
**At risk:** The CIP draft itself; the framing of `mechanism-welfare-impact-2026-05-14.md`'s per-arm summary; the un-reserved-priority-only vs un-reserved-both-dynamic vs RB-reserved vs partitioned comparison.

**What goes wrong:** A CIP framed as "menu of options" but whose evidence package shows one option dominating on every reported metric becomes de-facto advocacy. The published `family-b-results-table-2026-05-14.md` Table 2 already shows un-reserved-both-dynamic dominating on net-utility (med +1.70e+10 vs partitioned's +3.02e+09) and inclusion rate. If the CIP's menu-item descriptions don't surface the *non-welfare* trade-offs (anti-bribery property, standard-user fee-drift exposure, on-chain-validated priority commitment), the menu reads as a single-option recommendation in disguise.

**Why it happens:** Welfare is the most legible metric in the metrics collector; non-welfare properties (anti-bribery, audit/dispute behavior, signal-source spec-openness, complexity to implement and reason about) are harder to quantify and easier to omit. The `validity-threats.md` audit already flags this: RB-reserved's anti-bribery property is "only formally true under the honest-producer assumption" and un-reserved variants have "anti-bribery property absent — disclose explicitly."

**Precedent:** EIP-1559's deployment narrative is the canonical good example here — the published analyses (Liu et al., Roughgarden's "Transaction Fee Mechanism Design" foundational work) explicitly distinguish welfare-style claims from incentive-compatibility / side-contract-proofness properties because the impossibility result (Chung & Shi, SODA'23) makes that distinction matter. A CIP that compresses this two-axis space into one welfare number reproduces a known authorial mistake.

**Prevention:** The coverage-check deliverable (PROJECT.md Active item 4) must include a column for *each non-welfare property* of each menu item, not just the simulator-job backing. Concretely add to the menu × claim matrix: anti-bribery (formal / informal / absent), signal-source-anchoring (deployed-data / spec-open / unanchored), standard-user-fee-drift-exposure (none / bounded-pending-output / unbounded), implementation complexity (chain-derived state required / per-block validity rule / both). Each non-welfare cell must cite either a spec section, a simulator measurement, or an explicit "this property is not exercised by the current evidence base — disclosed gap."

**Detection:** Take the draft coverage check. Strip the welfare numbers. Does the remaining table support a 4-way menu, or does it collapse to "use un-reserved-both-dynamic"?

**Resolution phase:** Coverage check (PROJECT.md Active item 4); must precede CIP draft handoff.

### CRIT-3: Reviewer-anticipated-question gaps — "why this controller cadence?"

**Category:** CIP-process / Realism-disclosure
**At risk:** The Family B commitment narrative; the per-priced-block cadence calibration; the publication framing in `family-b-decision-2026-05-14.md` §"Publication framing".

**What goes wrong:** A reviewer asks "why did you choose per-canonical-block controller cadence rather than per-epoch or per-2-blocks?" and the answer in the CIP is "EIP-1559-faithful" — which is a *bullet-point answer*, not a *resolution of doubt*. The deeper answer is in the 2026-05-14 chain-derived-bug2-investigation; it's specifically that the prior accumulator was effectively double-stepping per RB-EB pair, and Family B corrects this to match the spec. But the CIP itself, lifted from the published evidence, may not transmit that chain of reasoning intelligibly. The cadence is one of the four un-anchored controller knobs called out in PROJECT.md Active item 5; the others (window-length 32, the two multiplier floors, lane-signal-source choices) sit in the same trap.

**Why it happens:** "Inherited from EIP-1559" is the fastest defence to write and the weakest defence to publish; it inherits Ethereum's calibration choices on faith, even where the linear-Leios setting (90 KB RBs blended with 12 MB EBs, 133× capacity ratio) makes the inheritance not-obviously-correct. The cardano-realism-audit already flags window-length 32 as "a round-number choice, not an empirical anchor" — that's the precondition for the trap.

**Precedent:** CIP-1's review criteria explicitly list "design choices have relevant justifications or rationale" as a check ([cips.cardano.org/cip/CIP-1](https://cips.cardano.org/cip/CIP-1)). EIPs frequently get bounced for cite-without-rationale on parameter choices — `bytecode versioning` and `EIP-4844 blob count` discussions in the EthMagicians forums are public-record examples.

**Prevention:** For each of the four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source choices), produce a CIP-disclosure paragraph that either (a) anchors against deployed-system data, or (b) explicitly says "we chose X; an alternative Y exists; we did not exercise Y in this evidence base because [reason]; the qualitative finding is conditional on X." The PROJECT.md formulation "Either anchor against deployed-system data, or write the CIP disclosure paragraph" is correct; the trap is *forgetting any of the four*.

**Detection:** Grep the draft CIP for "[default]", "[standard]", "[as in EIP-1559]" — each is a candidate cite-without-rationale.

**Resolution phase:** Anchoring or disclosure for the 4 unanchored controller knobs (PROJECT.md Active item 5).

### CRIT-4: Inherited-substrate limitations not disclosed (Leios simulator)

**Category:** Realism-disclosure
**At risk:** Any CIP claim phrased as "the simulator faithfully models Cardano Leios"; the cardano-realism-audit's TL;DR; the universal evidence-base disclaimer.

**What goes wrong:** The CIP cites simulator-derived welfare numbers, but the simulator inherits f64 in non-pricing code paths (slot lottery, propagation, vote diffusion, distribution sampling — see `validity-threats.md` "CR-1 (`f64::sqrt` in `endorsement_window_priced_blocks`) puts a small but nonzero asterisk on cross-arch reproducibility"). The CIP claims reproducibility without scope-qualifying to "intra-arch on x86_64/glibc". A reviewer reproducing on ARM gets different numbers and writes an objection.

**Why it happens:** Inherited limitations are easy to forget because they pre-exist the phase-2 work. The pricing kernel was hardened to integer/rational; the substrate was not. The validity-threats audit already documents this — but a CIP reader doesn't read `.planning/`.

**Precedent:** EIP-1559's own deployment surprises (Reijsbergen's chaotic-oscillation regime, Liu et al.'s mainnet observation that those regimes don't fire in practice) are precisely the inherited-substrate-limitation pattern: the *mechanism* was well-modelled; the *demand and miner behavior substrate* was simplified-then-deployed and produced unexpected dynamics. The lesson: state the substrate scope before stating the mechanism conclusion.

**Prevention:** Write a single "Substrate scope" paragraph that lists every inherited-limitation category and goes into the CIP verbatim. The cardano-realism-audit and validity-threats Standard footer already supply ~70% of this — but the CIP version must be one paragraph, not a 70-page audit, and must explicitly cover: (a) f64 in non-pricing hot paths and the cross-arch caveat; (b) propagation model fidelity (the simulator's RTT-driven topology vs mainnet's reality); (c) actor model utility-maximising assumption vs strategic-bidder reality (deferred per PROJECT.md out-of-scope item 2); (d) refund-CIP dependency.

**Detection:** Hand the draft CIP to someone who hasn't read `.planning/`. Ask: "what does this simulator faithfully model, and what does it not?" If the answer requires reading the audit, the disclosure paragraph is insufficient.

**Resolution phase:** Refreshed cardano-realism-audit.md (PROJECT.md Active item 6) — must produce both the audit and the CIP-pasteable substrate-scope paragraph.

### CRIT-5: Calibration-stale parameters cited as current

**Category:** Realism-disclosure / Coverage
**At risk:** The `topology-realistic-100.yaml` epoch-582 stake snapshot; the SundaeSwap-Jan-2022 demand profile (now 4 years stale); the Q1 2026 paper-like-realistic demand mix calibration.

**What goes wrong:** The CIP cites "mainnet-snapshot stake distribution" and "Cardano Q1 2026 realistic demand mix" as anchors, but the snapshot is from epoch 582 (retrieved 2026-05-14 per CLAUDE.md). If publication runs across a 6-month review cycle, the snapshot is 1–2 epochs stale by submission and 4–5 epochs stale by reviewer-deep-read. The demand profile drift is worse: `paper_like_realistic` was calibrated against Q1 2026 traffic but the simulator does not auto-refresh.

**Why it happens:** Calibration drift is silent — nothing in the codebase tells you the snapshot is old. The cardano-realism-audit dates each anchor but does not flag freshness-decay.

**Precedent:** EIP-1559's pre-launch simulations were calibrated against 2020-Q4 mainnet demand; by London-fork (August 2021) the demand mix had shifted enough that the simulation results were already being re-reported with caveats. The lesson: anchor calibration *dates*, not just *anchor sources*.

**Prevention:** Every calibrated parameter in the CIP must have a `(value, source, date-retrieved)` triple. Add a freshness-policy disclosure: "Calibration anchors retrieved 2026-05-XX; parameter drift over the publication horizon does not affect the qualitative findings because [bound argument]; magnitudes may shift by up to [N]% under epoch drift." Either argue the bound or disclose it. The pool-number sensitivity test (PROJECT.md Active item 2) is a prototype of this: if 100 ≈ 150, *and* the bound also holds across the snapshot-epoch range, the disclosure becomes credible.

**Detection:** Grep `.planning/` and `parameters/` for any date string older than 6 months at publication time. Each is a candidate stale-anchor.

**Resolution phase:** Realism-risks register (PROJECT.md Active item 1); pool-number sensitivity test (Active item 2) as the prototype-pattern; refreshed cardano-realism-audit (Active item 6) for the final disclosure pass.

## Moderate Pitfalls

### MOD-1: Defaults-only parameter coverage

**Category:** Coverage gaps
**At risk:** The 7 goldens-pinned suites; specifically `phase-2-rb-scarcity.yaml` and `phase-2-urgency-inversion.yaml` which use `multiplier_floor = 4` exclusively (validity-threats LOW verdict for both).

**What goes wrong:** The CIP claims X about the RB-scarcity gradient or the urgency-inversion failure mode, but the evidence is from `multiplier_floor = 4` only — a non-default value that *itself* the audit flags as calibration accommodation, not economic claim. A reviewer who pulls up the suite YAML sees no `multiplier_floor = 16` companion run and asks "does this replicate at the spec default?" The honest answer ("priority demand stays too thin to drift at x16, which is itself the finding") is not in the suite output today.

**Why it happens:** The calibration-fix postmortem (`calibration-fix-postmortem.md`) corrected a bigger framing bug, but the `multiplier_floor = 4` choice survived. CLAUDE.md flags it explicitly — but suite outputs don't.

**Prevention:** For both LOW suites, either (a) add a `multiplier_floor = 16` companion job and report whichever way it lands (replication or "priority too thin to drift" boundary), or (b) recast the CIP claim as exploratory at floor=4 only. The validity-threats recommendation §3 is exactly this; tracking it as a moderate pitfall keeps it from being deferred indefinitely.

**Resolution phase:** Additional targeted cheap tests (PROJECT.md Active item 8) — one cheap floor=16 companion run per LOW suite.

### MOD-2: Steady-state assumption at 2000 slots

**Category:** Coverage gaps / Methodological
**At risk:** All 19 phase-2 suites; specifically the un-reserved-both-dynamic standard-lane drift claim (validity-threats UNRESOLVED for moderate / realistic / sundaeswap variants).

**What goes wrong:** The CIP claims X about long-run behavior, but every suite runs 2000 slots (~10 min simulated time). For controllers that mix RB samples (rb-prob = 0.05, ~one sample per 20 slots) with EB-borne samples, 2000 slots gives ~100 controller updates — enough for transient response but not obviously enough for asymptotic behavior. The audit flagged this as "verify 2000 slots suffices for steady-state behaviour under each menu item" (PROJECT.md Active item 7); the trap is shipping without verifying.

**Why it happens:** Long runs are expensive; the temptation to assume steady-state from 2000-slot output is high.

**Prevention:** Run one canonical job per arm at 2× and 4× run length; compare the last-N-slots welfare-rate to the full-run welfare-rate. If they agree within seed-IQR, 2000 is sufficient. If not, raise the default for affected jobs and re-pin goldens. The cheap-test pattern applies — one canonical job per arm is sufficient to resolve doubt for all suites in that arm.

**Resolution phase:** Run-length / steady-state validation (PROJECT.md Active item 7).

### MOD-3: Adversarial-case stress not exercised

**Category:** Coverage gaps
**At risk:** Any CIP claim phrased as "the mechanism is robust"; CPS-0023's "permissionless access" goal; the un-reserved variants' anti-bribery story (already disclosed as "absent" but the *failure mode* under strategic bidding is not in the evidence).

**What goes wrong:** PROJECT.md is explicit that strategic-bidder modelling is out of scope and disclosed as future work. The trap is *forgetting to disclose it prominently* in the CIP — if the CIP reads as "robust mechanism" without leading with the actor-model caveat, a reviewer who knows the transaction-fee-mechanism-design literature (Chung & Shi, Roughgarden) will spot the gap in 30 seconds.

**Why it happens:** Adversarial modelling is the kind of work that's easy to defer and hard to remember to surface. CPS-0023 also implicitly invites this question via its "Open Questions" §"Is there an MEV implication here?".

**Precedent:** Roughgarden's foundational TFM work ([eprint 2021/1474](https://eprint.iacr.org/2021/1474)) and Chung & Shi's impossibility result (SODA'23) define the strategic-bidder regime exactly. Any CIP-grade work that does not cite these and lead its caveat list with "the evidence base uses a utility-maximising actor model; strategic-bidder behavior including bribery, side-contracts, and MEV strategies is not exercised" leaves an obvious objection vector.

**Prevention:** Make the actor-model caveat one of the top-three disclosure items in the CIP. Frame as: "We exercise the menu against a utility-maximising 3- or 11-component actor model with empirical demand calibration. Strategic-bidder regimes (bribery, side-contracts, MEV-aware ordering, sustained gaming of the controller) are out of scope for this evidence base and are disclosed as future work; the CPS's open question on MEV implication is acknowledged." Cite Chung & Shi for the formal impossibility frame.

**Resolution phase:** Realism-risks register (PROJECT.md Active item 1) — actor-model scope is a register entry, not a test.

### MOD-4: Hash-diversity sanity check skipped

**Category:** Reproducibility / Methodological
**At risk:** Any claim that the 3-seed evidence is genuinely diverse rather than artifactually-correlated (the `hash-div` column in `family-b-results-table-2026-05-14.md` is currently a sanity check, not a publication gate).

**What goes wrong:** Three seeds run, all three produce nearly-identical `pricing_event_stream.sha256` (hash-div ≈ 1 across the cell). The 3-seed median is reported but the underlying samples are correlated by some structural artifact (e.g., the same node winning the RB lottery in slot 0 across seeds for some reason, propagating through the rest of the run). A reviewer who looks at the hash-diversity column sees the issue; one who doesn't, doesn't. The current results table reports hash-div ≈ N seeds for most cells (good), but the few cells with hash-div < N (which exist) are not flagged in the narrative.

**Why it happens:** Hash-diversity is a quality-of-evidence signal that's easy to add as a column and easy to omit from prose summaries.

**Prevention:** Make hash-diversity a publication gate: any cell with `hash-div < N_seeds` flagged inline ("seed correlation observed in this cell; results condition on the seed-correlation event") OR re-run with different seed values until hash-div equals N_seeds. The cost is one column in the CIP's evidence table; the benefit is closing a clean methodological vector.

**Resolution phase:** Multi-seed variance bands (PROJECT.md Active item 3) — hash-diversity check is a one-line addition to the variance-bands deliverable.

### MOD-5: Cheap-tests that don't actually resolve the doubt

**Category:** Methodological
**At risk:** The realism-risks register pattern itself; the prototype pool-number sensitivity test (PROJECT.md Active item 2).

**What goes wrong:** A LIVE risk is logged, a cheap test runs, the test produces a number that looks favourable, and the register marks the risk MITIGATED. But the test was scoped narrowly enough that it doesn't actually exercise the failure mode the risk anticipated. Example: pool-number test compares 100 vs 150 pools and finds Δ% < 5% on welfare. Register marks the risk MITIGATED. But the failure mode of concern is at 600 pools (CIP-0164), where slot-battle frequency scales super-linearly — the 100→150 comparison doesn't bracket that.

**Why it happens:** Cheap tests are scoped to be cheap; the scoping decision is often made before the failure mode is fully articulated.

**Precedent:** This is the canonical "looking under the streetlight" methodological pattern in empirical software engineering ([Threats to validity in controlled experiments, Wohlin et al., reviewed in HackerNoon](https://hackernoon.com/assessing-validity-threats-in-controlled-software-engineering-experiments)). The fix is to write the failure-mode hypothesis *before* the test and have someone else check that the test actually rules it out.

**Prevention:** Every cheap test in the register must declare its failure-mode hypothesis upfront: "We test X across the range Y; we expect Z; if observed, the risk is MITIGATED *within range Y*; the risk remains LIVE outside range Y." The pool-number test specifically should make this explicit: "100→150 sensitivity bounds the realistic-100 calibration against same-order-of-magnitude pool counts; the 600-pool migration risk remains a separate LIVE-or-DISCLOSED item." The realism-risks register format (PROJECT.md Active item 1) should include a `scope-of-resolution` field per test.

**Resolution phase:** Realism-risks register design (PROJECT.md Active item 1) — bake the scope-of-resolution field into the template, then apply uniformly to every entry.

### MOD-6: Determinism claim scope-creep

**Category:** Reproducibility
**At risk:** Any CIP statement of the form "results are deterministic" or "any reviewer can reproduce"; the suite-level goldens narrative.

**What goes wrong:** CLAUDE.md and validity-threats.md are explicit that determinism is *intra-architecture* — the goldens reproduce bit-identically on x86_64/glibc but cross-arch is not proven and CR-1 (`f64::sqrt`) is the residual asterisk. The CIP draft says "reproducible by any reviewer" because that's the obvious CIP-friendly sentence; a reviewer on an ARM build (increasingly common in 2026) gets different bits and writes an objection. PROJECT.md Out-of-Scope item 4 confirms cross-arch CI is deferred — the trap is not disclosing it.

**Why it happens:** "Deterministic" is a comforting word that papers over scope. The fix is one adjective.

**Prevention:** Every appearance of "deterministic" or "reproducible" in the CIP must be qualified as "intra-architecture deterministic (x86_64/glibc as the reference build environment)" or "reproducible on the reference toolchain". Add to the CIP's reproducibility paragraph: "Cross-architecture reproducibility was deferred per the project's scope decision and is contingent on replacing the residual f64::sqrt site (`endorsement_window_priced_blocks`) with `libm::sqrt`; see CR-1 in `.planning/REVIEW.md` for the disposition."

**Resolution phase:** Refreshed cardano-realism-audit.md (PROJECT.md Active item 6) — both the audit's wording and the CIP-pasteable footer carry this caveat consistently.

### MOD-7: Evidence package not aligned with menu structure

**Category:** CIP-process
**At risk:** The coverage check (PROJECT.md Active item 4); the menu-item ↔ simulator-job mapping.

**What goes wrong:** The CIP draft describes a 4-item menu (un-reserved-priority-only, un-reserved-both-dynamic, RB-reserved-priority-only, partitioned-both-dynamic), but the evidence base has 19 suites organized by demand profile (congested / moderate / realistic / sundaeswap) × mechanism arm (singlelane / priority-only / both-dynamic) — not by menu item. Mapping menu items to suites requires some translation; if the translation is implicit, a reviewer auditing the evidence has to do it themselves and may misalign. The result is a CIP whose evidence appears to back claim X but a reviewer's reconstruction backs claim X' (a subtly different claim).

**Why it happens:** The suite organisation evolved during phase-2 (M3 → M4 → M5 progression) and was not refactored when the menu structure was settled. The 7 M3/M4 mechanism-characterisation suites + 12 demand-regime suites have their own internal logic; the menu has different internal logic; the join is implicit.

**Prevention:** The coverage check (PROJECT.md Active item 4) must produce an *explicit* menu-item × claim × suite × job table. Every row = one (menu item, claim about that item, suite + job-name backing the claim). No implicit mapping. If a menu item has a claim with no backing job, that's a coverage gap and must be flagged as either "add a job" or "weaken the claim".

**Resolution phase:** Coverage check (PROJECT.md Active item 4) — produced before CIP draft handoff.

## Minor Pitfalls

### MIN-1: Slot-battle disclosure inconsistency

**Category:** Realism-disclosure
**At risk:** The disclosure framing of WR-1 across audit / CIP / validity-threats — currently the three sources tell three slightly different versions of the same RESOLVED-2026-05-14 fact.

**What goes wrong:** Validity-threats and REVIEW.md both record WR-1 as RESOLVED by chain-derivation; the cardano-realism-audit has a 2026-05-13 annotation acknowledging the topology shift but doesn't carry the 2026-05-14 WR-1 resolution forward in the same place. A reviewer reading the audit alone gets stale information.

**Prevention:** Audit refresh (PROJECT.md Active item 6) is the right place to reconcile.

**Resolution phase:** Refreshed cardano-realism-audit.md.

### MIN-2: Welfare-as-f64 reporting boundary not surfaced

**Category:** Realism-disclosure
**At risk:** Any welfare-magnitude claim; the numeric representation contract that's prominent in CLAUDE.md but absent from the CIP.

**What goes wrong:** CLAUDE.md is explicit that reporting outputs (`retained_value`, `net_utility`, `retained_value_ratio`) are f64 — they never feed back into simulation but they are subject to floating-point precision artefacts. A reviewer who sees a welfare delta of, say, +1.234e+10 vs +1.235e+10 may treat them as meaningfully different when they're inside f64 reporting noise.

**Prevention:** Add to the CIP's evidence-base footer: "Reporting outputs (welfare aggregates) are computed in f64 and are subject to ~15-digit floating-point precision. Reported magnitudes should be interpreted to ≤ 3 significant figures; comparisons within that precision are not meaningfully different."

**Resolution phase:** Refreshed cardano-realism-audit.md (PROJECT.md Active item 6) — the audit's existing numeric-representation paragraph just needs a one-line "and therefore the CIP-grade magnitude resolution is ~3 sig fig" addendum.

### MIN-3: SundaeSwap demand-profile origin caveat under-stated

**Category:** Coverage / Realism-disclosure
**At risk:** Any claim from the sundaeswap suites; the four sundaeswap-* variants of the demand-regime suites.

**What goes wrong:** The SundaeSwap-Jan-2022 launch is the *single most empirically-anchored demand source* in phase-2 (per validity-threats); the CIP is likely to lean on it for empirical credibility. But the event is now 4 years old, was a retail-frenzy spike rather than a representative steady-state, and conditioning the CIP narrative on it implicitly claims the spike shape is recurring or recurrent — which is at best unproven.

**Prevention:** Frame sundaeswap claims explicitly as "spike-event robustness" rather than "general behavior under realistic demand". The validity-threats §"phase-2-sundaeswap-singlelane" trust rating already does this; carry the same framing into the CIP.

**Resolution phase:** Coverage check (PROJECT.md Active item 4) — the menu × claim table is the right place to enforce per-claim demand-scope.

### MIN-4: Manifest serde-casing drift not user-visible (but reviewer-visible if they download artefacts)

**Category:** Reproducibility (minor)
**At risk:** Reviewer-side reproduction; the `output/` directory contents that the user may publish alongside the CIP.

**What goes wrong:** CLAUDE.md flags that YAML configs use kebab-case while `RunSummary` uses snake_case, and the inconsistency is historical-accident-frozen. A reviewer attempting to write a quick script against the published artefacts hits a mixed-casing surprise. Not a publication-blocker; a reproduction-friction point.

**Prevention:** If the CIP publishes a sample manifest, ship a one-paragraph schema note that explains the kebab-vs-snake split and points at the CLAUDE.md Conventions section.

**Resolution phase:** Out of scope of this audit milestone; surface as a CIP-side documentation note rather than a code change.

## Phase-Specific Warnings

Maps each PITFALLS entry to the milestone phase (PROJECT.md Active items) that should resolve it. This is the actionability summary — a roadmap-planner can use it as a checklist.

| Phase / PROJECT.md Active item | Pitfalls to resolve in this phase |
|---|---|
| Active item 1: Realism-risks register | CRIT-5 (calibration-stale parameters), MOD-3 (adversarial-case scope statement), MOD-5 (cheap-test scope-of-resolution field) |
| Active item 2: Pool-number sensitivity test (prototype) | Validates MOD-5 pattern; bounded scope for CRIT-5 |
| Active item 3: Multi-seed variance bands | CRIT-1 (single-seed claims), MOD-4 (hash-diversity sanity check) |
| Active item 4: Coverage check | CRIT-2 (menu collapsing to advocacy), MOD-7 (evidence-package menu-alignment), MIN-3 (sundaeswap demand framing) |
| Active item 5: Anchoring or disclosure for 4 unanchored knobs | CRIT-3 (reviewer-anticipated-question gaps for cadence/window-length/floors/signal-source) |
| Active item 6: Refreshed cardano-realism-audit.md | CRIT-4 (inherited-substrate disclosure), MOD-6 (determinism scope-creep), MIN-1 (WR-1 disclosure consistency), MIN-2 (welfare-as-f64 boundary) |
| Active item 7: Run-length / steady-state validation | MOD-2 (steady-state at 2000 slots) |
| Active item 8: Additional targeted cheap tests | MOD-1 (defaults-only at multiplier_floor=4 for LOW suites); any further LIVE risks surfaced by Active item 1 |

## Cross-cutting prevention principles

Distilled from the precedents above; useful as planning-time discipline.

1. **State scope before claim.** Every CIP claim must lead with its scope (which menu item, which demand profile, which floor, how many seeds, intra- or cross-arch). The pattern that fails reviewers is `[claim]; [caveat]` — the pattern that survives is `under [scope]: [claim]`.
2. **Cite by source-and-date.** Every calibration anchor: `(value, source, date-retrieved)`. Mainnet stake distribution is a moving target; "epoch 582, retrieved 2026-05-14" beats "mainnet-derived" every time.
3. **Surface non-welfare properties explicitly.** The menu has 4 items; welfare is one axis; anti-bribery, signal-source-anchoring, standard-user-drift-exposure, implementation-complexity are 4 more. Suppressing them turns a menu into advocacy.
4. **Test what worries you, not what runs fastest.** The cheap-test pattern is sound, but cheap-tests must declare their failure-mode hypothesis upfront and the test must actually rule it out.
5. **Make hash-diversity a publication gate.** Three seeds with identical event-stream hashes are not three seeds.
6. **Inherit limitations explicitly.** The Leios simulator's f64 in non-pricing paths, the propagation model fidelity, the utility-maximising actor model — each is an inherited limitation that must be in the CIP's substrate-scope paragraph, not just in the audit.
7. **Anchor or disclose, never assume.** The four un-anchored controller knobs are the canonical test case: for each, the CIP either anchors against deployed-system data or carries the explicit "conditional on X" disclosure paragraph. There's no middle path.

## Sources

### Phase-2 evidence base (authoritative, internal)

- `/home/will/git/arc-tiered-pricing/CLAUDE.md` — numeric-representation contract, determinism scope, calibration choices (sections "Numeric representation contract", "Determinism scope", "Calibration choices")
- `/home/will/git/arc-tiered-pricing/docs/phase-2/validity-threats.md` — per-claim trust ratings for all 19 suites, the 4 cells that flip welfare sign under Family B, the LOW/UNRESOLVED disposition
- `/home/will/git/arc-tiered-pricing/docs/phase-2/cardano-realism-audit.md` — 12 disclosure items across 4 categories; the un-anchored controller knobs
- `/home/will/git/arc-tiered-pricing/.planning/REVIEW.md` — Fix Status table; WR-1 resolution 2026-05-14; CR-1 cross-arch caveat
- `/home/will/git/arc-tiered-pricing/.planning/family-b-decision-2026-05-14.md` — Family B publication commitment; per-arm welfare-impact summary
- `/home/will/git/arc-tiered-pricing/.planning/mechanism-welfare-impact-2026-05-14.md` — 33-job sundaeswap smoke at seed=1 (single-seed flip-cell evidence)
- `/home/will/git/arc-tiered-pricing/.planning/family-b-results-table-2026-05-14.md` — 19 suites × 3 seeds = 468 (job, seed) pairs; per-arm aggregates; hash-diversity column
- `/home/will/git/arc-tiered-pricing/docs/phase-2/CPS-0023/README.md` — the urgency-signaling CPS this evidence package responds to
- `/home/will/git/arc-tiered-pricing/.planning/PROJECT.md` — Active requirements list (8 items mapped above)

### External precedents

- [Empirical Analysis of EIP-1559 (Liu et al., CCS'22)](https://dl.acm.org/doi/10.1145/3548606.3559341) — deployment retrospective showing fee-estimation improvement, waiting-time reduction, modest fee-level effect; demonstrates how empirical follow-ups distinguish welfare-style claims from theoretical-instability claims
- [Dynamical Analysis of the EIP-1559 Ethereum Fee Market (Reijsbergen et al., AFT'21)](https://arxiv.org/abs/2102.10567) — chaotic-oscillation regime; Lyapunov bounds on step-size for global convergence; precedent for the window-length-32 disclosure framing
- [Transaction Fees on a Honeymoon: Ethereum's EIP-1559 One Month Later (Liu et al., 2021)](https://arxiv.org/abs/2110.04753) — calibration drift between pre-launch and post-launch demand mix; precedent for CRIT-5 (calibration-stale parameters)
- [Foundations of Transaction Fee Mechanism Design (Chung & Shi, SODA'23)](https://eprint.iacr.org/2021/1474) — impossibility result for user-IC + miner-IC + side-contract-proofness; precedent for MOD-3 (adversarial-case disclosure) and CRIT-2 (non-welfare-property surfacing)
- [Transaction Fee Mechanism Design (Roughgarden, foundational)](https://arxiv.org/html/2106.01340v3) — defines the strategic-bidder regime; canonical citation for the actor-model caveat
- [CIP-1 (Cardano Improvement Proposals Process)](https://cips.cardano.org/cip/CIP-1) — review criteria include "design choices have relevant justifications or rationale"; precedent for CRIT-3 (reviewer-anticipated-questions)
- [Threats to Validity in Controlled Software Engineering Experiments](https://hackernoon.com/assessing-validity-threats-in-controlled-software-engineering-experiments) — the four-validity framework (construct / internal / external / conclusion); precedent for MOD-5 (cheap-test scope-of-resolution)
- [Simulation-Based Studies in Software Engineering: A Matter of Validity](http://www.scielo.edu.uy/scielo.php?script=sci_arttext&pid=S0717-50002015000100005) — explicit framing of "lack of evidence regarding model validity reduces findings only to the simulation model"; precedent for CRIT-4 (inherited-substrate disclosure)
- [Sensitivity analysis for a Bitcoin simulation model (ScienceDirect)](https://www.sciencedirect.com/science/article/pii/S2666281722001305) — Docker / fixed-seed reproducibility patterns; precedent for MOD-6 (determinism scope-creep) and the hash-diversity sanity-check (MOD-4)
- [Golden Tests (Tom Sydney Kerckhove, Casper Blockchain)](https://medium.com/casperblockchain/golden-tests-e521077ae235) — golden-hash regression fragility patterns; precedent for the goldens-pinning narrative
