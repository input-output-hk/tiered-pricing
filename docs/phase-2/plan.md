## Phase 2: Dynamic pricing validation and implementation scoping

Phase 2 pivot: From tiered pricing to dynamic pricing

During the interim planning period between phase 1 and phase 2, feedback from community discussions suggested that a full-fat tiered pricing mechanism wasn't an area of interest, but the community members _were_ interested in some simple way to signal urgency.

As such, we've shrunk the scope to what we're calling "dynamic pricing" for the sake of implementation simplicity. We define "dynamic pricing" as:

A transaction pricing mechanism in which the protocol adjusts the required fee over time in response to observed demand or congestion, rather than using a fixed fee. 

In this work, the main dynamic-pricing design is an EIP-1559-like fee coefficient: the fee coefficient raises when recent block utilisation is above a target level, lowers it when utilisation is below target, and applies the fee coefficient to ordinary transactions.

Other designs have been suggested, such as an expensive (5x or 10x base fee) fast lane which results in queue jumping. This is still much closer to our definition of dynamic pricing than it is to true tiered pricing.

Plan:

Will: Experiments

Polina: Formal spec continuation

Andre: Formalisation of statistical properties

Nicolas: Prototype Polina's ledger changes on Cardano ledger

Stop criteria:

* The dynamic-pricing candidate fails to beat or match the flat-fee baseline on the core welfare metrics
* Dynamic pricing introduces unacceptable inclusion loss, latency, or price instability
* A simple urgency signal cannot improve urgency outcomes without materially degrading inclusion, welfare, or implementation simplicity
* Research questions block progress and cannot be resolved in this phase


-----

Design space entering phase 2:

The phase-1 writeup identified a broad tiered-pricing design space. After the Phase 2 pivot, most of that design space is no longer primary. The experiment workstream now focuses on a smaller set of dynamic-pricing choices:

* **Control**: fixed flat fee
* **Main candidate**: EIP-1559-like dynamic fee coefficient
* **Dynamic-pricing parameters**: fee-change denominator, target utilisation, smoothing if needed
* **Optional urgency signal**: paid priority lane, e.g. 5x or 10x base fee, with either strict or capped priority capacity
* **Traffic scenario**: paper-like heterogeneous demand, congestion spikes, and high-urgency bot-like demand where useful

The earlier tiered-pricing mechanisms remain useful as background evidence and as a comparison point, but they are not the primary Phase 2 candidate set. New tiered designs should not be added unless a specific question requires them.


The fixed flat fee serves as the main control for the EIP-1559-like dynamic-fee candidate. If additional dynamic-pricing variants emerge during phase 2, they can be added, but should go through a lightweight evaluation before entering the experiment pipeline.

-----

Experiments workstream:

Using the output of phase 1 and the Phase 2 pivot, the experiment question set is:

1. Does an EIP-1559-like dynamic fee coefficient improve welfare, inclusion, latency, and stability relative to a fixed flat fee?
2. Which target utilisation and fee-change denominator ranges are robust across load profiles?
3. Does the dynamic fee remain stable under transient congestion and recovery?
4. Does a simple paid priority lane provide useful urgency signalling without materially degrading welfare, inclusion, or tail latency?
5. Under what load profiles does dynamic pricing fail, and are those failures caused by pricing behaviour or by effective Leios throughput limits?
6. What implementation and UX complexity does the recommended dynamic-pricing design introduce?

The approach is one-at-a-time (OAT): vary one design choice at a time, holding everything else fixed at a pivot configuration, so that each result is attributable to a single variable change. The active axes are pricing mechanism, dynamic-fee parameters, optional priority-signal policy, and traffic scenario.

The experiments proceed in four stages:

**Stage A. Dynamic-pricing baseline validation:** Compare fixed flat fee and EIP-1559-like dynamic pricing under the core traffic scenarios. Establish whether the dynamic fee coefficient is worth carrying forward.

- 1 week

**Stage B. Robustness sweep:** Sweep fee-change denominator and target utilisation under moderate and congested demand. Check inclusion, retained value, net utility, latency, and price stability across multiple seeds.

- 1 week

**Stage C. Simple urgency-signal check:** Test narrow priority-lane variants, such as 5x and 10x paid fast lanes, against the dynamic-fee baseline. The priority lane must preserve most of the base mechanism's welfare and inclusion before urgency separation matters.

- 0.5 weeks

**Stage D. Stress testing and implementation assessment:** Run the selected dynamic-pricing candidate under varied load conditions.

- 1 week

Elimination criteria:

