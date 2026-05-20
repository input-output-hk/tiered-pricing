"""
Wave 0 test scaffolds for the viz-website ingest layer.

These tests target the not-yet-existent ``build`` module under
``sim-rs/scripts/viz/build.py``. Plan 01-02 will land that module
and make these tests green; until then, the tests are intentionally
red (or skipped at import-failure time) — this file is the locked
verification map for Plans 01-02 and 01-03.

Each test class exercises one verified-schema or landmine pinned in
``01-RESEARCH.md`` (Phase Requirements → Test Map, ``## Common
Pitfalls``) and ``01-PATTERNS.md`` (Pattern D for kebab-vs-snake
casing, Pattern E for ``latency_blocks_observations`` list-to-mean).

Run from ``sim-rs/scripts/viz/``::

    python -m unittest discover -s tests -v

Abbreviations: SPA = Single-Page Application; CSV = Comma-Separated
Values; JSON = JavaScript Object Notation. ``Plot`` in this file
refers to the Observable Plot library that the static bundle loads
in the browser; the Python build module never touches Plot.
"""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

# Locate fixtures relative to this test file so the suite works
# regardless of the caller's current working directory.
FIXTURES = Path(__file__).resolve().parent / "fixtures"

# Insert sim-rs/scripts/viz/ on the path so ``import build`` finds the
# (not-yet-existent) build module landed by Plan 01-02. Guard the
# import with try/except so this file remains discoverable until then.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

try:
    import build  # type: ignore  # noqa: F401  — landed by Plan 01-02

    BUILD_AVAILABLE = True
except ImportError:
    BUILD_AVAILABLE = False


# --------------------------------------------------------------------- helpers


def _copy_fixture(fixture_name: str, dest_root: Path) -> Path:
    """Copy a checked-in fixture tree into a fresh tmp dir.

    Returns the destination directory containing the fixture tree.
    Tests pass this directory as ``source=`` to ``build.run_build``.
    """
    src = FIXTURES / fixture_name
    dst = dest_root / fixture_name
    shutil.copytree(src, dst)
    return dst


# ------------------------------------------------------------- VIZ-01 Tier 1


