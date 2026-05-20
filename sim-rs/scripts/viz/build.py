#!/usr/bin/env python3
"""
phase-2 simulator visualisation build script (ingest layer).

Walks an experiment-output tree (default ``sim-rs/output/``) for
``manifest.json`` files, ingests each suite's per-(job, seed)
``run_summary.json`` and ``time_series.csv``, and emits a three-tier
JSON layout under ``--output`` that the static browser bundle then
consumes.

Usage::

  python3 sim-rs/scripts/viz/build.py
      --source sim-rs/output --output sim-rs/output/viz

  python3 sim-rs/scripts/viz/build.py --include 'phase-2/*'

  # --serve / --port flags land in Plan 01-04 (build entry-point).

The emitted layout (D-09 of the phase 01 CONTEXT) is::

  <output>/data/index.json
  <output>/data/<suite_id>.json
  <output>/data/<suite_id>/<job>-<seed>.json

where ``suite_id`` is derived from
``manifest_path.parent.relative_to(source)`` with ``/`` replaced by
``__`` (D-22 / RESEARCH.md Pitfall 2). The Single-Page Application
(SPA) shell loads ``index.json`` on page load, fetches per-suite
JSON on suite click, and fetches per-(job, seed) JSON only when the
user opens that view.

Design contract:

  * stdlib-only (D-01 / D-02 / D-05 / D-08 of CONTEXT.md): no
    ``yaml``, no ``requests``, no ``jinja2``, no ``flask``, no
    virtualenv, no ``requirements.txt``.
  * ``manifest.json`` uses kebab-case keys (``suite-name``,
    ``started-at-utc``, ``jobs[job][seed].status``); ``run_summary.json``
    uses snake_case keys (``priority_retained_value_total``,
    ``components``, ``latency_blocks_observations``). The two casings
    live next to each other on disk by historical Serde accident
    (CLAUDE.md "Serde rename casing is mixed by historical accident"
    / RESEARCH.md Pitfall 1). Square-bracket access is used at every
    required-field site so a schema regression surfaces as a clear
    ``KeyError`` at the call site.
  * ``latency_blocks_observations`` is a per-component list of
    block-latency observations; the Rust accessor
    ``ComponentSummary::latency_blocks_mean()`` is dropped at
    serialisation, so this module computes the mean in Python
    (RESEARCH.md Pitfall 5).
  * ``time_series.csv`` has a pinned 15-column header (matches the
    Rust ``HEADER`` constant in ``sim-cli/src/metrics/time_series.rs``);
    the build emits long-form ``{slot, lane, metric, value}`` records
    so Observable Plot's ``stroke: "lane"`` channel groups series
    natively (D-11 / RESEARCH.md Pattern F).
  * Skip-and-warn on malformed manifest, missing ``run_summary.json``,
    missing ``time_series.csv`` (D-21). The build never exits non-zero
    on data errors; warnings accumulate and print at the end.
  * The per-suite JSON's ``aggregates`` field is ``None`` for every
    phase-2 suite (CRITICAL LANDMINE #2 / RESEARCH.md Pitfall 3):
    the historical ``priority_only_fast_path_overall_comparison.csv``
    lives under ``sim-rs/output/analysis/`` from older work, not in
    any phase-2 suite root. ``metrics_comparison.txt`` is human-only
    prose-Markdown and is never opened by this module
    (RESEARCH.md Pitfall 4) — every field it carries is also in
    ``run_summary.json``.

Reporting numbers (``retained_value``, ``net_utility``,
``retained_value_ratio``) are emitted as plain ``float`` here: this
module reads simulator outputs and never writes back into
simulation-affecting state, so the project's "no ``f64`` in
simulation-affecting state" rule (CLAUDE.md numeric-representation
contract) does not apply to the build's JSON shape.
"""

import argparse
import csv
import fnmatch
import json
import sys
from datetime import datetime
from pathlib import Path

