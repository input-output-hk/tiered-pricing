# CIP Evidence Package — Phase-2 Dynamic-Pricing

This directory is the **single source of truth** for the Cardano Improvement Proposal (CIP) responding to CPS-0023 ("Urgency Signaling"). The CIP itself is written separately and pastes from this evidence package.

## Where to start

→ **[cip-author-summary.md](cip-author-summary.md)** — the paste guide. Read this first.

The paste guide maps each CIP section (Methodology, Calibration, Trust matrix, Evidence, Limitations) to its source artefact in this directory and the specific paragraphs / rows to copy.

## Directory layout

```
cip-evidence/
├── README.md                                # You are here
├── cip-author-summary.md                    # Paste guide — start here
│
├── audit-documents/                         # The 6 CIP-cited source-of-truth artefacts
│   ├── cardano-realism-audit.md             #   Calibration source: 17 (value, source, date-retrieved) triples
│   ├── validity-threats.md                  #   Trust matrix: 19 per-suite blocks
│   ├── realism-risks-register.md            #   Limitations: 24 DISCLOSED RSK-NN entries
│   ├── coverage-check.md                    #   Evidence: 55 CLM-NN rows (claim → backing simulator job)
│   ├── methodology-overview.md              #   Methodology: ODD index + worked example
│   └── latency-by-urgency.md                #   Operational user-experience axis: latency × urgency × mechanism
│
├── test-results/                            # Phase-3 empirical evidence
│   ├── multi-seed-variance/                 #   TEST-03 + TEST-04 N=20 BCa Confidence Intervals
│   ├── multiplier-floor-16-companion/       #   TEST-07a regime-dependence finding
│   ├── hash-diversity-gate/                 #   COV-05 17/17 BACKED-eligible pass
│   ├── pool-number-sensitivity/             #   TEST-05 (partial coverage; disclose-only)
│   └── run-length-steady-state/             #   TEST-06 (partial coverage; disclose-only)
│
└── consistency-audit/                       # HAND-02 reproducible audit
    ├── verify-consistency.sh                #   Four-check verifier (run from any cwd)
    └── CONSISTENCY-REPORT.md                #   Phase-5-close audit log; OVERALL: PASS
```

## What the milestone delivered

A reader of the CIP can:

1. **Verify each menu-option claim** against a specific `(suite, job, seed)` tuple via [`audit-documents/coverage-check.md`](audit-documents/coverage-check.md).
2. **Inspect what is and isn't faithfully modelled** via [`audit-documents/realism-risks-register.md`](audit-documents/realism-risks-register.md) — 24 DISCLOSED disclosure-paragraphs, all CIP-pasteable.
3. **Read the trust matrix** at [`audit-documents/validity-threats.md`](audit-documents/validity-threats.md) — 19 per-suite blocks at 2 HIGH + 13 MEDIUM + 4 LOW + 0 UNRESOLVED aggregate trust.
4. **Cite calibration values** via [`audit-documents/cardano-realism-audit.md`](audit-documents/cardano-realism-audit.md)'s 17 `(value, source, date-retrieved)` triples.
5. **Reference methodology** via [`audit-documents/methodology-overview.md`](audit-documents/methodology-overview.md) (Overview, Design concepts, Details — ODD — protocol index with a worked example).
6. **See latency-by-urgency trade-offs** at [`audit-documents/latency-by-urgency.md`](audit-documents/latency-by-urgency.md) — per-mechanism observed inclusion latency and inclusion rate across 11 urgency-tagged user classes at N=20 seeds. Surfaces the user-experience axis the welfare-only findings don't.
7. **Re-run the consistency audit** independently: `bash cip-evidence/consistency-audit/verify-consistency.sh` — exit 0 on PASS.

## Headline empirical findings

From Phase-3 evidence at N=20 seeds with Bias-corrected and accelerated (BCa) 95% Confidence Intervals (per [`test-results/multi-seed-variance/results.md`](test-results/multi-seed-variance/results.md)):

- **Un-reserved two-lane mechanisms outperform single-lane Ethereum Improvement Proposal 1559 (EIP-1559)** at `multiplier_floor = 4` under `sundaeswap_moderate` demand. CIs strictly positive; sign-coherence 0.90.
- **Ranking-block-reserved (RB-reserved) two-lane mechanisms underperform single-lane EIP-1559** under the same calibration. This refutes the pre-Phase-3 single-seed framing that "two-lane mechanisms outperform single-lane EIP-1559" generally — that ordering holds only for the un-reserved variants.
- **The `multiplier_floor = 4` calibration is regime-dependent** at `multiplier_floor = 16` (per [`test-results/multiplier-floor-16-companion/results.md`](test-results/multiplier-floor-16-companion/results.md)): the rb-scarcity finding inverts and the urgency-inversion finding weakly reverses.

## Reproducing the audit

The consistency-verification script is intentionally pure POSIX shell (`bash + grep + awk + sed`; no `yq` dependency):

```bash
bash cip-evidence/consistency-audit/verify-consistency.sh
```

Exit code 0 means OVERALL: PASS across all four checks:

1. **RSK-NN / CLM-NN / EXP-NN dead-reference scan** — every identifier reference resolves to a canonical definition site.
2. **`backing-job` path resolution** — every coverage-check `backing-job` cell resolves to a `(suite, job)` entry in `sim-rs/parameters/phase-2-sweep/suites/`.
3. **`golden-sha256` cross-check** — every truncated hash in coverage-check.md matches the corresponding `.goldens/<suite>.sha256` file.
4. **Markdown link + backtick-path resolution** — every link in the six in-scope documents resolves on disk.

If a future edit to any of the six in-scope documents introduces a dead reference, the script exits 1 and points at the offending file + identifier. The script is the audit trail; the per-run snapshot is [`consistency-audit/CONSISTENCY-REPORT.md`](consistency-audit/CONSISTENCY-REPORT.md).

## Citable reference

The annotated git tag **`phase-2-cip-evidence-v1`** is the citable reference the CIP quotes. See [`cip-author-summary.md`](cip-author-summary.md) §"Pinned references" for the embedded tag-message draft and the user-applied tag recipe.

## What is NOT in this directory

These items are elsewhere in the repo or out of scope; see [`cip-author-summary.md`](cip-author-summary.md) §"What is NOT in this evidence base" for the full list:

- **Simulator source code** — `sim-rs/sim-core/`, `sim-rs/sim-cli/`. The CIP cites the simulator by repo Uniform Resource Locator (URL) and the citable tag.
- **Simulator parameters** — `sim-rs/parameters/phase-2-sweep/suites/`. The coverage-check references these YAML files; they live with the simulator.
- **GSD planning artefacts** — `.planning/` (PROJECT.md, ROADMAP.md, phase-by-phase PLAN.md / SUMMARY.md / VERIFICATION.md records). These are the audit trail of how the evidence was produced, not the evidence itself.
- **Upstream Leios documents** — the `ImpactAnalysis.md` precedent cited for the RSK-NN / CLM-NN identifier conventions lives in `input-output-hk/ouroboros-leios`, not in this repo.
- **The CIP draft itself** — written separately by the CIP author from this paste guide.
