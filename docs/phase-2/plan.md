## Phase 2: Hypothesis formation, experimentation and design space reduction
Plan:

Will: Experiments

Polina: Formal spec continuation

Andre: Formalisation of statistical properties

Nicolas: Prototype Polina's ledger changes on Cardano ledger

Stop criteria:

* No candidates survive the experiment elimination criteria (they all fail to beat the flat-fee and/or EIP-1559 controls on the core welfare metrics)
* Inclusion degradation exceeds the agreed threshold under all designs
* Observable urgency separation / price discrimination is not achievable under any design
* Research questions block progress and can't be resolved in this phase

-----

Design space entering phase 2:

The phase-1 writeup identified a large design space. For the purposes of experiments, we've collapsed it into 5 independent axes, each with a small number of levels:

* **Pricing mechanism type**: Baseline (flat fee), EIP-1559 (dynamic pricing, no tiers), tiered (multi-tier with delays)
* **Block path selection policy**: Shared (one tier space across RBs and EBs), naive RB/EB (hard two-tier partition, RB = fast, EB = slow), continuous RB/EB (tiers span both paths with per-path pricing), RB tier-0 reserved
* **Overflow handling**: None, reject-only, overflow as pricing signal, overflow as tier-update signal
* **Tier structure**: maximum tier count 2 / 4 / 12 / 50
* **Traffic scenario**: paper_like (heterogeneous standard mix), sundaeswap_congestion (demand spike with high-urgency bots)

The baselines (flat fee, EIP-1559) serve as controls against which all tiered configurations are measured. If additional axis levels emerge during phase 2 (for example, from the formal spec work surfacing a constraint that suggests a new design), they can be added, but should go through a lightweight evaluation before entering the experiment pipeline.

-----

Experiments workstream:

Using the output of phase 1, specifically the "Questions for experiments":

1. Does the paper's solution applied to linear-Leios result in performance greater than or equal to its application to a Praos-like structure?
2. How does the EB = slow lane, RB = fast lane shaped solution compare to the paper-like solution's performance?
3. ~~How much does it affect each solution candidate if RBs can include both an EB certificate and transactions?~~ - This question is retired: linear-Leios will not accept a design in which RBs can include both an EB certificate and inline transactions, so this branch of the design space is closed by protocol constraint rather than experiment.
4. Which solutions are most versatile in terms of load distribution variations?
5. At what repricing frequency does oscillation emerge, and how does this vary by load profile?
6. Does the mechanism achieve price discrimination (different urgency classes paying different fees) under each candidate design?

The approach is one-at-a-time (OAT): vary one axis of the design space at a time, holding everything else fixed at a pivot configuration, so that each result is attributable to a single variable change. The design space is collapsed into 5 axes: pricing mechanism type, block path selection policy, overflow handling, tier structure, and traffic scenario.

The experiments proceed in four stages: - Probably use AWS

**Stage A.** OAT sweep (Q1-Q2 and Q4-Q5, including a repricing-frequency sub-sweep): For each axis, vary its levels while holding everything else at the pivot. Each configuration is run with multiple seeds. This covers the active structural questions (does tiered pricing beat baseline? which block path policy wins? how does overflow handling affect things? how many tiers?) in a systematic way. In addition, this stage includes two targeted controls: `leios-tiered-eb-off` as the Praos-like control for Q1 (same tiered mechanism, EBs disabled), and a small dedicated sweep of `base_fee_change_denominator` under both traffic scenarios so that Q5 is answered directly rather than only inferred from default runs.

- 2 weeks

**Stage B.** Close call check: To resolve the case where there's a close call between two or more points on any given axis.

- 0.5 weeks (may be zero; if not needed, time rolls into stage C)

**Stage C.** Winner assembly and validation: Take the best setting per axis, combine them into one configuration, and validate that the combination actually improves on the pivot and the baselines. If the combination underperforms, the OAT data tells us which axis pairing is responsible.

- 0.5 weeks

**Stage D.** Stress testing and welfare validation (Q4, Q6): Run the validated winner under varied load conditions and traffic scenarios. Confirm that it achieves price discrimination (different urgency classes paying different fees and experiencing different delays). This is the paper's core promise; if the surviving mechanism doesn't achieve this, it isn't doing what we set out to do.

- 1 week

Elimination criteria:

Before the sweep starts, we should agree provisional numeric thresholds for "too much" inclusion loss and "unacceptable" oscillation. These can be revised later, but they should be explicit before we start eliminating candidates.

* Baseline-comparison: a candidate must show a clear welfare improvement over at least one control (flat-fee and/or EIP-1559), judged on the core welfare metrics: retained value ratio, net utility, diversity, and inclusion rate. To become the recommended phase-2 winner, it should beat both controls overall unless there is a clear reason to carry multiple candidates forward.
* Urgency-separation: a candidate must show observable urgency separation, i.e. higher-urgency classes should, on average, pay higher fees and experience lower delays than lower-urgency classes.
* Inclusion-loss: a candidate must not exceed the agreed inclusion-loss threshold relative to the relevant baseline under the evaluated load profile.
* Stability: a candidate must not show unacceptable oscillation in prices over time.