* **Baseline comparison:** the dynamic-pricing candidate should beat or closely match fixed flat fees on retained value, net utility, inclusion rate, and latency.
* **Robustness:** the mechanism should not depend on a single fragile parameter setting.
* **Stability:** the fee coefficient must not show unacceptable oscillation over time.
* **Urgency extension discipline:** a priority-lane variant is eliminated if it improves urgency ordering only by causing material welfare loss, inclusion loss, or tail-latency degradation.
* **Implementation simplicity:** a candidate that requires delayed transaction validation, complex wallet-side urgency selection, or substantial mempool machinery should be treated as outside the narrowed Phase 2 scope unless explicitly requested by community members and it dramatically outperforms the simpler dynamic-fee mechanism.

If a configuration fails any of these criteria, it is eliminated before any finer-grained ranking of survivors.

Dependencies: Stages are sequential; each stage's output informs the next.

Deliverable: A recommendation for the dynamic-pricing mechanism and parameter range to carry forward, with comparison tables covering retained value, inclusion rate, net utility, latency, fee stability, and per-urgency-class outcomes. The report should also explain that (fully-fledged) tiered-pricing ideas were deliberately dropped from scope and why.

-----

Formal spec workstream:

Work will continue on the ledger specification until it's complete. After the pivot, the priority is the protocol and ledger surface needed for dynamic fee coefficients, including how fee coefficients are observed, applied, and updated.

Informal assessment of mempool change feasibility: what changes are needed to support dynamic fee updates and any narrow paid-priority mechanism?

Additionally, Polina will draft a CIP for a fee change return mechanism. This is a separate deliverable from the ledger spec work and will be produced in the first part of the phase, once the ledger specification is complete.

Dependencies: None initially. However, milestone 1 requires the spec to be complete enough for the prototype workstream to begin building on it, so there's a soft dependency between this workstream and the prototype's start date.

Deliverable: Completed ledger formal spec for dynamic fee coefficients. A written assessment of mempool change feasibility. A draft CIP for a fee change return mechanism.

-----

Statistical formalisation workstream:

This phase will be formalisation of mechanism-agnostic properties. These are the properties that any dynamic pricing mechanism should satisfy, regardless of the specifics of the design. The specific properties are Andre's to define.

Once this is complete, we can verify that our experimental designs satisfy these properties.

Mechanism-specific properties (derived from the recommended experiment candidates) are deferred to future work.

Dependencies: None.

Deliverable: A document defining the mechanism-agnostic properties, including for each property: a precise statement and a justification for why the property matters. High-level specification - how any dynamic pricing system should behave.

-----

Prototype workstream:

This will be dependent on the formal spec workstream to a degree.

We'll be able to run a simple linear-Leios (prototype) version of dynamic pricing by the end of the phase. This will be based on the following branches:

https://github.com/IntersectMBO/cardano-node/tree/leios-prototype
https://github.com/IntersectMBO/cardano-ledger/tree/leios-prototype
https://github.com/IntersectMBO/ouroboros-consensus/tree/leios-prototype
https://github.com/IntersectMBO/ouroboros-network/tree/leios-prototype
https://github.com/IntersectMBO/cardano-api/tree/leios-prototype

Dependencies: Formal spec workstream

Deliverable: Linear-Leios with dynamic pricing

-----

Analysis dimensions:

In addition to the experiment-driven comparison, we should produce lightweight analysis across the following dimensions for each recommended candidate. These won't all be deep investigations; some will naturally fall out of the experiment results, and others will be brief qualitative assessments.

* **UX tradeoffs**: How predictable is the fee? Can wallets explain the dynamic fee coefficient and any optional priority lane clearly?
* **Security tradeoffs**: Does the design increase the surface area for MEV extraction? How much urgency information is revealed by a paid priority-lane choice? (Building on the preliminary security discussion from phase 1.)
* **Revenue generation**: Does the candidate maintain or improve on baseline revenue? This falls out of the experiment metrics directly.
* **Throughput effects**: Does the mechanism reduce effective throughput, for example by reserving too much capacity for an optional priority lane?
* **Fairness**: Does the candidate preserve reasonable access for low-value and low-urgency transactions? Per-urgency-class welfare breakdowns from the experiments are the primary input here.
* **Implementation complexity**: A rough qualitative assessment of how much work each candidate would require to implement on Cardano, informed by the formal spec and prototype workstreams.

These don't need their own workstream. They should be addressed as part of the phase-2 writeup, drawing on outputs from all four workstreams.

-----

Community involvement:

Early feedback of experiment results to Carlos, so he can engage the community. Specifically:

* After milestone 1 (dynamic-pricing down-select), share the reduced candidate set and the reasoning behind eliminations with Carlos
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

