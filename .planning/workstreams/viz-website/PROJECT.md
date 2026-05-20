---
workstream: viz-website
parent_milestone: v1.0 — Phase-2 CIP Evidence Audit
created: 2026-05-20
---

# Visualisation Website for Experiment Output Metrics

## What This Is

A small visualisation surface for the experiment artefacts produced under [`sim-rs/output/`](../../../sim-rs/output/). The phase-2 simulator already emits per-(job, seed) CSVs, suite-level comparison tables, time-series, paired-bootstrap results, and the standalone `tiered_plot.html`. This workstream gives those outputs a navigable home so suite results can be browsed and compared without crawling output directories or eyeballing CSV files.

Standalone workstream — runs alongside the phase-2 CIP evidence milestone but is not part of its delivery. The CIP evidence work is the source of truth; this is a reader for it.

## Core Value

A user (the simulator developer in the immediate term; potentially the CIP reader in the longer term) can open the viz site locally, pick a suite run, and see the headline metrics, time-series, and comparison tables without manually opening CSV files or assembling plots in a notebook.

## Scope Boundary

In scope:
- Reading the artefacts that the simulator already writes (CSV, JSON, manifest.json) under [`sim-rs/output/`](../../../sim-rs/output/).
- Presenting them via charts and tables in a browser.
- Local-first: the site should run on the dev machine; deployment is optional.

Out of scope:
- Generating new metrics or changing the simulator's output schema.
- Anything that requires re-running suites from inside the website.
- Authentication, multi-user concerns, server-side state.

## Open Decisions

Deliberately left for [`/gsd-discuss-phase`](../../phases/) to surface:
- Static (committed HTML) vs build-script vs local dev server vs off-the-shelf framework.
- Tech stack and chart library.
- Repo location (sim-rs/viz/? top-level viz/? docs/?).
- Which metric outputs are highest priority first.
- Styling / theming.

## Context

The phase-2 simulator emits:
- `sim-rs/output/<root>/<suite-run>/manifest.json` per suite invocation
- Per-(job, seed) directories with `time_series.csv`, paired-bootstrap CSVs, comparison CSVs
- Suite-level aggregates (e.g. `priority_only_fast_path_overall_comparison.csv`)
- A one-off `tiered_plot.html` from earlier exploration

Roughly 100+ suite runs exist under [`sim-rs/output/phase-2/`](../../../sim-rs/output/phase-2/) at workstream creation time.

## Constraints

- The viz site reads files; it does not write or modify any experiment artefact.
- Local-first: must work without internet / external services for the simulator developer.
- Determinism scope of the simulator is not affected — this workstream touches no `sim-core` or `sim-cli` source files unless explicitly scoped in a later phase.