# Long-form CSV → JSON mapping for the pinned 15-column
# ``time_series.csv`` header. Source: ``sim-cli/src/metrics/time_series.rs``
# lines 16-20 (the Rust ``HEADER`` constant). Every value is an
# integer in Rust (``u64``-formatted via ``write!``), so the reader
# casts via ``int(row[col])`` unconditionally — there is no f64 path
# through this module (PATTERNS.md Pattern F).
#
# The ``util_priority_window_x_1e9`` and ``util_standard_window_x_1e9``
# columns are intentionally dropped on the floor: they encode a fixed-
# point utilisation signal (×1e9) that the browser does not chart in
# v1 (every VIZ-04 / D-14 pane is covered by the 13 entries below).
# A future iteration can add them by extending LANE_FIELDS and
# adjusting the front-end metric whitelist.
LANE_FIELDS = [
    ("c_priority", "priority", "quote_per_byte"),
    ("c_standard", "standard", "quote_per_byte"),
    ("mempool_bytes_total", "total", "mempool_bytes"),
    ("mempool_bytes_priority", "priority", "mempool_bytes"),
    ("mempool_bytes_standard", "standard", "mempool_bytes"),
    ("included_bytes_priority", "priority", "included_bytes"),
    ("included_bytes_standard", "standard", "included_bytes"),
    ("included_count_priority", "priority", "included_count"),
    ("included_count_standard", "standard", "included_count"),
    ("fees_paid_lovelace", "total", "fees_paid_lovelace"),
    ("refund_lovelace", "total", "refund_lovelace"),
    ("evicted_quote_drift_count", "total", "evicted_quote_drift_count"),
]


# --------------------------------------------------------------------- helpers


def _iso_to_epoch(s):
    """Parse an ISO-8601 UTC timestamp (with trailing ``Z``) to a
    float-seconds epoch. Returns ``None`` on ``None`` input or on any
    parse failure — callers are expected to treat None as "missing"
    rather than 0.

    RESEARCH.md "Don't Hand-Roll" guidance: use
    ``datetime.fromisoformat`` after swapping the trailing ``Z`` for
    the canonical ``+00:00`` offset. ``fromisoformat`` accepts
    sub-second precision (e.g. ``.000000000``) on Python 3.11+, which
    matches the Rust writer's nanosecond-precision timestamps in
    ``manifest.json``.
    """
    if s is None:
        return None
    try:
        normalised = s.replace("Z", "+00:00")
        return datetime.fromisoformat(normalised).timestamp()
    except (ValueError, AttributeError):
        return None


def _max_concurrent_jobs(manifest):
    """Return the maximum number of (job, seed) start/complete
    intervals that overlap at any single instant in the manifest.

    Proxy for the RESEARCH.md Open Question #1 ("parallelism is not a
    persisted field"): the runner's ``Manifest`` carries no explicit
    parallelism count, but it does carry ``started-at-utc`` and
    ``completed-at-utc`` per (job, seed). Sweeping +1/-1 events over
    the merged timeline yields the wall-clock maximum overlap, which
    matches the operational meaning of "parallelism" — how many jobs
    were in flight at the busiest moment.

    Returns ``None`` if fewer than 2 (job, seed) entries have both
    a parseable start AND a parseable complete timestamp. Returns an
    ``int`` >= 1 otherwise.
    """
    events = []
    for job_entries in manifest.get("jobs", {}).values():
        for entry in job_entries.values():
            start = _iso_to_epoch(entry.get("started-at-utc"))
            end = _iso_to_epoch(entry.get("completed-at-utc"))
            if start is None or end is None:
                continue
            events.append((start, +1))
            # Sentinel: process ``end`` AFTER ``start`` if they share
            # the same timestamp, so a zero-duration interval counts
            # as concurrency-1 rather than being dropped. Sort key
            # (timestamp, delta) — +1 before -1 at the same instant.
            events.append((end, -1))
    if len(events) < 4:  # fewer than 2 usable (start, end) pairs
        return None
    events.sort(key=lambda x: (x[0], -x[1]))
    overlap = 0
    peak = 0
    for _, delta in events:
        overlap += delta
        if overlap > peak:
            peak = overlap
    return peak if peak >= 1 else None


def _matches_globs(rel_path, includes, excludes):
    """Apply ``--include`` / ``--exclude`` globs to a path string.

    Empty ``includes`` means accept-all. Any exclude match drops the
    suite. Matched against the *relative-to-source* directory string,
    not the absolute path, so users write portable glob patterns like
    ``phase-2/*``.
    """
    if includes and not any(fnmatch.fnmatch(rel_path, pat) for pat in includes):
        return False
    if any(fnmatch.fnmatch(rel_path, pat) for pat in excludes):
        return False
    return True


# ----------------------------------------------------------- discovery layer