@unittest.skipUnless(BUILD_AVAILABLE, "build module not yet implemented (Wave 1)")
class IndexBuildTest(unittest.TestCase):
    """Tier-1 ``index.json`` lists every discovered suite — VIZ-01."""

    def test_index_lists_all_manifests(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            src = tmp_path / "src"
            src.mkdir()
            _copy_fixture("mini-suite", src)
            out = tmp_path / "out"
            warnings: list = []
            build.run_build(
                source=src,
                output=out,
                includes=[],
                excludes=[],
                warnings=warnings,
            )
            index = json.loads((out / "data" / "index.json").read_text())
            self.assertGreaterEqual(len(index["suites"]), 1)


# ------------------------------------------------------------- VIZ-02 Tier 2


@unittest.skipUnless(BUILD_AVAILABLE, "build module not yet implemented (Wave 1)")
class SuiteJsonTest(unittest.TestCase):
    """Tier-2 ``<suite>.json`` — VIZ-02 (job/seed inventory) and VIZ-05
    (seed-grouping for cross-seed overlay)."""

    def _build_mini_suite(self, tmp_path: Path) -> tuple[Path, list]:
        src = tmp_path / "src"
        src.mkdir()
        _copy_fixture("mini-suite", src)
        out = tmp_path / "out"
        warnings: list = []
        build.run_build(
            source=src,
            output=out,
            includes=[],
            excludes=[],
            warnings=warnings,
        )
        return out, warnings

    def test_jobs_match_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            out, _warnings = self._build_mini_suite(tmp_path)
            # The mini-suite manifest has exactly one job × two seeds.
            index = json.loads((out / "data" / "index.json").read_text())
            self.assertEqual(len(index["suites"]), 1)
            suite_id = index["suites"][0]["id"]
            suite = json.loads((out / "data" / f"{suite_id}.json").read_text())
            jobs = suite["jobs"]
            self.assertIn("d8_target0.5_window32", jobs)
            seeds = jobs["d8_target0.5_window32"]["seeds"]
            self.assertEqual(sorted(seeds.keys()), ["1", "2"])

    def test_seed_grouping_present(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            out, _warnings = self._build_mini_suite(tmp_path)
            index = json.loads((out / "data" / "index.json").read_text())
            suite_id = index["suites"][0]["id"]
            suite = json.loads((out / "data" / f"{suite_id}.json").read_text())
            seeds = suite["jobs"]["d8_target0.5_window32"]["seeds"]
            # Each seed must expose a headline metrics map so the
            # browser can build the cross-seed overlay without
            # re-walking per-(job, seed) JSONs.
            for seed_str in ("1", "2"):
                self.assertIn("headline", seeds[seed_str])
                self.assertIsInstance(seeds[seed_str]["headline"], dict)


# ------------------------------------------------------------- VIZ-03 Tier 3


@unittest.skipUnless(BUILD_AVAILABLE, "build module not yet implemented (Wave 1)")
class SeedJsonTest(unittest.TestCase):
    """Tier-3 ``<suite>/<job>-<seed>.json`` — VIZ-03 (headline metrics)
    and VIZ-04 (long-form time-series records)."""

    def _build_and_load_seed(self, tmp_path: Path, fixture: str) -> dict:
        src = tmp_path / "src"
        src.mkdir()
        _copy_fixture(fixture, src)
        out = tmp_path / "out"
        warnings: list = []
        build.run_build(
            source=src,
            output=out,
            includes=[],
            excludes=[],
            warnings=warnings,
        )
        index = json.loads((out / "data" / "index.json").read_text())
        suite_id = index["suites"][0]["id"]
        # Discover any seed file under data/<suite_id>/ — tests only
        # need one to assert structure.
        seed_files = sorted((out / "data" / suite_id).glob("*.json"))
        self.assertGreaterEqual(len(seed_files), 1)
        return json.loads(seed_files[0].read_text())

    def test_headline_fields_present(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            seed_data = self._build_and_load_seed(tmp_path, "mini-suite")
            self.assertIn("retained_value", seed_data)
            self.assertIn("net_utility", seed_data)
            self.assertIn("retained_value_ratio", seed_data)
            self.assertIn("peak_mempool_bytes", seed_data)
            # ``latency_blocks_mean`` is per-component (not per-lane).
            # See Pitfall 5 / Open Question #2 — UI label is
            # "Latency by demand component (blocks)".
            self.assertIn("components", seed_data)
            self.assertGreaterEqual(len(seed_data["components"]), 1)
            for c in seed_data["components"]:
                self.assertIn("latency_blocks_mean", c)
                self.assertIsInstance(c["latency_blocks_mean"], float)

    def test_time_series_long_form(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            seed_data = self._build_and_load_seed(tmp_path, "mini-suite")
            ts = seed_data["time_series"]
            self.assertIsInstance(ts, list)
            self.assertGreater(len(ts), 0)
            # Long-form records: {slot, lane, metric, value}, NOT
            # wide CSV-shaped rows. Plot's stroke channel groups
            # data into series by ``lane``.
            sample = ts[0]
            self.assertEqual(
                set(sample.keys()),
                {"slot", "lane", "metric", "value"},
            )
            self.assertIsInstance(sample["slot"], int)
            self.assertIsInstance(sample["value"], int)
            lanes = {r["lane"] for r in ts}
            metrics = {r["metric"] for r in ts}
            self.assertIn("priority", lanes)
            self.assertIn("standard", lanes)
            self.assertIn("total", lanes)
            self.assertIn("quote_per_byte", metrics)
            self.assertIn("mempool_bytes", metrics)


# ----------------------------------------------------------- Ingest landmines


@unittest.skipUnless(BUILD_AVAILABLE, "build module not yet implemented (Wave 1)")
class IngestTest(unittest.TestCase):
    """Pitfalls 1 / 2 / 3 / 5 / 8 from ``01-RESEARCH.md`` — the
    casing landmine, the suite-id-from-path landmine, the
    no-priority-only-CSV landmine, the latency-list-to-mean landmine,
    and the missing-time-series soft-failure path."""

    def test_kebab_case_manifest_snake_case_run_summary(self):
        """Pitfall 1: ``manifest.json`` is kebab-case;
        ``run_summary.json`` is snake_case. The build script must
        read both with their native casing."""
        manifest = json.loads(
            (FIXTURES / "mini-suite" / "manifest.json").read_text()
        )
        # Kebab-case access on the manifest.
        self.assertEqual(manifest["suite-name"], "phase-2-mini-suite")
        self.assertIn("started-at-utc", manifest)
        first_job = next(iter(manifest["jobs"].values()))
        first_seed = next(iter(first_job.values()))
        self.assertIn("completed-at-utc", first_seed)
        self.assertIn("output-path", first_seed)

        rs = json.loads(
            (
                FIXTURES
                / "mini-suite"
                / "d8_target0.5_window32"
                / "1"
                / "run_summary.json"
            ).read_text()
        )
        # Snake_case access on the run_summary.
        self.assertIn("priority_retained_value_total", rs)
        self.assertIn("standard_retained_value_total", rs)
        self.assertIn("pricing_event_stream_sha256", rs)
        self.assertIn("latency_blocks_observations", rs["components"][0])

    def test_latency_blocks_observations_aggregated_to_mean(self):
        """Pitfall 5: ``latency_blocks_observations`` is a list per
        component; the Rust ``latency_blocks_mean()`` accessor is
        dropped at serialisation. Build script must compute the
        mean from the list in Python."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            src = tmp_path / "src"
            src.mkdir()
            _copy_fixture("mini-suite", src)
            out = tmp_path / "out"
            warnings: list = []
            build.run_build(
                source=src,
                output=out,
                includes=[],
                excludes=[],
                warnings=warnings,
            )
            # Expected mean for seed 1, component 0 (fixture values).
            fixture_rs = json.loads(
                (
                    FIXTURES
                    / "mini-suite"
                    / "d8_target0.5_window32"
                    / "1"
                    / "run_summary.json"
                ).read_text()
            )
            obs = fixture_rs["components"][0]["latency_blocks_observations"]
            expected_mean = sum(obs) / len(obs)
            # Build output for seed 1.
            index = json.loads((out / "data" / "index.json").read_text())
            suite_id = index["suites"][0]["id"]
            seed_files = sorted(
                (out / "data" / suite_id).glob("d8_target0.5_window32-1*.json")
            )
            self.assertGreaterEqual(len(seed_files), 1)
            seed_data = json.loads(seed_files[0].read_text())
            comp0 = next(
                c
                for c in seed_data["components"]
                if c.get("index", c.get("component_index")) == 0
            )
            self.assertAlmostEqual(
                comp0["latency_blocks_mean"], expected_mean, places=9
            )

    def test_missing_time_series_csv_returns_empty_list_with_warning(self):
        """Pitfall 8: missing ``time_series.csv`` is a soft failure;
        ``time_series == []`` and ``peak_mempool_bytes is None`` plus
        a warning string mentioning the missing CSV."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            src = tmp_path / "src"
            src.mkdir()
            _copy_fixture("no-time-series", src)
            out = tmp_path / "out"
            warnings: list = []
            build.run_build(
                source=src,
                output=out,
                includes=[],
                excludes=[],
                warnings=warnings,
            )
            index = json.loads((out / "data" / "index.json").read_text())
            self.assertGreaterEqual(len(index["suites"]), 1)
            suite_id = index["suites"][0]["id"]
            seed_files = sorted((out / "data" / suite_id).glob("*.json"))
            self.assertGreaterEqual(len(seed_files), 1)
            seed_data = json.loads(seed_files[0].read_text())
            self.assertEqual(seed_data["time_series"], [])
            self.assertIsNone(seed_data["peak_mempool_bytes"])
            self.assertTrue(
                any("time_series.csv" in str(w) for w in warnings),
                f"expected a time_series.csv warning; got {warnings}",
            )

    def test_phase_2_has_no_priority_only_fast_path_csv(self):
        """Pitfall 3 / VIZ-05: per-suite aggregate CSV
        (``priority_only_fast_path_overall_comparison.csv``) does not
        exist in the phase-2 suite tree. The build must NOT pretend
        it does — the suite JSON's ``aggregates`` field is either
        absent or an empty list / None when no ``*.csv`` is present
        at the suite root."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            src = tmp_path / "src"
            src.mkdir()
            _copy_fixture("mini-suite", src)
            # Sanity: no CSV at the suite root.
            suite_root = src / "mini-suite"
            root_csvs = list(suite_root.glob("*.csv"))
            self.assertEqual(root_csvs, [])
            out = tmp_path / "out"
            warnings: list = []
            build.run_build(
                source=src,
                output=out,
                includes=[],
                excludes=[],
                warnings=warnings,
            )
            index = json.loads((out / "data" / "index.json").read_text())
            suite_id = index["suites"][0]["id"]
            suite = json.loads((out / "data" / f"{suite_id}.json").read_text())
            aggregates = suite.get("aggregates")
            # Either absent (None / KeyError-via-get) or empty list.
            self.assertIn(aggregates, (None, [], {}))

    def test_suite_id_derived_from_path_not_suite_name(self):
        """Pitfall 2 / D-22: two suite directories whose
        ``"suite-name"`` collides must still appear as distinct
        entries in ``index.json``, identified by a path-derived id
        (e.g. containing ``__`` as the directory separator after
        sanitisation)."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            src = tmp_path / "src"
            (src / "phase-2" / "suite-x").mkdir(parents=True)
            (src / "phase-2-extra" / "suite-x").mkdir(parents=True)
            common_manifest = {
                "suite-name": "shared-name",
                "started-at-utc": "2026-05-20T10:00:00.000000000Z",
                "jobs": {},
            }
            (src / "phase-2" / "suite-x" / "manifest.json").write_text(
                json.dumps(common_manifest)
            )
            (src / "phase-2-extra" / "suite-x" / "manifest.json").write_text(
                json.dumps(common_manifest)
            )
            out = tmp_path / "out"
            warnings: list = []
            build.run_build(
                source=src,
                output=out,
                includes=[],
                excludes=[],
                warnings=warnings,
            )
            index = json.loads((out / "data" / "index.json").read_text())
            ids = {s["id"] for s in index["suites"]}
            self.assertEqual(len(ids), 2, f"expected 2 distinct ids, got {ids}")
            # The path-derived id sanitises ``/`` to ``__`` so the two
            # parent paths surface in the id.
            self.assertTrue(
                any("__" in i for i in ids),
                f"expected ``__`` separator in path-derived ids; got {ids}",
            )


# --------------------------------------------------------- Error handling


@unittest.skipUnless(BUILD_AVAILABLE, "build module not yet implemented (Wave 1)")
class ErrorHandlingTest(unittest.TestCase):
    """D-21: skip-and-warn on malformed manifest; build continues
    against the remaining valid suites and exits 0."""

    def test_malformed_manifest_skipped_with_warning(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            src = tmp_path / "src"
            src.mkdir()
            _copy_fixture("malformed-suite", src)
            _copy_fixture("mini-suite", src)
            out = tmp_path / "out"
            warnings: list = []
            # Must not raise — malformed manifest is skip-and-warn.
            build.run_build(
                source=src,
                output=out,
                includes=[],
                excludes=[],
                warnings=warnings,
            )
            index = json.loads((out / "data" / "index.json").read_text())
            ids = [s["id"] for s in index["suites"]]
            # mini-suite is ingested; malformed-suite is not.
            self.assertTrue(any("mini-suite" in i for i in ids))
            self.assertFalse(
                any("malformed-suite" in i for i in ids),
                f"malformed-suite should be skipped; got {ids}",
            )
            self.assertTrue(
                any("malformed-suite" in str(w) for w in warnings),
                f"expected a warning mentioning malformed-suite; "
                f"got {warnings}",
            )


if __name__ == "__main__":
    unittest.main()
