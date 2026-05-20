---
phase: 01-viz-site-mvp
plan: 04
subsystem: serve-entry-point
tags: [python, stdlib, http-server, threading-http-server, security-bind, subprocess-smoke, single-documented-command]

# Dependency graph
requires:
  - "Plan 01-02 ingest module (`run_build`, `parse_args`, `discover_suites`, `load_seed`, `LANE_FIELDS`)"
  - "Plan 01-03 static bundle (`static/index.html`, `static/main.js`, `static/style.css`, `static/plot.min.js`, `static/PLOT_VERSION.txt`)"
provides:
  - "sim-rs/scripts/viz/build.py — expanded with `serve`, `copy_static_assets`, `--serve`, `--port`; `main()` orchestrates run_build → copy_static_assets → optional serve"
  - "sim-rs/scripts/viz/tests/test_serve_smoke.py — 6-method ServeSmokeTest with subprocess + urllib.request HTTP smoke against the mini-suite fixture"
  - "The single documented command from VIZ-06: `python sim-rs/scripts/viz/build.py --serve --source <source> --output <output>` produces a self-contained browseable bundle on 127.0.0.1:<port>"
  - "Locked HTTP smoke surface for Plan 01-05 (browser views) and Plan 01-06 (README + final E2E checkpoint) to extend without re-touching the serve pipeline"
affects:
  - 01-05-PLAN-browser-views
  - 01-06-PLAN-readme-and-checkpoint

# Tech tracking
tech-stack:
  added:
    - "Python stdlib `http.server.ThreadingHTTPServer` + `SimpleHTTPRequestHandler` + `functools.partial` — already approved by D-08; no requirements.txt"
    - "Python stdlib `shutil.copy2` for asset mirroring (preserves mtime → enables future refresh-tracking via PLOT_VERSION.txt)"
  patterns:
    - "Explicit `127.0.0.1` bind in the address tuple's first element (NOT `\"\"`, `None`, `0.0.0.0`) — the only place this landmine appears in the codebase"
    - "`allow_reuse_address = True` set BEFORE `serve_forever` so quick restarts don't trip TIME_WAIT"
    - "`functools.partial(SimpleHTTPRequestHandler, directory=str(output_dir))` anchors the served path explicitly, never inheriting CWD"
    - "Subprocess-based HTTP smoke: kernel-picked free port (`socket.bind((\"127.0.0.1\", 0))`), 5-second poll-with-deadline startup gate that surfaces subprocess stderr on early exit"
    - "Self-contained bundle layout: `index.html` at served root; `static/*.{js,css,txt}` under `<output>/static/` (mirrors the SPA shell's `static/<asset>` relative href paths)"

key-files:
  created:
    - sim-rs/scripts/viz/tests/test_serve_smoke.py
  modified:
    - sim-rs/scripts/viz/build.py

key-decisions:
  - "Bind `127.0.0.1` explicitly (T-01-04-01 mitigation) — grep-gated by the plan's structural acceptance criteria; the literal substring `(\"127.0.0.1\"` appears once in build.py, the strings `(\"0.0.0.0\"`, `(\"\", port)`, `(None, port)` appear zero times"
  - "`allow_reuse_address = True` (T-01-04-02 mitigation) — set on the server instance after construction, before serve_forever, inside the `with` block"
  - "Free-port pick over hardcoded 8000 for the smoke test — `socket.bind((\"127.0.0.1\", 0))` + `getsockname()[1]` avoids collision with parallel test runs and with developer servers already on 8000. Single-process race window is negligible on a single-developer machine"
  - "Subprocess-based smoke test, not in-process — `serve_forever()` blocks the calling thread; the test runs the server in a separate process and curls it over loopback. The 5-second polling deadline with subprocess-poll-on-early-exit surfaces ImportError / bind-failures as real diagnostic stderr rather than a generic timeout"
  - "`index.html` copied to bundle root, not under `static/` — the SPA shell references its assets via `static/<asset>` relative paths, which only resolve correctly when `index.html` itself is served from the bundle root (confirmed in orchestrator pre-flight)"
  - "Build-only mode prints a `python -m http.server --bind 127.0.0.1 --directory <output> 8000` instruction to stderr so inspecting a build without `--serve` is one copy-paste away — matches D-04's 'served via local HTTP server' constraint even when the script isn't holding the port"

