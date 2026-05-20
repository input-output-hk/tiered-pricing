"""
Integration smoke test for ``build.py --serve``.

Spawns the actual entry-point as a subprocess against the checked-in
mini-suite fixture and fetches representative URLs over HTTP. The
goal is end-to-end coverage of the full Plan 01-04 surface — the
``argparse`` flag wiring, the ``run_build`` → ``copy_static_assets``
→ ``serve`` sequence, the ``127.0.0.1`` bind, the
``ThreadingHTTPServer`` lifetime — so any regression in the entry
point surfaces here rather than at next-plan integration time.

The test is stdlib-only (``unittest``, ``urllib.request``,
``subprocess``, ``socket``, ``tempfile``) per the workstream's
D-08 "no ``requirements.txt``" decision. No ``requests``, no
``pytest``, no ``httpx``, no test fixtures library.

Run from ``sim-rs/``::

    python -m unittest discover -s scripts/viz/tests -t scripts/viz

Abbreviations: SPA = Single-Page Application; CSV = Comma-Separated
Values; JSON = JavaScript Object Notation; HTTP = Hypertext Transfer
Protocol. ``Plot`` in this file refers to the Observable Plot
library vendored under ``static/plot.min.js`` by Plan 01-03.
"""

import json
import socket
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.error
import urllib.request
from pathlib import Path

# Resolve repo-relative paths once at module load so the test works
# regardless of the caller's current working directory.
THIS_DIR = Path(__file__).resolve().parent
VIZ_DIR = THIS_DIR.parent
BUILD_PY = VIZ_DIR / "build.py"
FIXTURE = THIS_DIR / "fixtures" / "mini-suite"

# Pinned forbidden-label canary from Plan 01-03 / RESEARCH.md Pitfall 5:
# ``latency_blocks_observations`` is per-component, NOT per-lane;
# the SPA's headline label reads exactly this string. If the bundle
# regresses to "latency by lane" the test_static_main_js_served gate
# below fires.
HEADLINE_LATENCY_LABEL = "Latency by demand component (blocks)"

# Sanity floor for the vendored Observable Plot bundle. Plan 01-03
# pinned 209,183 bytes; allow generous slack (>= 50 KB) so an annual
# refresh doesn't churn this constant.
MIN_PLOT_BUNDLE_BYTES = 50_000


def _pick_free_port() -> int:
    """Bind a TCP socket to ``("127.0.0.1", 0)`` to let the kernel pick
    a free ephemeral port, read the chosen port, then release the
    socket. The subprocess re-binds the same port number moments later.

    There's a small race window between the socket close here and the
    subprocess's bind. On a single-developer machine running one test
    at a time, the race is negligible; the alternative (a hardcoded
    high port) would collide with concurrent dev work and with parallel
    test runs.
    """
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]
    finally:
        s.close()