def discover_suites(source, includes, excludes, warnings):
    """Walk ``source`` for every ``manifest.json`` and return one
    descriptor dict per accepted suite.

    Each returned dict carries:

      * ``id``       — path-derived identifier, ``/`` → ``__``
                       (D-22 / RESEARCH.md Pitfall 2). Two suites
                       whose ``manifest["suite-name"]`` collides
                       still get distinct ids because the parent
                       directory enters the id.
      * ``dir``      — ``Path`` to the suite directory (the parent
                       of ``manifest.json``).
      * ``name``     — ``manifest["suite-name"]`` if present,
                       else the parent directory name.
      * ``started_at`` — ``manifest["started-at-utc"]`` (kebab key).
      * ``jobs``     — the raw ``manifest["jobs"]`` mapping
                       (kebab-cased entries left as-is so downstream
                       readers exercise the casing landmine
                       deliberately).
      * ``manifest`` — the full parsed manifest for downstream use.
      * ``rel_path`` — the path string used for include/exclude
                       glob matching, surfaced for the index.

    Skip-and-warn (D-21): on any
    ``(json.JSONDecodeError, OSError)`` raised by ``open`` /
    ``json.load``, append a warning and continue. ``rglob`` output
    is sorted so emission order is deterministic across runs (lets
    the suite-level goldens in ``sim-cli/tests/determinism.rs``
    serve as a future template).
    """
    suites = []
    source = source.resolve()
    for manifest_path in sorted(source.rglob("manifest.json")):
        rel_dir = manifest_path.parent.relative_to(source)
        rel_path = str(rel_dir)
        if rel_path == ".":
            # ``manifest.json`` sitting directly at ``source`` —
            # treat as a single anonymous suite at the root.
            rel_path = manifest_path.parent.name
        if not _matches_globs(rel_path, includes, excludes):
            continue
        try:
            with open(manifest_path) as f:
                manifest = json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            warnings.append(
                f"skip {manifest_path}: malformed manifest ({e})"
            )
            continue
        suite_id = str(rel_dir).replace("/", "__") if rel_dir != Path(".") else manifest_path.parent.name
        suites.append({
            "id": suite_id,
            "dir": manifest_path.parent,
            "name": manifest.get("suite-name", manifest_path.parent.name),
            "started_at": manifest["started-at-utc"] if "started-at-utc" in manifest else None,
            "jobs": manifest.get("jobs", {}),
            "manifest": manifest,
            "rel_path": rel_path,
        })
    return suites


# ------------------------------------------------------- per-(job, seed) layer


def _read_time_series_long(csv_path):
    """Generator: yield long-form ``{slot, lane, metric, value}``
    records from a per-(job, seed) ``time_series.csv``.

    The Rust writer pins the 15-column header
    (``sim-cli/src/metrics/time_series.rs`` lines 16-20), so this
    reader uses square-bracket access on every column in ``LANE_FIELDS``
    and never paraphrases the header. A schema regression on the
    writer side surfaces here as a ``KeyError`` at the call site
    rather than silently emitting empty series.

    Every column value is an integer (``u64`` in Rust formatted
    via ``write!``), so the reader casts via ``int(row[col])``
    unconditionally — there is no f64 path through this generator
    (PATTERNS.md Pattern F).

    Yields one record per (slot, LANE_FIELDS entry) — so a CSV with
    N rows yields ``N × len(LANE_FIELDS)`` records, in slot-then-
    field order.
    """
    with open(csv_path, newline="") as f:
        for row in csv.DictReader(f):
            slot = int(row["slot"])
            for col, lane, metric in LANE_FIELDS:
                yield {
                    "slot": slot,
                    "lane": lane,
                    "metric": metric,
                    "value": int(row[col]),
                }