patterns-established:
  - "Pattern: SPA-bundle copy mirrors source layout — `copy_static_assets(static_src, output)` iterates `static_src.iterdir()` (flat by Plan 01-03 design), routes `index.html` to bundle root, everything else to `<output>/static/`; soft-fail if `static_src` is missing"
  - "Pattern: Sequential `main()` — `run_build` → `copy_static_assets` → warnings summary → optional `serve` (blocking). No threads, no async (RESEARCH.md Pitfall 7)"
  - "Pattern: subprocess smoke setUp/tearDown with poll-deadline startup + stderr capture on early exit — locks the diagnostic-info-on-failure expectation that VALIDATION.md projects"

requirements-completed: [VIZ-06]

# Metrics
duration: 4min
completed: 2026-05-20
---

# Phase 01 Plan 04: viz-website Wave 3 serve entry-point Summary

**Extends `sim-rs/scripts/viz/build.py` with `--serve` / `--port` flags, `copy_static_assets`, and an explicit `127.0.0.1`-bound `ThreadingHTTPServer`, then locks the entire pipeline behind a 6-method subprocess HTTP smoke test against the mini-suite fixture. Closes VIZ-06's "single documented command" surface and mitigates threats T-01-04-01 (bind landmine) and T-01-04-02 (TIME_WAIT collision).**

## Performance

- **Duration:** ~4 min (12:16:22Z start → 12:20:26Z SUMMARY write; 244 s elapsed)
- **Tasks:** 3 (Task 0 pre-flight already verified by orchestrator; Tasks 1 + 2 executed + committed atomically)
- **Files created:** 1 (`tests/test_serve_smoke.py`)
- **Files modified:** 1 (`build.py`)
- **Test count:** 17 (11 ingest + 6 smoke), all green
- **Full suite runtime:** 0.65 s (well under the 15 s budget noted in VALIDATION.md)

## Resolved CLI surface (`parse_args` flag set)

```
usage: build.py [-h] [--source SOURCE] [--output OUTPUT] [--include INCLUDE]
                [--exclude EXCLUDE] [--serve] [--port PORT]

options:
  -h, --help         show this help message and exit
  --source SOURCE    Root directory to walk for manifest.json files
                     (default: sim-rs/output).
  --output OUTPUT    Output directory for the generated bundle
                     (default: sim-rs/output/viz; gitignored transitively
                     via the existing sim-rs/.gitignore /output rule).
  --include INCLUDE  Glob (matched against the relative-to-source path) to
                     include. May be passed multiple times. Empty = include all.
  --exclude EXCLUDE  Glob (matched against the relative-to-source path) to
                     exclude. May be passed multiple times.
  --serve            After build, serve via http.server on --port (default 8000).
  --port PORT        Port for --serve (default 8000).
```

Plan 01-02 shipped the first four flags; this plan added `--serve` (`action="store_true"`, default `False`) and `--port` (`type=int`, default `8000`). Argparse exposes them as snake_case attributes (`args.serve`, `args.port`) per PATTERNS.md Pattern B.

## `127.0.0.1` grep gates (security; T-01-04-01)

```
$ grep -nE '\("127\.0\.0\.1"' sim-rs/scripts/viz/build.py
736:    with ThreadingHTTPServer(("127.0.0.1", port), handler) as httpd:

$ grep -nE '\("0\.0\.0\.0"' sim-rs/scripts/viz/build.py
  (empty)

$ grep -nE 'HTTPServer\s*\(\s*\(\s*""' sim-rs/scripts/viz/build.py
  (empty)

$ grep -nE 'HTTPServer\s*\(\s*\(\s*None' sim-rs/scripts/viz/build.py
  (empty)
```

The bind tuple's first element is exactly the literal string `"127.0.0.1"`; the four forbidden alternatives (`("0.0.0.0"`, `("", port)`, `(None, port)`, missing-host single-arg form) are all absent. The narrow-pattern check is more precise than the plan's `<verification>`-block broad pattern (`"":\s*$|, *""`), which would have false-positive-matched JSON `.get()` defaults like `rs.get("...", "")` on line 424 — that's pre-existing Plan 01-02 code unrelated to the bind landmine.