* Experiments: dynamic-pricing baseline validation, robustness sweep, and simple urgency-signal check (phases A-C) - Will (3 weeks) ← this is the bulk of the experimental work
* Formal spec: continue ledger specification - Polina (3 weeks)
* Statistical formalisation: Properties - Andre (3 weeks)
* Prototype: begin dynamic pricing implementation, building on the spec - Nicolas (3 weeks)

Block 3 (2 weeks duration):

* Experiments: stress testing and welfare validation (phase D) - Will (1 week), then analysis (1 week)
* Formal spec: continue ledger specification, conclude mempool feasibility assessment - Polina (2 weeks)
* Statistical formalisation: Properties (continued) - Andre (2 weeks)
* Prototype: continue dynamic pricing implementation - Nicolas (2 weeks)

Block 4 (4 weeks duration):

* CIP skeleton (+ tech report): draft CIP structure and initial content, informed by experiment results - Will (4 weeks)
* Formal spec: complete ledger specification - Polina (4 weeks)
* Statistical formalisation: Properties (continued) + documentation - Andre (4 weeks)
* Prototype: complete dynamic pricing implementation - Nicolas (4 weeks)

Block 5 (1 week duration):

* CIP skeleton: all workstreams contribute their sections - Will (1 week)

Total elapsed: 12 weeks

-----

Work plan risks:

* Experiments narrow the selection-set too late to steer other workstreams in an efficient manner

* Prototype workstream could outpace formal work, which would risk formal work invalidating the prototype later

* Experiment tooling (batch orchestration, suspendable runs) takes longer than the 2-week block 1 allocation, delaying the dynamic-pricing robustness sweep

-----

Risk mitigation:

* Experiments narrowing too late: If the robustness sweep takes longer than expected, we should prioritise getting at least a preliminary parameter ranking to the team rather than waiting for complete results.
* Prototype outpacing formal work: Nicolas's block 1 is intentionally lighter (prep and familiarisation) to give Polina time to get the spec into a state that the prototype can build on. The weekly tech discussion should surface any divergence early.
* Tooling delay: if the batch orchestration or suspendable-run tooling overruns block 1, Will can begin the dynamic-pricing sweep manually (one config at a time) while finishing the tooling in parallel. The tooling is a throughput optimisation, not a blocker - individual runs can be launched without it.
* Andre's mechanism-agnostic formalisation has no dependency on the experiment results, but the 12-week timeline means Andre may finish well before the phase ends. If so, he can begin scoping mechanism-specific properties (deferred to future work) using the reduced dynamic-pricing candidate set from milestone 1.

-----

Milestone 1: Dynamic-pricing down-select (end of block 2, week 5)
  - Experiments: baseline validation, robustness sweep, and simple urgency-signal check complete (phases A-C)  
  - Spec: fee change return mechanism CIP drafted
  - Spec: ledger changes complete enough for prototype work
  - Stats: mechanism-agnostic properties in progress
  Sync: team reviews the recommended dynamic-pricing candidate and parameter range

Milestone 2: Validated design (end of block 3, week 7)
  - Experiments: stress testing and welfare validation complete (phase D)  
  - mempool feasibility assessed
  Sync: team reviews validated dynamic-pricing recommendation, informs remaining work

Milestone 3: Phase wrap-up (end of block 5, week 12)
  - Prototype: simple dynamic pricing running
  - Spec: ledger specification complete
  - Stats: mechanism-agnostic properties complete
  Sync: team reviews, CIP skeleton serves as phase 2 wrap-up document





Wrap-up deliverable:

A CIP skeleton (+ tech report) that doubles as the phase-2 wrap-up document. It subsumes what would otherwise be a separate writeup. Sections should include:

CIP:
* Architecture of remaining dynamic-pricing candidates, with justification for why they survived and what was eliminated
* Analysis dimensions (UX, security, revenue, throughput, fairness, implementation complexity) for recommended candidates
* A recommendation for which candidates to carry forward into future work (might not be all of the viable candidates due to implementation complexity)
* Spec changes and mempool feasibility assessment
* Reference to fee change return mechanism CIP (Polina's separate deliverable)
* Statistical formalisation: mechanism-agnostic properties
* Status of the Praos (or Leios) prototype
* Updated open questions for research and experiments, to be addressed in future work - Ideally we won't have any open questions at this point though?
* Concretely identified implementation challenges

Experiment report:
* Summary of experiment results: what was tested, per-axis comparisons, stress test outcomes

Some sections (experiment results, eliminated candidates, analysis dimensions) should be substantive by the end of phase 2. Others (final recommendation, complete formal spec, community feedback) are deferred to future work.