If a configuration fails any of these criteria, it is eliminated before any finer-grained ranking between survivors.

Dependencies: Stages are sequential; each stage's output informs the next.

Deliverable: Per-axis comparison tables (retained value ratio, inclusion rate, diversity, net utility, price stability) broken down per-urgency-class, plus an explicit summary of repricing stability thresholds. A written summary of which design choices were eliminated, which criterion each eliminated design failed, and which survived into the combined winner.

-----

Formal spec workstream:

Work will continue on the ledger specification until it's complete. This includes the delay-aware transaction validation rules and the new protocol parameters. Delay enforcement itself is a mempool-level concern, not a ledger one, but the ledger needs to know that a transaction has waited its specified delay. The ledger spec will include assumptions that certain linear-Leios functions exist.

Informal assessment of mempool change feasibility: what changes are needed to support immature transaction tracking and tier-aware fetching?

Additionally, Polina will draft a CIP for a fee change return mechanism. This is a separate deliverable from the ledger spec work and will be produced in the first part of the phase, once the ledger specification is complete.

Dependencies: None initially. However, milestone 1 requires the spec to be complete enough for the prototype workstream to begin building on it, so there's a soft dependency between this workstream and the prototype's start date.

Deliverable: Completed ledger formal spec with delay-aware validation rules. A written assessment of mempool change feasibility. A draft CIP for a fee change return mechanism.

-----

Statistical formalisation workstream:

This phase will be formalisation of mechanism-agnostic properties. These are the properties that any tiered pricing mechanism should satisfy, regardless of the specifics of the design. The specific properties are Andre's to define.

Once this is complete, we can verify that our experimental designs satisfy these properties.

Mechanism-specific properties (derived from the "winners" of the experiment workstream) are deferred to phase 3.

Dependencies: None.

Deliverable: A document defining the mechanism-agnostic properties, including for each property: a precise statement and a justification for why the property matters. High-level specification - how any tiered pricing system should behave.

-----

Praos prototype workstream:

This will be dependent on the formal spec workstream to a degree.

We'll be able to run a simple Praos version of dynamic pricing (not tiers) by the end of the phase.

Dependencies: Formal spec workstream

Deliverable: Praos with dynamic pricing (not tiered pricing yet)

-----

Analysis dimensions:

In addition to the experiment-driven comparison, we should produce lightweight analysis across the following dimensions for each surviving candidate. These won't all be deep investigations; some will naturally fall out of the experiment results, and others will be brief qualitative assessments.

* **UX tradeoffs**: How complex is tier selection from the user's perspective? Does the wallet need to reason about multiple tier-sets (RB vs EB)? How predictable is latency?
* **Security tradeoffs**: Does the design increase the surface area for MEV extraction? How much urgency information is revealed by tier choice? (Building on the preliminary security discussion from phase 1.)
* **Revenue generation**: Does the candidate maintain or improve on baseline revenue? This falls out of the experiment metrics directly.
* **Throughput effects**: Does the mechanism reduce effective throughput (for example, by fragmenting block space into under-utilised tiers)?
* **Fairness**: Does the candidate achieve the paper's promise of inclusion for low-urgency transactions? Per-urgency-class welfare breakdowns from the experiments are the primary input here.
* **Implementation complexity**: A rough qualitative assessment of how much work each candidate would require to implement on Cardano, informed by the formal spec and prototype workstreams.

These don't need their own workstream. They should be addressed as part of the phase-2 writeup, drawing on outputs from all four workstreams.

-----

Community involvement:

Early feedback of experiment results to Carlos, so he can engage the community. Specifically:

* After milestone 1 (structural down-select), share the reduced candidate set and the reasoning behind eliminations with Carlos
* If experiment results are surprising or challenge assumptions from phase 1, flag these early rather than waiting for the writeup
* Carlos can use preliminary results to start socialising the direction with interested community members, without committing to a specific design

-----

Work organisation:

Spanning the whole phase:

* Weekly stakeholders meeting
    * Checklist items for weekly stakeholders meeting:
        * Experiment results: anything new to share?
        * Candidate set: any candidates eliminated or added?
        * Cross-workstream blockers: is the spec on track for the prototype?
* Weekly tech discussion, for technical team members only
    * Checklist items for weekly team members meeting:
        * Questions for research: up to date?
        * Glossary: up to date?
        * Implementation challenges: anything new surfaced by the spec or prototype work?

Block 1 (2 weeks duration):

