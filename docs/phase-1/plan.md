## Phase 1: Requirements gathering and research analysis
Plan:  


* Study the paper, start considering challenges - 1 week
    * While we're not necessarily documenting questions explicitly at this stage, we'll be informally considering how the shape of the paper's solution maps onto Cardano and linear Leios
    * This should include the beginning of requirements gathering (discuss with Carlos), which will likely span the whole of phase 1 in the background

    Deliverables: Initial requirements and constraints doc, which will be updated as the project progresses and a glossary of terms, to also be updated as the project progresses

* Go into the limitations - 1 week
    * Separately to the challenges of mapping the paper onto Cardano, we should identify gaps between the functionality offered by the paper's solution and the desired functionality    
    * It'll also include considerations, such as, for example, whether or not the paper's solution empowers front-running adversaries
    * In other words, is there anything we don't like about the functionality offered by the paper's solution?

    Deliverables: A list of things we don't like about the functionality offered by the paper's solution (if any)

* Gather the initial questions we'll need to answer... - 1 week
    *  ...in order to understand if the task is possible 
        * These might be:
            * Questions that research can answer
            * Questions that we can answer with experiments such as a prototype or a simulation
            * Questions with a product/UX perspective
        * This won't be experiment design yet
        * This represents the "official" gathering and simplification of questions that'll be raised all throughout the phase

    Deliverables: A set of questions, with a low-resolution description (research, product, experiment etc) of how we think we can answer each one

* Gather the challenges that we will have in the context of linear Leios (and in the context of Cardano in general) - As a part of the above
    * This will be a reification of the output of the earlier task: "Start considering the challenges", where we'll write down, in a clear and digestible manner, what engineering challenges are raised by the mapping of the paper's solution onto Cardano and linear Leios
    * Examples of already known challenges:
        * Leios delays emerge from volume spikes above RB capacity; the paper assumes delays are mechanism-controlled. How do we reconcile these models?
        * If we have dynamic tiers, how does tier removal work? For example: a tx is submitted to a tier with +2 block delay, but at T+1 that tier is removed. Does the tx fail validation? Or does removal only prevent new submissions while in-flight txs continue to settle?
        * Plutus fees are already calculated in 2-dimensions. How does that work here? 
        * Block fullness isn't just byte size, ExUnits also count towards capacity. How does this affect tier price calculation?
        * How do we track in-limbo transactions?

    Deliverables: A list of known challenges

* Prototype prep - 1 week
    * As a precursor for our experiments in subsequent phases, we should attempt to identify relevant properties of the problem and solution spaces discovered as the outputs of prior tasks
    * Polina to think about properties, and which ones might be better proven statistically vs formally
        * Polina to think about what might be the optimal language to use for FM work
        * Will to think about what might be the best way to build statistical/simulatory prototypes

    Deliverables:

        * Polina to produce:
            * A list of properties 
            * A discussion of which properties would be better suited to a formal spec or a statistical experiment of some sort
            * A decision on which language(s) would be best for this work, with justification
        * Will to produce:
            * A discussion on what engineering techniques we should use for our simulations


* Start making changes to relevant specs - 5 weeks:
    * Formal ledger spec changes
    * Formal consensus spec changes
    * Informal mempool spec changes

    This'll allow us to make a headstart on discovering any challenges specific to implementing the paper's solution on Cardano.

    Additionally, having the formal spec changes come first sequences nicely with work in subsequent phases where we'll most likely be building an engineering prototype on Cardano, because that work will be expedited by the existence of spec changes.

    Deliverables: Ledger and consensus type changes, description of mempool changes

