# Architecture — CIP evidence-base artefacts

**Project:** phase-2 CIP evidence audit
**Researched:** 2026-05-15
**Scope:** how CIP-grade evidence packages are structured as artefacts, and
how those artefacts integrate with a mature simulator codebase. This
document informs phase structure in the roadmap.
**Explicit non-scope:** the simulator's own component architecture, which
is already captured in [`.planning/codebase/ARCHITECTURE.md`](../codebase/ARCHITECTURE.md)
and [`CLAUDE.md`](../../CLAUDE.md).

## TL;DR

The evidence base sits on top of the simulator as a small set of
**lightly cross-linked Markdown artefacts in `.planning/` and
`docs/phase-2/`** — not a new program, not a new directory tree, not a
new toolchain. The shape that fits both the CIP/CPS tradition and the
mature codebase is:

```
docs/phase-2/realism-risks-register.md   (NEW — central register)
  │
  ├──→ docs/phase-2/CPS-0023/README.md   (consumer: CIP author cites)
  │
  ├──→ docs/phase-2/cardano-realism-audit.md      (refreshed)
  ├──→ docs/phase-2/validity-threats.md           (existing — refreshed)
  ├──→ docs/phase-2/coverage-check.md             (NEW — claim → job map)
  │
  └──→ .planning/realism-tests/                   (NEW — per-LIVE-risk artefacts)
         ├── pool-number-sensitivity/
         │     ├── README.md            (test design + verdict)
         │     ├── results.md           (rendered table)
         │     └── jobs.yaml            (suite-fragment, runnable)
         ├── multi-seed-variance/
         ├── run-length-steady-state/
         └── …
```

The four core artefacts (register, coverage check, tests folder,
refreshed audit/validity-threats) all share a single **stable
identifier scheme** — `RRR-NN` for risks, `CLM-NN` for menu-item
claims — so cross-document cites are unambiguous and grep-able.

The simulator codebase itself is **read-only substrate** for this
milestone. Evidence artefacts cite simulator artefacts (suite YAMLs,
goldens, run summaries) but the simulator's source-of-truth manifests
under `sim-rs/output/<run-id>/` stay untouched.

## Recommended architecture

### Component diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│ docs/phase-2/CPS-0023/  (the problem statement, fixed)               │
│   Goals, Open Questions, Use Cases                                   │
└──────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ "responds to"
                              │
┌──────────────────────────────────────────────────────────────────────┐
│ CIP-XXXX (user authors; out of scope for this milestone)             │
│   - Abstract, Motivation, Specification, Rationale                   │
│   - Evidence subsection → cites Coverage Check + Register            │
│   - Limitations subsection → cites disclosed RRR entries             │
└──────────────────────────────────────────────────────────────────────┘
            │                              │
            │ cites                        │ cites
            ▼                              ▼
┌──────────────────────────────┐  ┌────────────────────────────────────┐
│ docs/phase-2/coverage-check  │  │ docs/phase-2/realism-risks-register│
│   CLM-01 → suite/job/cell    │  │   RRR-01: status, evidence,        │
│   CLM-02 → suite/job/cell    │  │           disclosure framing       │
│   …                          │  │   RRR-02: …                        │
└──────────┬───────────────────┘  └────────────────┬───────────────────┘
           │ "backing evidence"                    │ "test artefact"
           │                                       │
           ▼                                       ▼
┌──────────────────────────────────┐  ┌────────────────────────────────┐
│ sim-rs/parameters/phase-2-sweep/ │  │ .planning/realism-tests/<name>/│
│   suites/<suite>.yaml            │  │   README.md   (design)         │
│   suites/.goldens/<suite>.json   │  │   results.md  (verdict)        │
│   (read-only, simulator owns)    │  │   jobs.yaml   (runnable)       │
└──────────────────────────────────┘  └────────────┬───────────────────┘
                                                   │ produces
                                                   ▼
                                      ┌────────────────────────────────┐
                                      │ sim-rs/output/<run-id>/        │
                                      │   manifest.json                │
                                      │   <job>/<seed>/run_summary.json│
                                      │   (simulator-owned ground      │
                                      │    truth; not committed)       │
                                      └────────────────────────────────┘
                  ▲
                  │ refreshed by milestone
                  │