def load_seed(seed_dir, warnings):
    """Read a single (job, seed) from disk and return the per-seed
    JSON-shaped dict that the browser's per-(job, seed) detail view
    consumes.

    ``seed_dir`` is conventionally ``<suite_dir>/<job_name>/<seed>``
    (the manifest's ``output-path`` is optional and not always
    reliable). The reader looks for ``run_summary.json`` and
    ``time_series.csv`` inside that directory.

    Returns ``None`` and appends a warning if ``run_summary.json`` is
    missing or unparseable — the seed is then skipped in the per-
    suite emit layer (Task 3). Returns a populated dict with
    ``time_series = []`` and ``peak_mempool_bytes = None`` (plus a
    warning) if ``run_summary.json`` is present but ``time_series.csv``
    is missing (RESEARCH.md Pitfall 8 / D-21 soft-failure path).

    Snake_case access (``priority_retained_value_total``,
    ``standard_retained_value_total``, ``components[i].net_utility_total``,
    ``components[i].latency_blocks_observations``) reflects the
    ``RunSummary`` / ``ComponentSummary`` shapes in
    ``sim-cli/src/metrics/collector.rs`` (which carry no
    ``rename_all = "kebab-case"`` attribute). Required fields use
    square-bracket access so a regression surfaces as a clear
    KeyError; optional / M6-noise fields use ``.get(key, default)``
    per the RESEARCH.md "Verified Schemas" guidance.

    ``latency_blocks_observations`` is a per-component list of
    block-latency observations — NOT a scalar. The Rust accessor
    ``ComponentSummary::latency_blocks_mean()`` is dropped at
    serialisation, so this function computes the mean in Python
    using the same formula
    (``sum(obs) / len(obs)`` with a ``0.0`` empty-list fallback;
    RESEARCH.md Pitfall 5).
    """
    rs_path = seed_dir / "run_summary.json"
    if not rs_path.exists():
        warnings.append(f"skip {seed_dir}: run_summary.json missing")
        return None
    try:
        with open(rs_path) as f:
            rs = json.load(f)
    except (json.JSONDecodeError, OSError) as e:
        warnings.append(f"skip {seed_dir}: run_summary.json unreadable ({e})")
        return None

    # Required snake_case fields (KeyError on schema regression).
    priority_retained = rs["priority_retained_value_total"]
    standard_retained = rs["standard_retained_value_total"]
    retained_value = priority_retained + standard_retained

    # Lovelace totals serialise as numbers (u128 in Rust).
    priority_included_lovelace = rs.get("priority_included_value_total", 0)
    standard_included_lovelace = rs.get("standard_included_value_total", 0)
    included_lovelace_total = priority_included_lovelace + standard_included_lovelace
    if included_lovelace_total > 0:
        retained_value_ratio = retained_value / included_lovelace_total
    else:
        retained_value_ratio = None

    components_out = []
    net_utility_sum = 0.0
    for c in rs["components"]:
        obs = c["latency_blocks_observations"]
        # Pitfall 5 / RESEARCH.md landmine #3: the Vec<f64> is
        # serialised; the mean is computed downstream.
        latency_mean = sum(obs) / len(obs) if obs else 0.0
        net_utility_sum += c["net_utility_total"]
        components_out.append({
            "index": c["component_index"],
            "txs_submitted": c["txs_submitted"],
            "txs_included": c["txs_included"],
            "txs_evicted_quote_drift": c.get("txs_evicted_quote_drift", 0),
            "bytes_included": c["bytes_included"],
            "fees_paid_lovelace": c.get("fees_paid_lovelace", 0),
            "refund_lovelace": c.get("refund_lovelace", 0),
            "retained_value": c["retained_value_total"],
            "net_utility": c["net_utility_total"],
            "latency_blocks_mean": latency_mean,
            "priority_included": c["priority_included"],
            "standard_included": c["standard_included"],
        })

    ts_path = seed_dir / "time_series.csv"
    if ts_path.exists():
        try:
            time_series = list(_read_time_series_long(ts_path))
        except (OSError, ValueError, KeyError) as e:
            warnings.append(f"skip {ts_path}: time_series.csv unreadable ({e})")
            time_series = []
        if time_series:
            mempool_bytes_total = (
                r["value"]
                for r in time_series
                if r["metric"] == "mempool_bytes" and r["lane"] == "total"
            )
            peak_mempool_bytes = max(mempool_bytes_total, default=0)
        else:
            peak_mempool_bytes = None
    else:
        warnings.append(f"{seed_dir}: time_series.csv missing")
        time_series = []
        peak_mempool_bytes = None

    return {
        "retained_value": retained_value,
        "priority_retained_value": priority_retained,
        "standard_retained_value": standard_retained,
        "net_utility": net_utility_sum,
        "retained_value_ratio": retained_value_ratio,
        "total_txs_submitted": rs.get("total_txs_submitted", 0),
        "total_txs_included": rs.get("total_txs_included", 0),
        "total_txs_evicted_quote_drift": rs.get("total_txs_evicted_quote_drift", 0),
        "total_fees_paid_lovelace": rs.get("total_fees_paid_lovelace", 0),
        "total_refund_lovelace": rs.get("total_refund_lovelace", 0),
        "pricing_event_stream_sha256": rs.get("pricing_event_stream_sha256", ""),
        "components": components_out,
        "time_series": time_series,
        "peak_mempool_bytes": peak_mempool_bytes,
    }


