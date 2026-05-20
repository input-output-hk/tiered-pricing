# phase-2 simulator visualisation site

A local-first visualisation surface for the experiment artefacts produced
under `sim-rs/output/`. The build script is a Python standard-library
program that walks `sim-rs/output/` for `manifest.json` files, parses each
suite's per-(job, seed) `run_summary.json` and `time_series.csv`, and
emits a static HyperText Markup Language (HTML) + JavaScript bundle that
renders the headline metrics and time-series in the browser. The result is
a Single-Page Application (SPA) with hash-based routing; charts use
[Observable Plot](https://observablehq.com/plot) (vendored under
`static/plot.min.js`, with its [D3](https://d3js.org/) peer dependency
vendored at `static/d3.min.js`).

The site is intended as an internal developer tool: minimal styling, no
design system, no dark mode, no public hosting. The audience is the
simulator dev. See [`PROJECT.md`](../../../.planning/workstreams/viz-website/PROJECT.md)
for the workstream scope; see [`REQUIREMENTS.md`](../../../.planning/workstreams/viz-website/REQUIREMENTS.md)
for the six VIZ-NN acceptance criteria the bundle satisfies.

## Quickstart

```
python sim-rs/scripts/viz/build.py --serve
```

This builds the bundle into `sim-rs/output/viz/` and then serves it on
`http://127.0.0.1:8000/`. Press Ctrl-C to stop. Re-run the same command
whenever new suites land under `sim-rs/output/` — the build is idempotent
and overwrites the previous bundle in place.

For a build-only invocation (no server), drop `--serve`. The script then
prints a `python -m http.server --bind 127.0.0.1 --directory <output>
8000` instruction to stderr so inspecting a previous build is one
copy-paste away.

## Flags

All paths default to repo-root-relative values; run the script from the
repository root (`/home/will/git/arc-tiered-pricing/` or equivalent) for
the defaults to resolve correctly.

| Flag        | Type            | Default              | Description |
|-------------|-----------------|----------------------|-------------|
| `--source`  | path            | `sim-rs/output`      | Root directory to walk for `manifest.json` files. |
| `--output`  | path            | `sim-rs/output/viz`  | Build output directory. Recreated on each run (existing files are overwritten in place). |
| `--include` | glob (repeatable) | — (include all)    | Glob matched against the suite path relative to `--source`. Repeat to layer multiple includes. Example: `--include 'phase-2/*'`. |
| `--exclude` | glob (repeatable) | — (exclude none)   | Glob matched against the suite path relative to `--source`. Wins over `--include` when both match. |
| `--serve`   | flag            | off                  | After build, serve `--output` over a local HTTP server bound to `127.0.0.1`. |
| `--port`    | int             | `8000`               | Port for `--serve`. |

The flag set matches `python sim-rs/scripts/viz/build.py --help`
verbatim; the table is a literal projection of the argparse declarations
so the doc cannot drift from the Command-Line Interface (CLI).

## Output layout

```
sim-rs/output/viz/
├── index.html                              SPA shell (served at /)
├── static/
│   ├── main.js                             Hash router + view renderers
│   ├── style.css                           Minimal developer-tool CSS
│   ├── plot.min.js                         Observable Plot 0.6.17 (vendored Universal Module Definition (UMD) bundle)
│   ├── d3.min.js                           D3 7.9.0 (vendored — Observable Plot's UMD form externalizes D3)
│   └── PLOT_VERSION.txt                    Pinned versions + retrieval date for both vendored bundles
└── data/
    ├── index.json                          Tier 1: list of every suite + suite-level metadata
    ├── <suite_id>.json                     Tier 2: per-suite manifest summary + (job, seed) table + cross-seed groupings
    └── <suite_id>/
        └── <job>-<seed>.json               Tier 3: per-(job, seed) headline metrics + long-form time-series
```

The three data tiers map directly onto the three browser views:

- **Tier 1 (`index.json`)** powers the home page (VIZ-01). Fetched on page
  load; bounded size regardless of how many suites are under `sim-rs/output/`.
- **Tier 2 (`<suite_id>.json`)** powers the suite drill-down (VIZ-02,
  VIZ-05). Fetched on suite click; cached in memory across navigations.
- **Tier 3 (`<suite_id>/<job>-<seed>.json`)** powers the per-(job, seed)
  detail (VIZ-03, VIZ-04). Fetched on demand only when the user opens that
  view, so deep drill-downs do not penalise the initial load.

The `<suite_id>` is derived from `manifest_path.parent.relative_to(--source)`
with `/` replaced by `__` so two suites named identically at different
paths (e.g. `phase-2/eip1559-robustness` and
`phase-2/eip1559-robustness-20260514-160045`) round-trip to distinct
identifiers and do not collide.

## What gets rendered

- **Home (`#/`):** sortable suite list (default sort: most-recent
  `started-at-utc` first). Each row links to the suite drill-down.
- **Suite drill-down (`#/suite/<suite_id>`):** manifest summary as a
  definition list, sortable (job, seed) table with headline-metric
  columns, and an in-suite cross-seed time-series overlay where the user
  picks a job + lane and sees one coloured line per seed.
- **Per-(job, seed) (`#/job/<suite_id>/<job>/<seed>`):** six-card
  headline-metric strip + per-component latency table + three Observable
  Plot panes (controller quote per lane, mempool bytes per lane, fees +
  refunds per slot). When the per-(job, seed) JSON has no
  `time_series`, a `(no time-series available)` placeholder renders in
  place of the charts so phase-3 / non-time-series suites do not crash
  the view.
- **Latency is reported per demand component**, not per lane — see Notes
  below for why this distinction matters.

## Notes

- **`sim-rs/output/viz/` is gitignored.** The exclusion is transitive:
  `sim-rs/.gitignore` line 2 (`/output`) catches everything under
  `sim-rs/output/`, including `viz/`. **No new `.gitignore` entry was
  added for this phase** — the existing rule is sufficient. Don't add
  `output/viz/`, `viz/`, or `/output/viz/` to either gitignore; the parent
  rule already handles it.
- **Stdlib-only.** Python's standard library is sufficient: the script
  uses `argparse`, `csv`, `fnmatch`, `functools`, `http.server`, `json`,
  `pathlib`, `shutil`, and `sys` and nothing else. There is no
  `requirements.txt`, no virtualenv, no `pip install`. PyYAML is NOT
  used by this script (matches CONTEXT.md decision D-08).
- **Observable Plot is vendored** at `static/plot.min.js` (~209 KB,
  version 0.6.17 as of 2026-05-20). The version + retrieval date is
  recorded in `static/PLOT_VERSION.txt`. Refresh annually with a one-line
  pull request; jsDelivr's `@0.6` major.minor tag auto-tracks patch
  releases. The bundle works fully offline.
- **D3 is vendored as a peer dependency** at `static/d3.min.js` (~280 KB,
  version 7.9.0). Observable Plot's UMD bundle externalizes D3 — its
  Immediately-Invoked Function Expression (IIFE) reads `globalThis.d3` at
  module init, so `static/d3.min.js` MUST be loaded before
  `static/plot.min.js` in `index.html` or Plot's first chart call fails
  with `Cannot read properties of undefined (reading 'timeSecond')` /
  `Plot.ruleY is not a function`. The script-tag order is locked at the
  top of `static/index.html`; do not reorder it. `PLOT_VERSION.txt`
  records the D3 pin alongside the Plot pin so the annual refresh covers
  both.
- **`metrics_comparison.txt` is human-only.** The build never parses it
  — every field it contains is also in `run_summary.json` (in
  structured form). The per-suite "aggregates" section is reserved for a
  future enhancement; phase-2 suites today emit no suite-level
  `*.csv`, so the section is omitted entirely from the suite view (an
  HTML comment in the rendered Document Object Model (DOM) carries the
  rationale for any future reader who opens DevTools).
- **`priority_only_fast_path_overall_comparison.csv` is historical
  pre-phase-2 work.** The filename appears in early VIZ-05 framing but
  lives under `sim-rs/output/analysis/`, not under any phase-2 suite
  root. Current phase-2 suites do not emit a CSV with this name (or any
  suite-level `*.csv`); the VIZ-05 deliverable is satisfied by the
  in-suite cross-seed time-series overlay instead.
- **Latency label.** The simulator's
  `ComponentSummary.latency_blocks_observations` is a per-component list
  mixing priority and standard inclusions; there is no per-lane
  breakdown. The UI displays "Latency by demand component (blocks)" and
  shows each component's priority / standard inclusion counts in the
  same row so the user can see which lane dominates. The
  per-lane-latency wording forbidden by RESEARCH.md Pitfall 5 is
  grep-gated out of `static/main.js` — the data simply does not support
  that mis-attribution.
- **Local-only network bind.** The server binds `127.0.0.1` exclusively,
  never `0.0.0.0`. Other devices on the Local Area Network (LAN) cannot
  reach the site. This is enforced as the literal first element of the
  `ThreadingHTTPServer` address tuple; the alternative bind forms
  (`""`, `None`, `"0.0.0.0"`) are absent from the file.
- **Browser DOM injection uses `textContent`, never `innerHTML`.** Suite
  names, job names, and seed strings are user-controlled (they come from
  the simulator's input YAMLs) and could contain `<` if a developer
  picks an unfortunate name. `textContent` defangs this without effort.

## Tests

```
python -m unittest discover -s sim-rs/scripts/viz/tests -t sim-rs/scripts/viz
```

The suite is 18 tests total: 11 ingest tests covering the build's
discovery / JSON-emission / casing / latency-mean / missing-CSV /
no-suite-level-CSV / suite-id-derivation paths against synthetic
fixtures under `tests/fixtures/`, plus 7 HTTP smoke tests that
subprocess-spawn `build.py --serve` on a kernel-chosen free port and
fetch the three URL shapes plus the four vendored static assets. The
full suite runs in under 1 second.

## Updating the vendored Observable Plot bundle

Annual refresh recipe (no version bump usually required; the `@0.6`
jsDelivr tag tracks Plot's 0.6.x line, and D3 7.x is API-stable):

```
curl -sLo sim-rs/scripts/viz/static/plot.min.js \
  https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/dist/plot.umd.min.js

curl -sLo sim-rs/scripts/viz/static/d3.min.js \
  https://cdn.jsdelivr.net/npm/d3@7/dist/d3.min.js
```

Then update `sim-rs/scripts/viz/static/PLOT_VERSION.txt` with both
resolved version strings + the retrieval date, run the test suite
(`python -m unittest discover -s sim-rs/scripts/viz/tests -t
sim-rs/scripts/viz` — the `test_plot_js_vendored_locally` and
`test_d3_js_vendored_locally` smoke tests lock the ≥ 50 KB and ≥ 100 KB
floors respectively), and commit. Do not refresh Plot without refreshing
D3 in the same pull request — Plot's UMD bundle relies on the D3 globals
at the version the Plot release was built against.