┌──────────────────────────────────────────────────────────────────────┐
│ docs/phase-2/cardano-realism-audit.md  (existing — refresh)          │
│ docs/phase-2/validity-threats.md       (existing — refresh)          │
│   These are the *current* register + per-suite trust matrix.         │
│   Refresh = drop annotations, fold into RRR-NN scheme, re-issue.     │
└──────────────────────────────────────────────────────────────────────┘
```

### Component responsibilities

| Component | Role | New / Existing | Source of truth for |
|-----------|------|---------------|---------------------|
| `docs/phase-2/realism-risks-register.md` | Single document, RRR-NN entries with status (LIVE / DORMANT / MITIGATED / DISCLOSED), category, impact-if-real, current evidence, decision, disclosure framing. | NEW | Status of every realism risk; what the CIP discloses; which risks have backing tests. |
| `docs/phase-2/coverage-check.md` | Claim-to-evidence matrix: CLM-NN rows × (claim text, menu option, backing suite, backing job/cell, status, RRR risks affecting). | NEW | Whether every menu-item trade-off claim has a specific simulator job behind it; surfaces gaps. |
| `.planning/realism-tests/<name>/` | One subfolder per LIVE-risk targeted test. `README.md` = test design + threshold; `results.md` = rendered verdict; `jobs.yaml` = runnable suite-fragment when applicable. | NEW (folder) | Per-test design, threshold, verdict; runnable inputs that regenerate the result. |
| `docs/phase-2/cardano-realism-audit.md` | Per-category calibration audit (RB cadence, fee structure, controller calibration, topology). | EXISTING — refresh | Calibration-anchor claims (mainnet-matching values). |
| `docs/phase-2/validity-threats.md` | Per-suite trust matrix (19 suites × HIGH/MEDIUM/LOW + caveats). | EXISTING — refresh | Per-suite trust verdict; per-suite caveats. |
| `.planning/codebase/*.md` | Codebase reverse-engineering (architecture, structure, testing). | EXISTING — read-only | Simulator architecture; not a milestone artefact. |
| `sim-rs/**` | Simulator codebase. | EXISTING — read-only this milestone | Mechanism implementation; not edited by this milestone except for any cheap tests that need new suite YAMLs. |
| `sim-rs/parameters/phase-2-sweep/suites/<name>.yaml` | Goldens-pinned phase-2 suites. | EXISTING; may add `.yaml` for cheap tests | Suite definitions cited from coverage check. |
| `sim-rs/output/<run-id>/manifest.json` + `<job>/<seed>/run_summary.json` | Per-run ground truth. | EXISTING; uncommitted on disk | Numerical results cited by `results.md` snippets. |

### Data flow — register entry lifecycle

A risk moves through a fixed pipeline. The arrows are mostly Markdown
edits, not code:

```
[1] Inventory      → [2] Triage       → [3] Cheap test     → [4] Verdict     → [5] Disclose
    (sweep audit       (categorise         (run targeted        (interpret        (write framing
     trail + audit      LIVE / DORMANT      jobs; record         table; mark       paragraph for
     surface)           / MITIGATED         in .planning/        as MITIGATED      CIP author to
                        / DISCLOSED)        realism-tests/)      or DISCLOSED)     paste into CIP)
       │                   │                   │                   │                 │
       │                   │                   │                   │                 │
       ▼                   ▼                   ▼                   ▼                 ▼
 RRR-NN created       status assigned     results.md +         status flipped    final-section
 (status = TBD)       (LIVE / DORMANT     verdict line in      to MITIGATED      paragraph in
                       / MITIGATED /      RRR-NN entry         or DISCLOSED      RRR-NN; cited
                       DISCLOSED)                                                from CIP
```

**Two terminal states**, both acceptable for CIP publication:
- **MITIGATED** — cheap test resolved the risk; CIP says nothing beyond
  the headline result.
- **DISCLOSED** — cheap test surfaced a real limitation, or the risk
  was too expensive to test; CIP includes a paragraph naming the gap
  and explaining why publication proceeds anyway.

Either is fine. The shape "register entry → cheap test → verdict →
disclosure" is the same path either way; only the terminal status
differs.

### Stable identifier scheme

Two cross-document ID prefixes, matching the Leios ImpactAnalysis.md
precedent (which uses REQ-, NEW-, UPD-, RSK-, ATK-, EXP-).

- **`RRR-NN`** — Realism-risks register entry. Lives in
  `realism-risks-register.md`. Referenced from `coverage-check.md` as
  a "risks affecting this claim" column, from the refreshed
  `validity-threats.md`, and from the CIP's Limitations section. ID is
  stable across edits; status field is the mutable bit.
- **`CLM-NN`** — Menu-item trade-off claim. Lives in
  `coverage-check.md`. Referenced from the CIP. Each CLM names the
  menu option it concerns and the trade-off direction (e.g., "priority-only-RB-reserved
  yields ≥ X welfare improvement on demand-regime Y under multiplier-floor Z").

Two prefixes, both numeric-sortable, both grep-able. No deeper
hierarchy: the cost of a deeper scheme (RRR-NN.M sub-entries, claim
families) exceeds the benefit at this milestone scale (~10-20 RRR
entries, ~10-20 CLM entries expected).

### Build order — register before coverage before tests

This is the critical dependency order. The register is upstream of
the coverage check, which is upstream of the per-test results
artefacts.

```
Phase 1: Register Inventory               (must run first)
   ├─ Sweep existing artefacts for risks already named
   │  (cardano-realism-audit.md, validity-threats.md, CONCERNS.md,
   │   REVIEW.md, mechanism-welfare-impact-2026-05-14.md, the 7
   │   spike READMEs).
   ├─ Inventory + de-duplicate into RRR-NN entries.
   └─ Initial triage: status = LIVE / DORMANT / MITIGATED / DISCLOSED
      based on existing evidence only. Each LIVE flagged for a cheap
      test in Phase 3.

Phase 2: Coverage-check Skeleton          (can overlap Phase 1 tail)
   ├─ Walk CPS-0023 Goals + Open Questions; map to menu options.
   ├─ Enumerate menu-item trade-off claims as CLM-NN.
   ├─ For each CLM-NN, identify backing suite/job/cell from existing
   │  results tables (family-b-results-table-2026-05-14.md, the 33-job
   │  smoke, the 7 goldens-pinned suites).
   └─ Surface gaps: claims with no backing job → flag as either
      "needs cheap test" (becomes a Phase 3 work item) or
      "out-of-scope, disclose" (joins the register as a DISCLOSED RRR).

Phase 3: Targeted Cheap Tests             (depends on Phase 1, 2)
   ├─ Per LIVE RRR-NN, decide test shape (smoke run, multi-seed,
   │  parameter sweep) and threshold.
   ├─ Author `.planning/realism-tests/<name>/{README.md, jobs.yaml}`.
   ├─ Run; record outputs in `results.md` (typically a small table
   │  pulled from run_summary.json + a verdict line).
   └─ Flip RRR-NN status to MITIGATED or DISCLOSED based on threshold.
      Also update CLM-NN rows in the coverage check that were
      blocked by this RRR.

Phase 4: Refresh + Anchor                 (depends on Phase 3)
   ├─ Refresh cardano-realism-audit.md (drop "[Annotation added
   │  2026-05-13]" framing; fold into authoritative rewrite).
   ├─ Refresh validity-threats.md (drop "Resolved" sections that are
   │  out of date; rewrite per-suite verdicts under the RRR-NN
   │  scheme).
   ├─ Anchor or disclose the 4 unanchored controller knobs
   │  (window-length 32, multiplier-floors 4 and 16, lane-signal
   │  sources). Each gets either a deployed-system anchor citation
   │  or a CIP-paragraph disclosure.
   └─ Final review pass: register, coverage check, audit, validity
      threats all read consistently and cite RRR-NN / CLM-NN
      identifiers correctly.

Phase 5: Handoff                          (depends on all prior)
   ├─ User-facing summary: "CIP author, paste these paragraphs into
   │  Limitations; cite these jobs in Evidence; the register lives
   │  here for follow-up readers."
   └─ Optional: a single-page "evidence map" diagram showing CIP
      claims → coverage check → suites → run artefacts.
```

The ordering matters because:
- **Coverage check before tests:** the coverage check surfaces *which*
  claims need backing. Running tests before knowing which claims are
  unbacked risks spending compute on claims that already have evidence.
- **Register before coverage check:** the register's LIVE entries are
  the constraints on which claims are credible. A claim that depends
  on a LIVE risk needs that risk resolved first, otherwise the
  coverage check would mark it "backed" when it's really "backed
  conditional on RRR-NN."
- **Refresh after tests:** the existing `cardano-realism-audit.md`
  and `validity-threats.md` already include claims that some tests
  will invalidate or upgrade. Refreshing them before running the
  tests would force a second refresh.

### Where extensions plug in

Three natural extension points:

1. **Additional cheap tests after register surfaces them.** Phase 3 is
   designed as an unbounded loop: each LIVE RRR-NN slots a new
   `.planning/realism-tests/<name>/` subfolder. The 3-5 anticipated
   targeted tests beyond pool-number-sensitivity, multi-seed-variance,
   and run-length-steady-state become Phase 3 work items as the
   register inventory completes.
2. **New CLM-NN claims if menu evolves.** If the user revises the
   menu of mechanism options (e.g., adds a new variant or splits an
   existing one), the coverage check gains CLM rows; existing rows
   don't churn.
3. **External evidence anchors.** The 4 unanchored controller knobs
   may turn out to have deployed-system data (Ethereum mainnet
   reports, prior Cardano mempool studies) that anchor them rather
   than requiring disclosure. Phase 4 has a "look for anchors before
   writing disclosure" step that can plug external citations into RRR
   entries.

## Patterns to follow

### Pattern 1: Register entry as the unit of disclosure

**What:** Every realism limitation is one RRR-NN entry in
`realism-risks-register.md` with a fixed schema. The CIP's Limitations
section paragraphs *are* the "disclosure framing" field of the
relevant RRR entries, copy-pasted by the CIP author.

**When:** Always — every limitation lives somewhere; if not in the
register, it's invisible.

**Schema (per RRR-NN entry):**

```markdown
## RRR-07 — Mempool revalidation cost scaling

**Status:** DISCLOSED
**Category:** topology / actor model
**Impact if real:** Welfare numbers under heavy revalidation cycles
  may be optimistic by O(...) %.
**Current evidence:** [`docs/phase-2/cardano-realism-audit.md` §"Mempool
  sizing"](../cardano-realism-audit.md), `.planning/realism-tests/
  revalidation-cost/results.md`.
**Decision:** DISCLOSED — testing the actual revalidation cost
  curve at deployed-mainnet scale requires a separate testbed; risk
  bounded by the suite's mempool sizes (2× one bearer block).
**Disclosure framing (for CIP):**
> The simulator's mempool revalidation cost is modelled as a constant
> per-eviction overhead, not as a function of mempool size; at the
> 2×-one-bearer-block sizing used in these suites this bounds the
> approximation error, but deployed mainnet sizes may differ. See
> RRR-07 in the realism-risks register for the full discussion.
```

**Why this works:** The CIP author lifts the framing paragraph
verbatim; the register entry stays alive for follow-on readers; the
status field is the single mutable bit if the test is later re-run.
This pattern matches both the Leios ImpactAnalysis.md tagging style
(RSK-/EXP- prefixes) and the IETF RFC 3552 security-considerations
"explain attacks out of scope, justify why" framing.

### Pattern 2: Claim-evidence matrix as a flat table

**What:** `docs/phase-2/coverage-check.md` is one Markdown table:
CLM-NN × (claim text, menu option, suite + job + cell, status, risks).
No nested headings; every claim is one row.

**When:** Always — the flat shape is what makes the coverage check
auditable. A CIP reader can scan the table and ask "is every claim a
green row?" without navigating.

**Schema example:**

| ID | Menu option | Claim | Backing suite | Backing job | Cell | Backing seeds | Risks |
|----|-------------|-------|---------------|-------------|------|---------------|-------|
| CLM-01 | priority-only-RB-reserved | Welfare ≥ X% above baseline under sundaeswap_moderate | `phase-2-priority-only-rb-reserved.yaml` | `rb_reserved_x16` | median across 3 seeds | 1,2,3 | RRR-04 |
| CLM-02 | both-dynamic-partitioned | Inclusion latency for urgency-high ≤ Y blocks | `phase-2-two-lane-both-dynamic.yaml` | `partitioned_x4` | per-component | 1 | RRR-03, RRR-08 |
| CLM-03 | both-dynamic-un-partitioned | (claim) | (no backing job yet) | — | — | — | gap → tracked as RRR-12 |

**Why this works:** Gaps surface as empty cells, immediately visible.
Compare to the embedded-narrative pattern of CIP-0164 (which has a
"Simulation Results" subsection with figures referenced inline by
prose): the flat-table pattern is mechanically grep-able and forces
each claim to either name a job or admit a gap. NeurIPS-style
reproducibility checklists (cite specific experiments per claim) use
the same shape.

### Pattern 3: Test design + results in adjacent files

**What:** Each `.planning/realism-tests/<name>/` subfolder has at most
three files:
- `README.md` — the test design, the question it answers, the
  threshold for MITIGATED-vs-DISCLOSED, the simulator-suite YAML
  fragment used.
- `results.md` — the run output (small table or 1-line headline +
  table) and the verdict line.
- `jobs.yaml` — the suite fragment (if it needs a new YAML or YAML
  set); omit if the test re-uses an existing suite.

**When:** For every LIVE-risk targeted test. The design + results
split is what lets the verdict be re-checked later by re-running
`experiment-suite run jobs.yaml` and comparing.

**Why this works:** Matches the prototype pool-number-sensitivity
pattern named in PROJECT.md (33-job smoke × {100 pools, 150 pools} ×
5 demand profiles, Δ% threshold). Three small files are cheaper to
maintain than one large per-test document; the split also matches
SIMPROV / RO-Crate provenance-tracking precedent (input artefact +
output artefact + verdict).

### Pattern 4: Status fields as the only mutable surface

**What:** RRR-NN entries have a `Status` field (LIVE / DORMANT /
MITIGATED / DISCLOSED). CLM-NN rows have a `Status` column (BACKED /
PARTIAL / GAP). These fields are the *only* things that change on
re-runs; the entry text otherwise stays.

**When:** Always — the rest of the entry is the design contract; the
status is the empirical fact.

**Why this works:** Lets future re-runs of cheap tests update the
single-cell status without rewriting the register. A CIP author
reading the register can sort by status to see what's still LIVE
versus what's been resolved. Mirrors the GSN "claim-strategy-solution"
pattern where claims are stable and solutions provide updating
evidence.

## Anti-patterns to avoid

### Anti-pattern 1: Evidence scattered across multiple Markdown files without a register

**What:** Limitations mentioned in passing in
`cardano-realism-audit.md`, in `validity-threats.md`, in the spike
READMEs, and in `CONCERNS.md`. No single document is the source of
truth.

**Why bad:** This is approximately the current state, and the cost of
the milestone is exactly the cost of pulling these scattered mentions
into a register. The CIP author can't write "see the realism-risks
register" if the register doesn't exist; they end up either
hand-curating limitations from 7 source documents (error-prone) or
omitting them (loss of epistemic justification).

**Instead:** One register. Existing documents stay (refresh, don't
delete) but point into the register, not the other way around.

### Anti-pattern 2: Evidence map as a narrative paragraph

**What:** A prose section in the CIP that says "we tested the
mechanism across multiple suites; figures 9-22 show the results." No
explicit per-claim → per-experiment mapping.

**Why bad:** This is the CIP-0164 / EIP-1559-paper pattern, and it's
adequate when the evidence is one big "we ran the protocol and it
worked" story. It is NOT adequate when the evidence is a menu of
options where each option has multiple trade-off claims that need
independent backing. A reader cannot independently audit "claim X is
true because of result Y" without a mechanical mapping.

**Instead:** Flat coverage-check table; CIP author cites it by ID.

### Anti-pattern 3: Burying cheap-test artefacts inside the simulator's `output/` directory

**What:** Run the pool-number-sensitivity test, point at
`sim-rs/output/20260515-...../` as the evidence, never write a
`.planning/realism-tests/pool-number-sensitivity/results.md`.

**Why bad:** `sim-rs/output/` is uncommitted (the manifest lives in
the run-id-keyed directory, which is large and ephemeral). Months
later the run-id-named directory is gone or rotated. Future readers
cannot reproduce or audit the verdict.

**Instead:** `.planning/realism-tests/<name>/results.md` records the
small distilled output (headline numbers + verdict line). The full
`sim-rs/output/<run-id>/` is the regenerable artefact; the register
entry plus the small results file is the durable evidence.

### Anti-pattern 4: Mixing register entries with verdict prose

**What:** Each RRR-NN entry is a free-form Markdown paragraph mixing
status, evidence, verdict, and disclosure framing into one block of
text.

**Why bad:** Loses the audit shape. The verdict is the empirical
output; the disclosure framing is the publication contract; they
shouldn't merge. Sorting by status becomes "skim the prose."

**Instead:** Fixed schema (see Pattern 1) with named subsections —
Status, Category, Impact if real, Current evidence, Decision,
Disclosure framing. Even if every subsection is one sentence, the
structure carries.

### Anti-pattern 5: Re-numbering on every edit

**What:** RRR entries renumbered as the register is reordered;
CLM-NN identifiers shift when claims are added or removed.

**Why bad:** Cross-document references break silently. The CIP draft
cites RRR-07; six months later RRR-07 is something else.

**Instead:** IDs are append-only. If RRR-04 is invalidated or merged,
mark it `Status: SUPERSEDED, see RRR-19`; do not renumber. This is
RFC tradition (RFC numbers are stable forever; obsoleted RFCs stay
in the index with their original number).

## Scalability considerations

The artefact set is small and the failure modes are not
data-scaling-driven but discipline-driven. The numbers below assume
the milestone-projected scope (10-20 RRR entries, 10-20 CLM entries,
3-8 cheap tests).

| Concern | At 10 entries | At 50 entries | At 200 entries |
|---------|---------------|---------------|----------------|
| Register navigability | 1 file, scrollable | Add `# Index` block at top sorted by status | Split by category into `register/topology.md`, `register/controller.md`, etc.; central index file links out |
| Coverage check navigability | 1 table | 1 table with status filter | Split by menu option into `coverage-by-option/*.md`; central index summarises green/yellow/red counts |
| Cross-document cite freshness | Manual review | Manual review + scripted grep | Generated cross-ref index (cheap Rust or shell script) |
| Test results staleness | Re-run on demand | Re-run on demand + manifest of "last run-id" per RRR | Auto-regenerate via `experiment-suite run` driven by a top-level CI runner |

For this milestone, "at 10 entries" is the operating regime. Splitting
the register or coverage check by category is **not** an early
optimisation worth taking; the single-file shape stays grep-able and
diffable.

## Sources

**Precedent — Cardano CIPs and CPS:**
- [CIP-9999 / CPS template](https://github.com/cardano-foundation/CIPs/blob/master/CIP-9999/README.md) — CPS-0023's parent template. Confirms that CIPs responding to a CPS engage the CPS's Open Questions in the CIP's Rationale section. No dedicated "Limitations" or "Evaluation Criteria" section in the template; limitations distributed contextually.
- [CIP-0164 — Ouroboros Linear Leios](https://github.com/cardano-scaling/CIPs/blob/leios/CIP-0164/README.md) — closest in-tradition precedent. Uses an "Evidence" subsection within Rationale, "Trade-offs & Limitations" subsection, figures numbered (Figure 9-22) and tables labelled (Table 5-6). Crucially: **no dedicated "validity threats" section, limitations distributed contextually** — this is the gap the proposed register pattern fixes. References external artefacts (Agda spec, ImpactAnalysis.md, BLS spec) but no consolidated manifest.
- [Leios ImpactAnalysis.md](https://github.com/input-output-hk/ouroboros-leios/blob/main/docs/ImpactAnalysis.md) — closest precedent for ID-prefixed structured tags (REQ-, NEW-, UPD-, RSK-, ATK-, EXP-). RSK entries are inline-labelled prose, not table rows. EXP-to-RSK linking is by narrative context, not explicit cross-reference. **No consolidated index** — this is the gap the proposed `realism-risks-register.md` fixes (single file, fixed schema, grep-able).

**Precedent — IETF / RFC tradition:**
- [RFC 3552 / BCP 72 — Security Considerations Guidelines](https://datatracker.ietf.org/doc/rfc3552/) — every RFC must have a Security Considerations section; should "explain what attacks are out of scope and what countermeasures can be applied." Justifies the "DISCLOSED" status: out-of-scope must be named, not omitted.

**Precedent — Academic / empirical software engineering:**
- [Mitigating Threats to Validity — Mustafa & Labiche, 2019](https://ieeexplore.ieee.org/document/8754344/) — traceability case study showing claim-to-mitigation matrix. Four-validity-type framework (conclusion / internal / construct / external) maps approximately to RRR categories.
- [Threats to Validity — hypocritical paper section? Verdecchia et al., ESEM 2024](https://robertoverdecchia.github.io/papers/ESEM_2024.pdf) — explicit critique of the "embed limitations in prose" pattern; finds threats sections are "seldom discussed in depth, mostly enforced afterthought." Justifies the register-as-first-class-artefact pattern over the "limitations in a paragraph" pattern.
- [NeurIPS Paper Checklist](https://neurips.cc/public/guides/PaperChecklist) — claim-evidence matching standard ("claims should match theoretical and experimental results"). Reproducibility appendix pattern.

**Precedent — assurance cases (CAE / GSN):**
- [CAE Framework — Claims-Arguments-Evidence](https://claimsargumentsevidence.org/resources/downloadable-resources/) — the formal pattern that the proposed RRR / CLM scheme approximates. Claims (CLM-NN) + Evidence (jobs/run summaries) + Argument (RRR-NN risks affecting the claim) = the three-leaf structure.
- [NASA assurance-case educational presentation](https://shemesh.larc.nasa.gov/arg/uac-module1-foundation.pdf) — confirms the claim/argument/evidence triple is the standard unit for structured assurance.

**Precedent — comparable simulators:**
- [Empirical Analysis of EIP-1559 — Liu et al., ACM CCS 2022](https://dl.acm.org/doi/10.1145/3548606.3559341) — peer-reviewed EIP-1559 analysis paper; uses prose-narrative evidence pattern (no register). Demonstrates the "as long as the evidence is one big story, narrative is fine" limit — Leios phase-2's menu-of-options shape requires the matrix.
- [SIMPROV — provenance for simulation studies, 2024](https://pmc.ncbi.nlm.nih.gov/articles/PMC12237051/) — argues for explicit input + output + verdict provenance triples in simulation work; justifies the three-file `.planning/realism-tests/<name>/` shape.
- [Provenance Run Crate / RO-Crate](https://journals.plos.org/plosone/article?id=10.1371%2Fjournal.pone.0309210) — standard for workflow-run provenance; the proposed `results.md` + `jobs.yaml` pair is a lightweight Markdown analogue.

**Internal — existing project artefacts surveyed:**
- [`docs/phase-2/CPS-0023/README.md`](../../docs/phase-2/CPS-0023/README.md) — the CPS being responded to.
- [`docs/phase-2/cardano-realism-audit.md`](../../docs/phase-2/cardano-realism-audit.md) — current category-by-category audit; refresh in Phase 4.
- [`docs/phase-2/validity-threats.md`](../../docs/phase-2/validity-threats.md) — current per-suite trust matrix; refresh in Phase 4.
- [`.planning/REVIEW.md`](../REVIEW.md) — Fix Status table (closest existing analog of an RRR scheme).
- [`.planning/codebase/ARCHITECTURE.md`](../codebase/ARCHITECTURE.md) — codebase architecture (out-of-scope for this document; cited for what the milestone does NOT need to re-research).