`allow_reuse_address = True` is set on the server instance at line 737, immediately after `with ThreadingHTTPServer(...) as httpd:` and before `httpd.serve_forever()` — closing T-01-04-02 (TIME_WAIT collision on quick restarts).

## Smoke test runtime

```
$ time python3 -m unittest discover -s scripts/viz/tests -t scripts/viz
...
Ran 17 tests in 0.651s
OK
python3 -m unittest discover  0.23s user 0.04s system 38% cpu 0.695 total
```

11 ingest tests + 6 smoke tests, all green in 0.65 s. The 6 smoke tests collectively spawn the build subprocess 6× (one per test method via setUp/tearDown), poll for the loopback URL on a kernel-chosen free port, fetch 1-2 URLs, and tear down. The 5-second startup deadline per test never fires; actual startup is ~80-100 ms.

## `build.py` line count

| Stage                       | Lines |
|-----------------------------|------:|
| After Plan 01-02 ingest     | 727   |
| After Plan 01-04 expansion  | **863** |
| Delta                       | +136  |

The +136 lines cover the two new public helpers (`copy_static_assets`, `serve`), the two new argparse entries, the wired-up `main()` body, the `functools` / `shutil` / `http.server` imports, and the inline docstrings explaining each landmine (the bind tuple's first element, `allow_reuse_address`, the `directory=` anchor, the SPA-shell-at-bundle-root layout choice). Substance is ~50 lines of actual code; docstrings + comments make up the rest, per the established Plan 01-02 docstring discipline.

## Task commits

Each task was committed atomically with a small, well-named patch:

1. **Task 0 — pre-flight human-verify** — Already verified by orchestrator before this agent ran (mini-suite fixture rendered correctly under stdlib `python3 -m http.server`; all four asset paths returned 200; SPA listed the two fixture suites). No commit; this gate is observational.
2. **Task 1 — `--serve` / `--port` + `copy_static_assets` + `serve()` in build.py** — `a67402c` (feat)
3. **Task 2 — `test_serve_smoke.py` with 6 HTTP smoke methods** — `1d432fa` (test)

No Rule 1/2/3 deviations were triggered. No checkpoints other than Task 0's orchestrator pre-flight (already approved).

## Files Created/Modified

- `sim-rs/scripts/viz/build.py` (modified, +149/-13) — added `functools`/`shutil`/`http.server` imports; added `copy_static_assets(static_src, output) -> None`; added `serve(output_dir, port) -> None`; extended `parse_args()` with `--serve` + `--port`; wired `main()` to call `run_build` → `copy_static_assets` → optional `serve` with a build-only fallback instruction line.
- `sim-rs/scripts/viz/tests/test_serve_smoke.py` (created, 267 lines) — `ServeSmokeTest(unittest.TestCase)` with `setUp`/`tearDown` + 6 test methods + `_pick_free_port()` helper + `_get(path)` helper + `HEADLINE_LATENCY_LABEL` / `MIN_PLOT_BUNDLE_BYTES` canaries.

## Public API (locked for Plans 01-05 / 01-06 to extend without breaking the serve pipeline)

```python
# sim-rs/scripts/viz/build.py
LANE_FIELDS                                                                      # 12-tuple list (unchanged from Plan 01-02)

discover_suites(source, includes, excludes, warnings) -> list[dict]              # unchanged from Plan 01-02
load_seed(seed_dir, warnings) -> dict | None                                      # unchanged from Plan 01-02
run_build(source, output, includes, excludes, warnings) -> None                   # unchanged from Plan 01-02
copy_static_assets(static_src: Path, output: Path) -> None                        # NEW (Plan 01-04)
serve(output_dir: Path, port: int) -> None                                        # NEW (Plan 01-04)
parse_args() -> argparse.Namespace                                                # EXTENDED (Plan 01-04: --serve, --port)
main() -> None                                                                    # EXTENDED (Plan 01-04: serve + copy_static_assets wiring)
```

```python
# sim-rs/scripts/viz/tests/test_serve_smoke.py
class ServeSmokeTest(unittest.TestCase):
    def setUp(self): ...                              # spawns build.py --serve on a free port, polls 5 s
    def tearDown(self): ...                           # terminate + kill + cleanup tempdir
    def test_root_returns_html(self): ...             # GET / -> 200 + <div id="app">
    def test_index_json_lists_fixture_suite(self):    # GET /data/index.json -> suite_count >= 1
    def test_per_suite_json_present(self): ...        # GET /data/<id>.json -> jobs map
    def test_per_seed_json_present(self): ...         # GET /data/<id>/<job>-<seed>.json -> time_series field
    def test_static_main_js_served(self): ...         # GET /static/main.js -> HEADLINE_LATENCY_LABEL canary
    def test_plot_js_vendored_locally(self): ...      # GET /static/plot.min.js -> >= 50 KB
```

## End-to-End Verification

### Build-only invocation against the fixture

```bash
$ rm -rf /tmp/viz-out-04
$ python3 sim-rs/scripts/viz/build.py \
    --source sim-rs/scripts/viz/tests/fixtures/mini-suite \
    --output /tmp/viz-out-04
Copied 5 static assets to /tmp/viz-out-04/static/ (+ index.html at bundle root)
Wrote /tmp/viz-out-04/data/index.json (source=sim-rs/scripts/viz/tests/fixtures/mini-suite)
Build complete. Open with: python -m http.server --bind 127.0.0.1 --directory /tmp/viz-out-04 8000

$ ls /tmp/viz-out-04/
data  index.html  static
$ ls /tmp/viz-out-04/static/
main.js  plot.min.js  PLOT_VERSION.txt  style.css
$ ls /tmp/viz-out-04/data/
index.json  mini-suite  mini-suite.json
```

All four required artefacts (`index.html`, `static/main.js`, `static/plot.min.js`, `data/index.json`) present at the expected bundle paths.

### Live `--serve` end-to-end (ad-hoc; not part of the test suite)

```bash
$ PORT=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
$ python3 sim-rs/scripts/viz/build.py --serve --port $PORT \
    --source sim-rs/scripts/viz/tests/fixtures/mini-suite \
    --output /tmp/viz-smoke &

$ curl -sS -o /dev/null -w '%{http_code} %{size_download}\n' http://127.0.0.1:$PORT/
200 859
$ curl -sS -o /dev/null -w '%{http_code} %{size_download}\n' http://127.0.0.1:$PORT/data/index.json
200 440
$ curl -sS -o /dev/null -w '%{http_code} %{size_download}\n' http://127.0.0.1:$PORT/static/main.js
200 12301

$ ss -ltn | grep ":$PORT "
LISTEN 0      5          127.0.0.1:55183      0.0.0.0:*
```

`ss -ltn` confirms the bind is `127.0.0.1:55183` only — the listener does NOT appear on `0.0.0.0` or any other interface. T-01-04-01 mitigation verified at the kernel level.

### Full test suite

```bash
$ cd sim-rs && python3 -m unittest discover -s scripts/viz/tests -t scripts/viz -v
... (17 test methods listed) ...
Ran 17 tests in 0.651s
OK
```

| Test                                                            | Status |
|-----------------------------------------------------------------|--------|
| `test_ingest.ErrorHandlingTest.test_malformed_manifest_skipped_with_warning` | PASS |
| `test_ingest.IndexBuildTest.test_index_lists_all_manifests`     | PASS   |
| `test_ingest.IngestTest.test_kebab_case_manifest_snake_case_run_summary` | PASS   |
| `test_ingest.IngestTest.test_latency_blocks_observations_aggregated_to_mean` | PASS |
| `test_ingest.IngestTest.test_missing_time_series_csv_returns_empty_list_with_warning` | PASS |
| `test_ingest.IngestTest.test_phase_2_has_no_priority_only_fast_path_csv` | PASS |
| `test_ingest.IngestTest.test_suite_id_derived_from_path_not_suite_name` | PASS |
| `test_ingest.SeedJsonTest.test_headline_fields_present`         | PASS   |
| `test_ingest.SeedJsonTest.test_time_series_long_form`           | PASS   |
| `test_ingest.SuiteJsonTest.test_jobs_match_manifest`            | PASS   |
| `test_ingest.SuiteJsonTest.test_seed_grouping_present`          | PASS   |
| `test_serve_smoke.ServeSmokeTest.test_index_json_lists_fixture_suite` | PASS |
| `test_serve_smoke.ServeSmokeTest.test_per_seed_json_present`    | PASS   |
| `test_serve_smoke.ServeSmokeTest.test_per_suite_json_present`   | PASS   |
| `test_serve_smoke.ServeSmokeTest.test_plot_js_vendored_locally` | PASS   |
| `test_serve_smoke.ServeSmokeTest.test_root_returns_html`        | PASS   |
| `test_serve_smoke.ServeSmokeTest.test_static_main_js_served`    | PASS   |

## Deviations from Plan

### Auto-fixed issues (Rules 1/2/3)

None — both tasks ran cleanly to verified completion the first time.

### Plan-action choices

**1. [Plan-verification clarification] Tightened the broad bind-landmine grep**
- **Found during:** the plan's `<verification>` block check after Task 2
- **Issue:** the plan's broad pattern `grep -nE '\("0\.0\.0\.0"|"":\s*$|, *""' scripts/viz/build.py` would false-positive on the Plan 01-02 line `rs.get("pricing_event_stream_sha256", "")` (a JSON-default empty string, not a network bind). The plan's `must_haves.truths` is more precise: "the bind address tuple's first element is `\"127.0.0.1\"` not `\"\"`, `None`, or `\"0.0.0.0\"`".
- **Resolution:** verified the three actual landmines explicitly (`("0.0.0.0"`, `HTTPServer(("", `, `HTTPServer((None,`); all three are absent. No `build.py` edit needed; this is a plan-acceptance interpretation choice, not a code change. The narrower gates better match T-01-04-01's actual threat surface (the address tuple's first element passed to the server constructor).
- **No follow-up needed.**

**2. [Plan-action wording] `copy_static_assets` reports "+ index.html at bundle root" in stderr**
- **Found during:** Task 1 implementation against the plan's spec ("Print to `sys.stderr`: `\"Copied N static assets to {output}/static/\"`")
- **Issue:** the asset count `N` in the stderr line includes `index.html`, but `index.html` lands at `<output>/` (bundle root), not `<output>/static/`. The literal plan-text wording would be slightly misleading.
- **Resolution:** Extended the stderr line to `Copied N static assets to {output}/static/ (+ index.html at bundle root)` — preserves the count + path the plan-text dictates, adds a parenthetical that documents the actual disk layout. Honors the plan's intent (one log line per build, points the user at where assets landed) without sacrificing accuracy.
- **No follow-up needed.**

### Auth gates

None — local-only HTTP server bound to loopback, no auth, no external services.

## Threat Surface Outcomes

| Threat ID  | Mitigation                                                                                  | Verified by                                                                  |
|------------|---------------------------------------------------------------------------------------------|------------------------------------------------------------------------------|
| T-01-04-01 | Bind tuple's first element is the literal `"127.0.0.1"`; `("0.0.0.0"` / `("", ` / `(None, ` absent | grep on build.py + `ss -ltn` confirms listener appears only on 127.0.0.1     |
| T-01-04-02 | `httpd.allow_reuse_address = True` set after construction, before `serve_forever`            | grep on build.py + ad-hoc back-to-back `--serve` invocations on same port    |
| T-01-04-03 | (accepted) `--output` path traversal — dev runs on their own filesystem                      | no mitigation; RESEARCH.md Security Domain LOW priority                      |

## Stubs introduced

None. This plan ships production-ready entry-point code and a production-ready integration smoke test. No `TBD` markers, no placeholder data, no stub data flows.

## User Setup Required

None — stdlib-only, no virtualenv, no API keys, no external services. The plan's `<task_0_already_satisfied>` block in the executor prompt confirms the orchestrator already ran the Task 0 pre-flight checkpoint against the mini-suite fixture under the stdlib `python3 -m http.server` and verified the SPA renders correctly. No re-prompting needed.

## Sanity Check vs Plan Targets

| Item                              | Target / Expected                                  | Actual                                              |
|-----------------------------------|----------------------------------------------------|-----------------------------------------------------|
| `build.py` line count (post-04)   | (informational; Plan 01-02 was 727)                | 863                                                  |
| `test_serve_smoke.py` line count  | (informational; ~150-300 expected for 6 methods)   | 267                                                  |
| Test runtime                      | <= 15 s                                            | 0.65 s                                               |
| `127.0.0.1` literal present       | yes (exactly once, in the bind tuple)              | yes (line 736 only)                                  |
| `0.0.0.0` literal absent          | yes                                                | yes (grep empty)                                     |
| `ThreadingHTTPServer` import      | yes                                                | yes (line 86)                                        |
| `allow_reuse_address = True`      | yes                                                | yes (line 737)                                       |
| `--serve` / `--port` flags        | yes, with kebab-case names + snake_case attrs      | yes (lines 807, 813)                                 |
| `copy_static_assets` exported     | top-level function                                 | yes (line 663)                                       |
| `serve` exported                  | top-level function                                 | yes (line 705)                                       |
| 6 smoke test methods present      | all six names from the plan                        | yes (`test_root_returns_html`, `test_index_json_lists_fixture_suite`, `test_per_suite_json_present`, `test_per_seed_json_present`, `test_static_main_js_served`, `test_plot_js_vendored_locally`) |
| Existing 11 ingest tests green    | yes                                                | yes                                                  |
| Stdlib-only smoke imports         | no `requests`/`httpx`/`flask`/`pytest`             | yes (`json`/`socket`/`subprocess`/`sys`/`tempfile`/`time`/`unittest`/`urllib.{error,request}`/`pathlib.Path` only) |
| Free port (no hardcoded 8000)     | `socket.bind(("127.0.0.1", 0))`                    | yes (`_pick_free_port` line 60-74)                   |
| `subprocess.Popen` (not in-process)| yes                                               | yes (line 99)                                        |
| Subprocess stderr on early exit   | surfaced in the test failure message               | yes (setUp lines 115-129 + 138-148)                  |

## Next Phase Readiness

- **Plan 01-05 (Wave 3 — browser views; parallel-eligible)** lands the real `renderHome` / `renderSuite` / `renderJob` chart panes with Observable Plot figures and the cross-seed overlay. It runs against the bundle that `--serve` now produces; no extra plumbing needed. The 6-method smoke test will continue to exercise the SPA's `<div id="app">` mount point, the data-tier JSON contract, and the bundle's `HEADLINE_LATENCY_LABEL` canary against the live `--serve` subprocess — any Plan 01-05 regression in those surfaces fires immediately.
- **Plan 01-06 (Wave 4 — README + end-to-end checkpoint)** documents the resolved command (`python sim-rs/scripts/viz/build.py --serve`) and runs a final hand-load against the live `sim-rs/output/` tree (1884 manifests as of Plan 01-02 smoke). The serve helper is operationally complete; Plan 01-06 is documentation + final visual checkpoint, no further code on this surface.
- **No blockers.** Wave 3 serve entry-point is complete; the smoke test gate is structurally green; both bind landmines are mitigated and verified at the kernel level.

## Self-Check: PASSED

**Created files (verified present on disk):**
- FOUND: `sim-rs/scripts/viz/tests/test_serve_smoke.py` (267 lines)

**Modified files (verified diff against Plan 01-02 baseline):**
- FOUND: `sim-rs/scripts/viz/build.py` (727 → 863 lines, +136)

**Commits (verified in `git log --oneline -3`):**
- FOUND: `a67402c` feat(01-04): add --serve / --port flags + copy_static_assets + serve() to build.py
- FOUND: `1d432fa` test(01-04): add HTTP smoke test for build.py --serve subprocess

**Security grep gates (verified clean):**
- FOUND: `("127.0.0.1"` appears once in build.py (line 736, in the `ThreadingHTTPServer` constructor)
- ABSENT: `("0.0.0.0"`, `HTTPServer(("",`, `HTTPServer((None,` — all three forbidden patterns
- FOUND: `ThreadingHTTPServer` import (line 86) + use (lines 712, 736)
- FOUND: `allow_reuse_address = True` (line 737)

**Tests (verified green):**
- 17/17 tests PASS in 0.65 s
- 11 ingest tests retained green (no regression from Plan 01-02)
- 6 smoke tests new and green

---
*Phase: 01-viz-site-mvp*
*Completed: 2026-05-20*
