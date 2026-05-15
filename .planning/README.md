# `.planning/` — GSD workspace for the `dynamic-experiment` branch

This directory is the [Get Shit Done](https://github.com/wgould/gsd)
working memory for the phase-2 dynamic-pricing rebuild. It holds
plans, spikes, code-review records, investigation notes, and
mechanism-decision memos that informed the publication-ready state
of the branch. The canonical user-facing simulator docs live in
[`../docs/phase-2/`](../docs/phase-2/) and in
[`../CLAUDE.md`](../CLAUDE.md); this directory is the audit trail
behind those docs.

If you are reading one file in `.planning/`, read
[`family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md).

## Top-level docs

### Mechanism decision (authoritative)

- [`family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md) —
  the publication-committed mechanism: chain-derived,
  EIP-1559-faithful, one controller step per canonical block.
  Includes ready-to-paste publication framing, the audit-trail of
  every artifact that informed the decision, and a follow-on-work
  list.

### Code review (`REVIEW.md`)

- [`REVIEW.md`](REVIEW.md) — 16 findings across the rebuild (1 Critical,
  7 Warning, 8 Info). 14 are RESOLVED or APPLIED; 2 (WR-2
  gate-reject info loss, WR-7 ActorComponent reallocation refactor)
  are explicitly deferred. WR-1 (controller contamination on
  slot-battle reorg) was RESOLVED 2026-05-14 via the chain-derived
  refactor. The Fix Status table at the top of `REVIEW.md` is the
  per-finding ground truth.

### Audit / investigation chain (chronological)

These document the empirical and conceptual path from the M5-era
single-producer single-lane baseline to the publication-grade
multi-producer chain-derived Family B mechanism:

- [`realism-fix-PLAN.md`](realism-fix-PLAN.md) — pivot from
  `topology-single-producer.yaml` to `topology-realistic-100.yaml`;
  prerequisites for the 100-node mainnet-derived topology adoption
  (paired with spike 006).
- [`smoke-comparison-2026-05-14.md`](smoke-comparison-2026-05-14.md) —
  initial 33-job smoke after the realism pivot; surfaced the 41
  observed slot battles that reclassified WR-1 from *dormant* to
  *LIVE*.
- [`chain-derived-controller-PLAN.md`](chain-derived-controller-PLAN.md) —
  the implementation deltas for spike 007's ADOPT verdict
  (chain-derived refactor).
- [`chain-derived-bug-investigation.md`](chain-derived-bug-investigation.md) —
  bug #1: `current_chain_tip_quote` one-step lag, identified and
  fixed during post-refactor revalidation.
- [`chain-derived-fix-revalidation-2026-05-14.md`](chain-derived-fix-revalidation-2026-05-14.md) —
  bug #1 fix revalidation.
- [`smoke-comparison-chain-derived-vs-accumulator-2026-05-14.md`](smoke-comparison-chain-derived-vs-accumulator-2026-05-14.md) —
  side-by-side comparison surfacing the residual divergence that
  triggered bug #2 investigation.
- [`chain-derived-bug2-investigation.md`](chain-derived-bug2-investigation.md) —
  "bug #2" reframed: the divergence was not a bug but a mechanism
  cadence difference (accumulator's effective 2-step-per-RB-EB
  pair vs chain-derived's 1-step-per-canonical-block).
- [`mechanism-welfare-impact-2026-05-14.md`](mechanism-welfare-impact-2026-05-14.md) —
  the 33-job welfare-impact characterisation of the cadence
  difference; supporting evidence for the Family B commitment.
- [`family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md) —
  the closing decision memo (see above).

## Sub-directories

- [`spikes/`](spikes/) — seven numbered audit spikes plus
  [`MANIFEST.md`](spikes/MANIFEST.md). Each spike has a README under
  its numbered subdirectory. Spike 005 (validity threats) points at
  [`../docs/phase-2/validity-threats.md`](../docs/phase-2/validity-threats.md)
  for its canonical content; the other six are self-contained under
  their subdirectories.
- [`codebase/`](codebase/) — codebase-wide analysis documents
  (ARCHITECTURE, CONCERNS, CONVENTIONS, INTEGRATIONS, STACK,
  STRUCTURE, TESTING). Read [`codebase/CONCERNS.md`](codebase/CONCERNS.md)
  for the inventory of fragilities and tech-debt items, including
  per-finding 2026-05-14 resolution annotations.

## How to navigate, by reader persona

- **Paper reviewer asking "what mechanism are we publishing?"**:
  read [`family-b-decision-2026-05-14.md`](family-b-decision-2026-05-14.md)
  start-to-finish, then skim
  [`mechanism-welfare-impact-2026-05-14.md`](mechanism-welfare-impact-2026-05-14.md)
  for the empirical backing.
- **New team member orienting**: read [`../CLAUDE.md`](../CLAUDE.md)
  first (project context + mechanism overview), then this README,
  then [`REVIEW.md`](REVIEW.md) Fix Status table to know what's
  live, applied, deferred, or resolved.
- **Auditor verifying a specific finding**: [`REVIEW.md`](REVIEW.md)
  Fix Status table → drill into the per-finding sections → follow
  cross-links into `spikes/`, `codebase/`, or the
  investigation-chain memos above.
- **Reproducer regenerating suite results**: read
  [`../CLAUDE.md`](../CLAUDE.md) §"Running the suites" first; this
  directory is then optional context.

## Convention

Files at the top level of `.planning/` are dated when their content
is time-anchored (e.g. `family-b-decision-2026-05-14.md`,
`mechanism-welfare-impact-2026-05-14.md`). PLAN.md files are the
forward-looking working artefacts. Spike READMEs follow the
Date / Verdict / Spike Question header used in
[`spikes/MANIFEST.md`](spikes/MANIFEST.md). Investigation notes
(`*-investigation.md`) are evidence dumps with timestamps and
git-revision pointers in their preambles.
