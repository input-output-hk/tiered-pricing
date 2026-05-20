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
import functools
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
#  (Task 2 lands here: ``load_seed`` + ``_read_time_series_long``.)


# --------------------------------------------------------- emit + CLI layer
#  (Task 3 lands here: ``_build_suite_json``, ``_emit_seed_json``,
#  ``run_build``, ``parse_args``, ``main``.)