* CPS draft: Will 0.5 weeks (to be revisited when Andre's got some properties)
* Experiment tooling: batch run orchestration, suspendable/continuable runs - Will (1.5 weeks)
* Fee change return mechanism CIP - Polina (1 week)
* Formal spec: continue ledger specification - Polina (1 week)
* Statistical formalisation: High-level spec - Andre (2 weeks)
* Prototype: prep work, familiarisation with the spec changes so far - Nicolas (2 weeks)

Block 2 (3 weeks duration):

* Experiments: OAT sweep, repricing sub-sweep, close-call check, winner assembly (phases A-C) - Will (3 weeks) ← this is the bulk of the experimental work
* Formal spec: continue ledger specification - Polina (3 weeks)
* Statistical formalisation: Properties - Andre (3 weeks)
* Prototype: begin Praos dynamic pricing implementation, building on the spec - Nicolas (3 weeks)

Block 3 (2 weeks duration):

* Experiments: stress testing and welfare validation (phase D) - Will (1 week), then analysis (1 week)
* Formal spec: continue ledger specification, conclude mempool feasibility assessment - Polina (2 weeks)
* Statistical formalisation: Properties (continued) - Andre (2 weeks)
* Prototype: continue Praos dynamic pricing implementation - Nicolas (2 weeks)

Block 4 (4 weeks duration):

* CIP skeleton (+ tech report): draft CIP structure and initial content, informed by experiment results - Will (4 weeks)
* Formal spec: complete ledger specification - Polina (4 weeks)
* Statistical formalisation: Properties (continued) + documentation - Andre (4 weeks)
* Prototype: complete Praos dynamic pricing implementation - Nicolas (4 weeks)

Block 5 (1 week duration):

* CIP skeleton: all workstreams contribute their sections - Will (1 week)

Total elapsed: 12 weeks

-----

Work plan risks:

* Experiments narrow the selection-set too late to steer other workstreams in an efficient manner

* Prototype workstream could outpace formal work, which would risk formal work invalidating the prototype later

* Experiment tooling (batch orchestration, suspendable runs) takes longer than the 2-week block 1 allocation, delaying the OAT sweep

-----

Risk mitigation:

* Experiments narrowing too late: If the OAT sweep takes longer than expected, we should prioritise getting at least a preliminary per-axis ranking to the team rather than waiting for complete results.
* Prototype outpacing formal work: Nicolas's block 1 is intentionally lighter (prep and familiarisation) to give Polina time to get the spec into a state that the prototype can build on. The weekly tech discussion should surface any divergence early.
* Tooling delay: if the batch orchestration or suspendable-run tooling overruns block 1, Will can begin the OAT sweep manually (one config at a time) while finishing the tooling in parallel. The tooling is a throughput optimisation, not a blocker - individual runs can be launched without it.
* Andre's mechanism-agnostic formalisation has no dependency on the experiment results, but the 12-week timeline means Andre may finish well before the phase ends. If so, he can begin scoping mechanism-specific properties (to be completed in phase 3) using the reduced candidate set from milestone 1.

-----


Milestone 1: Structural down-select (end of block 2, week 5)
  - Experiments: OAT sweep complete, winner assembled and validated (phases A-C)  
  - Spec: fee change return mechanism CIP drafted
  - Spec: ledger changes complete enough for prototype work
  - Stats: mechanism-agnostic properties in progress
  Sync: team reviews which design choices survived

Milestone 2: Validated design (end of block 3, week 7)
  - Experiments: stress testing and welfare validation complete (phase D)  
  - mempool feasibility assessed
  Sync: team reviews validated winner, informs remaining work

Milestone 3: Phase wrap-up (end of block 5, week 12)
  - Prototype: simple Praos version running
  - Spec: ledger specification complete
  - Stats: mechanism-agnostic properties complete
  Sync: team reviews, CIP skeleton serves as phase 2 wrap-up document





Wrap-up deliverable:

A CIP skeleton (+ tech report) that doubles as the phase-2 wrap-up document. It subsumes what would otherwise be a separate writeup. Sections should include:

The CIP is about describing solution (can _reference_ evidence), everything else (evidence, experimentation) goes in the technical report

CIP:
* Architecture of remaining candidates, with justification for why they survived and what was eliminated
* Analysis dimensions (UX, security, revenue, throughput, fairness, implementation complexity) for surviving candidates
* A recommendation for which candidates to carry forward into phase 3 (might not be all of the survivors due to implementation complexity)
* Spec changes and mempool feasibility assessment
* Reference to fee change return mechanism CIP (Polina's separate deliverable)
* Statistical formalisation: mechanism-agnostic properties
* Status of the Praos (or Leios) prototype
* Updated open questions for research and experiments, to be addressed in phase 3 - Ideally we won't have any open questions at this point though?
* Concretely identified implementation challenges

Experiment report:
* Summary of experiment results: what was tested, per-axis comparisons, stress test outcomes

Some sections (experiment results, eliminated candidates, analysis dimensions) should be substantive by the end of phase 2. Others (final recommendation, complete formal spec, community feedback) will be filled in during phase 3.