# --------------------------------------------------------- emit + CLI layer


def _seed_headline(seed_data):
    """Project the per-seed dict down to the cross-seed overlay
    headline shape carried in the per-suite JSON.

    Lets the browser build the in-suite cross-seed overlay (D-15 /
    VIZ-05) directly from a single ``<suite>.json`` round-trip,
    without re-fetching every per-(job, seed) JSON.
    """
    return {
        "retained_value": seed_data["retained_value"],
        "net_utility": seed_data["net_utility"],
        "retained_value_ratio": seed_data["retained_value_ratio"],
        "peak_mempool_bytes": seed_data["peak_mempool_bytes"],
        "components": [
            {
                "index": c["index"],
                "latency_blocks_mean": c["latency_blocks_mean"],
                "priority_included": c["priority_included"],
                "standard_included": c["standard_included"],
            }
            for c in seed_data["components"]
        ],
    }


def _emit_seed_json(suite_id, job_name, seed, seed_payload, output):
    """Write the per-(job, seed) JSON to
    ``<output>/data/<suite_id>/<job>-<seed>.json``.

    ``sort_keys=True`` and ``indent=2`` keep emission deterministic
    across runs — diff-based testing on this output stays cheap.
    """
    path = output / "data" / suite_id / f"{job_name}-{seed}.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(seed_payload, f, indent=2, sort_keys=True)


def _build_suite_json(suite, source, output, warnings):
    """Build the per-suite JSON (tier-2) and emit each per-(job, seed)
    JSON (tier-3) as we go.

    Returns the per-suite dict that ``run_build`` writes to
    ``<output>/data/<suite_id>.json``.

    The ``aggregates`` field is unconditionally ``None`` for phase-2.
    Pitfall 3 / CRITICAL LANDMINE #2: phase-2's metrics writer emits
    only ``metrics_comparison.txt`` (prose-Markdown). The historical
    ``priority_only_fast_path_overall_comparison.csv`` lives under
    ``sim-rs/output/analysis/`` from older work, not in any phase-2
    suite. NEVER parse ``metrics_comparison.txt`` (Pitfall 4) —
    every field it carries is also in ``run_summary.json``. If a
    future iteration emits suite-level CSVs, populate this field
    then; do not pretend it exists today.

    ``manifest`` is round-tripped verbatim (kebab-cased keys intact)
    so the browser's manifest summary panel can render the raw shape
    without re-deriving anything.
    """
    suite_id = suite["id"]
    manifest = suite["manifest"]
    suite_dir = suite["dir"]
    jobs_out = {}
    for job_name, seeds in suite["jobs"].items():
        seeds_out = {}
        for seed, job_entry in seeds.items():
            seed_dir = suite_dir / job_name / str(seed)
            seed_record = {
                "status": job_entry.get("status"),
                "started_at": job_entry.get("started-at-utc"),
                "completed_at": job_entry.get("completed-at-utc"),
            }
            seed_payload = load_seed(seed_dir, warnings)
            if seed_payload is not None:
                seed_record["headline"] = _seed_headline(seed_payload)
                # tier-3: per-(job, seed) JSON, includes the time-series.
                full_payload = dict(seed_payload)
                full_payload["suite_id"] = suite_id
                full_payload["job"] = job_name
                full_payload["seed"] = str(seed)
                _emit_seed_json(suite_id, job_name, str(seed), full_payload, output)
            else:
                seed_record["headline"] = None
            seeds_out[str(seed)] = seed_record
        jobs_out[job_name] = {"seeds": seeds_out}

    return {
        "id": suite_id,
        "name": suite["name"],
        "path": suite["rel_path"],
        "started_at": suite["started_at"],
        "manifest": manifest,
        "jobs": jobs_out,
        # Pitfall 3 (priority_only_fast_path_overall_comparison.csv):
        # no suite-level CSV is emitted in phase-2. ``aggregates`` is
        # null unconditionally; the browser render path gates on it.
        "aggregates": None,
    }