* A simulation of the paper's solution in the context of Leios, Praos, and a comparison with a simpler but less rigorous "EB = slow lane, RB = fast lane" approach - 4 weeks
    * This would let us validate that the paper's benefits apply to Leios' structure, rather than the more simple Praos structure which it was likely written with the assumption of in mind.
    * Pros:
        * Allows us to do a direct comparison of the paper's solution applies to Leios, with Praos as the control
        * Will allow us to easily tweak variables to adapt to any updates provided by research
        * Seems more manageably sized
    * Cons:
        * Won't tell us anything about the concrete implementation challenges with regards to applying the paper's solution to Cardano

    Deliverables: A basic simulation of the paper's solution against Praos (as a control) and Leios. A simulation of the "EB = slow lane, RB = fast lane" approach

* We'd ideally like to end up with some form of "wrapping-up-the-phase" style documentation, whether some official "report" or just internal notes, briefly covering: - 1 week
    * A statement of what we're optimising for
    * Recommendation of structural (RB = fast, EB = slow type thing) vs more structure agnostic paper-like solution, with justification     ← This might be a bit much for phase 1, but it's something I've already started to develop an opinion on, so it may naturally pop out of the prior work
    * Preliminary assessment of ledger impact ← This shouldn't be in-depth, and will be purely based on the output of prior tasks, hence why it doesn't come with its own task
    * Preliminary discussion of how this works with Plutus (Plutus fees being 2 dimensional) ← As above, not in-depth, and based on the output of prior tasks
    * Preliminary security discussion (front-running etc)    ← As above    
    * Open questions which require prototypes/simulations to answer ← The questions from earlier tasks arranged in a clear and digestible manner
    * Open questions which require research input to answer ← As above
    * Brief discussion on what good UX might look like
    * Discussion of design space options (such as 2-tier fixed vs N-tier dynamic vs hybrid) to drive actions in phase 2 ← Not in-depth, just some options
        * This should ideally include definitions of "tier", "delay", "inclusion", "maturity" and similar terms for each design space option, if possible at this point

Work organisation:

Spanning the whole phase:

* Weekly team meeting, including stakeholders (regular opportunity to keep Carlos looped in)
    * Checklist items for weekly stakeholders meeting:
        * Requirements & constraints doc - Up to date?
* Weekly tech discussion, for team members only
    * Checklist items for weekly team members meeting:
        * Glossary - Up to date?
        * Questions for research - Up to date?
        * Challenges with implementation on Cardano - Up to date?
        * Functional limitations - Up to date?

Block 1 (1 week duration):
Study the paper, start considering challenges of applying it to Cardano + linear Leios - Polina + Will (concurrent, total 2 man-weeks)

Block 2 (1 week duration)
Go into the limitations; what does the paper fail to resolve? One initial example could be front-running adversaries - Polina + Will (concurrent, total 2 man-weeks)

Block 3 (1.5 week duration)

* Gather questions + challenges, start discussing these with research - Will (1 weeks)
* Prototype prep:
    * Think about properties, and which ones might be better proven statistically vs formally - Polina (1 weeks)
    * Think about what might be the optimal language for FM work - Polina (0.5 weeks)
    * Think about what might be the best way to build statistical/simulatory prototypes - Will (0.5 weeks)

Block 4 (5 week duration)

Simulation of the paper's solution - Will (4 weeks)
Write-up - Will (1 week)
Formal spec changes - Polina (5 weeks)

Total elapsed: 8.5 weeks
Work plan risks:


* Concurrent work in block 4 on the spec could invalidate effort already spent on the simulation, or vice versa
    * This will be mitigated by informal knowledge sharing as a normal part of team dynamics, but it's possible that something's discovered in, say, week 3 of the block which invalidates earlier work
* If multiple challenges or limitations are raised in blocks 1 and 2, there's potential for research, depending on their capacity, to be a bottleneck preventing progression to block 4 (and potentially even block 3)


"Stop" criteria (what'd prevent us from progressing to phase 2):

* There is no mapping (which satisfies our constraints) from the paper onto Cardano + linear Leios and we can't come up with any alternative solutions
* There's no product fit
* Priority shifts away from tiered pricing