class ServeSmokeTest(unittest.TestCase):
    """End-to-end smoke against ``python build.py --serve``.

    setUp spawns the build entry-point as a subprocess (the serve
    loop blocks; we can't run it in-process and then curl ourselves
    from the same thread without juggling event loops). tearDown
    terminates it and cleans the tempdir. Each test method GETs a
    different URL shape; failures surface as plain
    ``assertEqual``/``assertIn`` rather than stack traces from the
    server side.
    """

    def setUp(self):
        # Per-test tempdir; cleaned up explicitly in tearDown.
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self._tmp.name)
        self.output_dir = self.tmp_path / "out"
        self.port = _pick_free_port()
        # Why a subprocess and not in-process: ``serve()`` calls
        # ``httpd.serve_forever()`` which blocks the calling thread.
        # The test must run the server in a separate process and
        # curl it over the loopback interface.
        self.proc = subprocess.Popen(
            [
                sys.executable,
                str(BUILD_PY),
                "--serve",
                "--source", str(FIXTURE),
                "--output", str(self.output_dir),
                "--port", str(self.port),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        # Poll the loopback URL until either the server answers or the
        # 5-second deadline lapses. A polling loop is preferred over a
        # flat sleep because the mini-suite emission is sub-100ms and a
        # 2-second sleep makes the test gratuitously slow; the
        # tight-loop fallback also catches a subprocess that crashed
        # before binding.
        deadline = time.monotonic() + 5.0
        url = f"http://127.0.0.1:{self.port}/"
        last_exc = None
        while time.monotonic() < deadline:
            # If the subprocess died early surface its stderr — that's
            # the actual error (e.g. ImportError, address-in-use, …).
            if self.proc.poll() is not None:
                stderr = self.proc.stderr.read().decode(errors="replace")
                self.fail(
                    f"build.py --serve exited early with code "
                    f"{self.proc.returncode}.\nstderr:\n{stderr}"
                )
            try:
                with urllib.request.urlopen(url, timeout=0.5) as resp:
                    if resp.status == 200:
                        return
            except (urllib.error.URLError, ConnectionError, OSError) as e:
                last_exc = e
            time.sleep(0.1)
        # Time-out path: kill the server then surface diagnostic info.
        self.proc.terminate()
        try:
            self.proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            self.proc.kill()
        stderr = self.proc.stderr.read().decode(errors="replace") if self.proc.stderr else ""
        self.fail(
            f"server failed to start within 5s on port {self.port}. "
            f"last error: {last_exc!r}\nstderr:\n{stderr}"
        )

    def tearDown(self):
        # Terminate the server process; escalate to kill if it ignores
        # SIGTERM (e.g. wedged in serve_forever). Always close the
        # tempdir to keep `/tmp` clean across the full test sweep.
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=2)
        # Drain remaining stdout/stderr pipes so file descriptors close.
        if self.proc.stdout:
            self.proc.stdout.close()
        if self.proc.stderr:
            self.proc.stderr.close()
        self._tmp.cleanup()

    # ---------------------------------------------------------- helpers

    def _get(self, path):
        """GET ``http://127.0.0.1:<port><path>``; return (status, body bytes)."""
        url = f"http://127.0.0.1:{self.port}{path}"
        with urllib.request.urlopen(url, timeout=2.0) as resp:
            return resp.status, resp.read()

    # ---------------------------------------------------------- gates

    def test_root_returns_html(self):
        """GET / returns 200 and the SPA shell mount point.

        ``<div id="app">`` is the mount point that ``main.js`` writes
        into; its presence in the response body confirms that
        ``copy_static_assets`` placed ``index.html`` at the bundle root
        rather than under ``static/`` (which would 404 here because
        the SPA's relative ``href="static/main.js"`` only resolves
        when the document itself is at the root).
        """
        status, body = self._get("/")
        self.assertEqual(status, 200)
        self.assertIn(b'<div id="app">', body)

    def test_index_json_lists_fixture_suite(self):
        """GET /data/index.json returns valid JSON with the fixture suite."""
        status, body = self._get("/data/index.json")
        self.assertEqual(status, 200)
        self.assertGreater(len(body), 0)
        payload = json.loads(body)
        self.assertGreaterEqual(payload["suite_count"], 1)
        suite_ids = {s["id"] for s in payload["suites"]}
        # The fixture lives at FIXTURE = .../tests/fixtures/mini-suite,
        # which discover_suites turns into the suite id "mini-suite"
        # (since the manifest sits directly at the source root).
        self.assertIn("mini-suite", suite_ids)

    def test_per_suite_json_present(self):
        """GET /data/<suite_id>.json returns valid JSON with a jobs map.

        Reads the first suite id from index.json then resolves the per-
        suite tier-2 JSON. ``jobs`` is the locked Plan 01-02 contract
        field that the SPA's per-suite drill-down consumes.
        """
        _, index_body = self._get("/data/index.json")
        index = json.loads(index_body)
        suite_id = index["suites"][0]["id"]
        status, body = self._get(f"/data/{suite_id}.json")
        self.assertEqual(status, 200)
        payload = json.loads(body)
        self.assertIn("jobs", payload)
        self.assertIsInstance(payload["jobs"], dict)
        self.assertGreater(len(payload["jobs"]), 0)

    def test_per_seed_json_present(self):
        """GET /data/<suite_id>/<job>-<seed>.json returns the tier-3 JSON.

        Walks index.json → per-suite JSON → first (job, seed) and
        confirms the tier-3 emit landed at the expected URL. The
        ``time_series`` field is the load-bearing payload the Plan
        01-05 chart panes consume — its presence here is the contract
        canary.
        """
        _, index_body = self._get("/data/index.json")
        index = json.loads(index_body)
        suite_id = index["suites"][0]["id"]
        _, suite_body = self._get(f"/data/{suite_id}.json")
        suite = json.loads(suite_body)
        job_name = next(iter(suite["jobs"]))
        seed = next(iter(suite["jobs"][job_name]["seeds"]))
        status, body = self._get(f"/data/{suite_id}/{job_name}-{seed}.json")
        self.assertEqual(status, 200)
        payload = json.loads(body)
        self.assertIn("time_series", payload)

    def test_static_main_js_served(self):
        """GET /static/main.js returns 200 with the pinned latency label.

        Two-in-one assertion: confirms the SPA module landed at the
        expected URL AND that Pitfall 5's per-component-not-per-lane
        UI string survived the copy. If a future edit reverts the
        label to "latency by lane" this test fires.
        """
        status, body = self._get("/static/main.js")
        self.assertEqual(status, 200)
        self.assertIn(HEADLINE_LATENCY_LABEL.encode(), body)

    def test_plot_js_vendored_locally(self):
        """GET /static/plot.min.js returns 200 with the vendored bundle.

        The byte-size floor (50 KB) catches a regression where the
        Plan 01-03 vendored UMD bundle is replaced by a CDN
        ``<script src="https://...">`` redirect or a stub file. Plan
        01-03 pinned ~209 KB; 50 KB is the generous floor.
        """
        status, body = self._get("/static/plot.min.js")
        self.assertEqual(status, 200)
        self.assertGreaterEqual(len(body), MIN_PLOT_BUNDLE_BYTES)


if __name__ == "__main__":
    unittest.main()