def _suite_index_entry(suite, suite_json):
    """Build the per-suite entry that lands in ``index.json``.

    ``max_concurrent_jobs`` is the RESEARCH.md Open Question #1
    proxy: the runner's ``Manifest`` carries no explicit parallelism
    field, so we derive an overlap-sweep proxy from the (started,
    completed) timestamps on each (job, seed). Null when fewer than
    two (job, seed) entries have parseable timestamps.
    """
    seeds_total = 0
    completed = 0
    for job_block in suite_json["jobs"].values():
        for seed_record in job_block["seeds"].values():
            seeds_total += 1
            if seed_record.get("status") == "completed":
                completed += 1
    return {
        "id": suite_json["id"],
        "name": suite_json["name"],
        "path": suite_json["path"],
        "started_at": suite_json["started_at"],
        "job_count": len(suite_json["jobs"]),
        "seed_count": seeds_total,
        "completed_count": completed,
        "max_concurrent_jobs": _max_concurrent_jobs(suite["manifest"]),
    }


def run_build(source, output, includes, excludes, warnings):
    """Top-level builder.

    Resolves ``source`` to an absolute path, discovers every suite
    via ``discover_suites``, builds + emits per-suite JSONs (tier-2)
    and per-(job, seed) JSONs (tier-3), then writes
    ``<output>/data/index.json`` (tier-1).

    Every error path appends to ``warnings`` and continues; the build
    never exits non-zero on data errors (D-21). ``json.dump`` calls
    use ``indent=2, sort_keys=True`` so emission is deterministic
    across runs.
    """
    source = source.resolve()
    output = output.resolve()
    data_dir = output / "data"
    data_dir.mkdir(parents=True, exist_ok=True)

    suites = discover_suites(source, includes, excludes, warnings)
    index_entries = []
    for suite in suites:
        try:
            suite_json = _build_suite_json(suite, source, output, warnings)
        except (OSError, KeyError, ValueError) as e:
            warnings.append(f"skip suite {suite['id']}: build failed ({e})")
            continue
        suite_path = data_dir / f"{suite['id']}.json"
        with open(suite_path, "w") as f:
            json.dump(suite_json, f, indent=2, sort_keys=True)
        index_entries.append(_suite_index_entry(suite, suite_json))

    index = {
        "generated_at": datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source": str(source),
        "suite_count": len(index_entries),
        "suites": index_entries,
    }
    with open(data_dir / "index.json", "w") as f:
        json.dump(index, f, indent=2, sort_keys=True)


# ------------------------------------------------------------- CLI surface


def parse_args():
    """Minimal argparse stub for the ingest layer.

    Plan 01-04 (Wave 3) extends this with ``--serve`` / ``--port``
    and the ``copy_static_assets`` hook. Flag names use kebab-case
    (matching ``generate-realistic-100-topology.py``) and the
    resulting Namespace exposes them as snake_case attributes.
    """
    parser = argparse.ArgumentParser(
        description=(
            "phase-2 simulator visualisation build script — ingests "
            "sim-rs/output/ into a three-tier JSON layout for the "
            "static browser bundle."
        ),
    )
    parser.add_argument(
        "--source",
        type=Path,
        default=Path("sim-rs/output"),
        help=(
            "Root directory to walk for manifest.json files "
            "(default: sim-rs/output)."
        ),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("sim-rs/output/viz"),
        help=(
            "Output directory for the generated bundle (default: "
            "sim-rs/output/viz; gitignored transitively via the "
            "existing sim-rs/.gitignore /output rule)."
        ),
    )
    parser.add_argument(
        "--include",
        action="append",
        default=[],
        help=(
            "Glob (matched against the relative-to-source path) to "
            "include. May be passed multiple times. Empty = include all."
        ),
    )
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        help=(
            "Glob (matched against the relative-to-source path) to "
            "exclude. May be passed multiple times."
        ),
    )
    return parser.parse_args()


def main():
    """Entry point: parse_args + run_build + report warnings.

    Plan 01-04 lands the ``--serve`` / ``--port`` branch + the
    ``copy_static_assets`` step that hangs ``static/`` next to
    ``data/`` so the local HTTP server serves both. For now this
    is a build-only stub.
    """
    args = parse_args()
    warnings = []
    run_build(
        source=args.source,
        output=args.output,
        includes=args.include,
        excludes=args.exclude,
        warnings=warnings,
    )
    if warnings:
        print(f"[warnings] {len(warnings)} issues:", file=sys.stderr)
        for w in warnings:
            print(f"  - {w}", file=sys.stderr)
    print(
        f"Wrote {args.output}/data/index.json "
        f"(source={args.source})",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
