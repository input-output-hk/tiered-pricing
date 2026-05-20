// phase-2 sim viz — browser entry-point (Plan 01-05 real renderers)
//
// Observable Plot 0.6.17 is loaded by the preceding `<script>` tag in
// index.html and is available as the global `window.Plot`. If a future
// iteration wants to use ESM, swap the index.html line and add
// `import * as Plot from "./plot.min.js"` here.
//
// JSON contract consumed by each view (locked by Plan 01-02):
//
//   data/index.json:
//     { generated_at, source, suite_count, suites: [
//         {id, name, path, started_at, job_count, seed_count,
//          completed_count, max_concurrent_jobs}
//     ] }
//
//   data/<suite_id>.json:
//     { id, name, path, started_at, manifest,
//       aggregates: null | object,
//       jobs: {
//         <job_name>: { seeds: {
//           <seed>: { status, started_at, completed_at,
//                     headline: {
//                       retained_value, net_utility,
//                       retained_value_ratio, peak_mempool_bytes,
//                       components: [
//                         {index, latency_blocks_mean,
//                          priority_included, standard_included}
//                       ]
//                     } }
//         } }
//       } }
//
//   data/<suite_id>/<job>-<seed>.json:
//     { suite_id, job, seed,
//       retained_value, priority_retained_value, standard_retained_value,
//       net_utility, retained_value_ratio,
//       total_txs_submitted, total_txs_included,
//       total_txs_evicted_quote_drift,
//       total_fees_paid_lovelace, total_refund_lovelace,
//       pricing_event_stream_sha256,
//       components: [...],
//       time_series: [ {slot, lane, metric, value}, ... ],
//       peak_mempool_bytes }
//
//   Long-form metric/lane combinations available in time_series
//   (set by build.py LANE_FIELDS):
//     metric ∈ {quote_per_byte, mempool_bytes, included_bytes,
//               included_count, fees_paid_lovelace, refund_lovelace,
//               evicted_quote_drift_count}
//     lane   ∈ {priority, standard, total}
//   Chart panes filter by (metric, lane-set) — never the whole array.
//
// Security rules enforced structurally (CRITICAL LANDMINE #7):
//   - All DOM text insertions go through `el(..., {text: ...})` which
//     sets text content via the textContent property; the unsafe
//     HTML-string sink is grep-gated out of this file.
//   - All UI strings referring to per-component latency use the
//     constant HEADLINE_LATENCY_LABEL — never spelled inline. The
//     per-lane-latency phrasing (forbidden per Pitfall 5) is
//     grep-gated out of this file.

const HEADLINE_LATENCY_LABEL = "Latency by demand component (blocks)";

// ----- Formatting helpers ------------------------------------------

function fmtInt(n) {
  if (n == null) return "—";
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 }).format(n);
}

function fmtRatio(n) {
  if (n == null) return "—";
  return new Intl.NumberFormat("en-US", { maximumFractionDigits: 4 }).format(n);
}

function fmtComponents(components) {
  // Per-component latency means as "0.93 / 4.21 / 1.10" (2 decimals).
  if (!Array.isArray(components) || components.length === 0) return "—";
  return components
    .slice(0, 3)
    .map((c) => {
      const v = c && c.latency_blocks_mean;
      return v == null ? "—" : Number(v).toFixed(2);
    })
    .join(" / ");
}

function sortBy(rows, key, dir) {
  // Null-safe sort: nulls always sort to the end regardless of dir.
  return [...rows].sort((a, b) => {
    const av = a == null ? undefined : a[key];
    const bv = b == null ? undefined : b[key];
    if (av == null && bv == null) return 0;
    if (av == null) return 1;
    if (bv == null) return -1;
    if (dir === "desc") {
      return bv > av ? 1 : bv < av ? -1 : 0;
    }
    return av > bv ? 1 : av < bv ? -1 : 0;
  });
}

// ----- DOM helpers --------------------------------------------------

function $app() {
  return document.getElementById("app");
}

function el(tag, opts, ...children) {
  opts = opts || {};
  const node = document.createElement(tag);
  if (opts.class) node.className = opts.class;
  if (opts.text !== undefined) node.textContent = opts.text;  // textContent only
  if (opts.href) node.href = opts.href;
  if (opts.title !== undefined) node.setAttribute("title", String(opts.title));
  if (opts.value !== undefined) node.value = opts.value;
  for (const c of children) {
    if (c == null) continue;
    node.append(c);
  }
  return node;
}

function setView(node) {
  $app().replaceChildren(node);
}

function renderError(message) {
  const root = el("section");
  root.append(el("h2", { text: "Error" }));
  root.append(el("p", { class: "muted", text: message }));
  setView(root);
}

function renderEmptyState(message) {
  const root = el("section");
  root.append(el("p", { class: "muted", text: message }));
  setView(root);
}

// ----- Per-suite JSON fetch cache ----------------------------------
//
// Map<suiteId, Promise<payload>> so re-visiting a suite (e.g. via
// browser back from a job view) doesn't re-fetch. Tiny enough to keep
// without an explicit eviction policy — total cached JSON is bounded
// by the number of suites the user actually clicks into.

const suiteCache = new Map();

function fetchSuite(suiteId) {
  if (!suiteCache.has(suiteId)) {
    const url = "data/" + encodeURIComponent(suiteId) + ".json";
    suiteCache.set(
      suiteId,
      fetch(url).then((res) => {
        if (res.status === 404) return { __notFound: true };
        if (!res.ok) {
          throw new Error("HTTP " + res.status);
        }
        return res.json();
      })
    );
  }
  return suiteCache.get(suiteId);
}

// ----- Router -------------------------------------------------------

async function route() {
  const hash = location.hash || "#/";
  // `#/`, `#/suite/<id>`, `#/job/<suite>/<job>/<seed>`
  const m = hash.match(/^#\/(?:(suite|job)\/(.+))?$/);
  if (!m || !m[1]) {
    return renderHome();
  }
  if (m[1] === "suite") {
    return renderSuite(m[2]);
  }
  if (m[1] === "job") {
    const parts = m[2].split("/");
    if (parts.length < 3) {
      return renderError(
        "Bad job URL: expected #/job/<suite_id>/<job>/<seed>, got " + hash
      );
    }
    // Suite id is the first segment; job is the second; seed is the
    // remainder joined back (seeds may be simple strings today, but
    // the join keeps the parse forward-compatible).
    const [suite, job, ...seedParts] = parts;
    return renderJob(suite, job, seedParts.join("/"));
  }
}

// ----- View: home (suite list) -------------------------------------
//
// VIZ-01: sortable table of every suite from data/index.json.
// Default sort is `started_at` descending (D-18). Clicking a column
// header re-sorts; second click on the same column flips direction.

async function renderHome() {
  let payload;
  try {
    const res = await fetch("data/index.json");
    if (!res.ok) {
      return renderError(
        "Could not load data/index.json (HTTP " + res.status + "). " +
          "Run `python sim-rs/scripts/viz/build.py` first."
      );
    }
    payload = await res.json();
  } catch (e) {
    return renderError("fetch failed: " + e.message);
  }

  const source = (payload && payload.source) || "(unknown source)";
  const generatedAt = (payload && payload.generated_at) || "(unknown timestamp)";
  const suites = (payload && payload.suites) || [];

  const root = el("section");

  // Header strip.
  const header = el("header");
  header.append(el("h1", { text: "phase-2 simulator visualisation" }));
  header.append(el("p", {
    class: "muted",
    text:
      suites.length + " suite" + (suites.length === 1 ? "" : "s") +
      " discovered from " + source + " at " + generatedAt,
  }));
  root.append(header);

  if (suites.length === 0) {
    // Empty-state copy per D-20.
    root.append(el("p", {
      class: "muted",
      text:
        "No suites found under " + source + ". " +
        "Re-run the build after suites land under sim-rs/output/.",
    }));
    setView(root);
    return;
  }

  // Column definitions: {key (data field), label (UI)}.
  // max_concurrent_jobs is the RESEARCH.md Open Q #1 proxy: max
  // overlap of (started, completed) intervals across (job, seed).
  // null means too few timestamps to compute.
  const columns = [
    { key: "name", label: "name" },
    { key: "path", label: "path" },
    { key: "started_at", label: "started_at" },
    { key: "job_count", label: "jobs" },
    { key: "seed_count", label: "seeds" },
    { key: "completed_count", label: "completed" },
    { key: "max_concurrent_jobs", label: "max-concurrent-jobs" },
    { key: "id", label: "id" },
  ];

  // Sort state lives in closure scope so re-renders during sort don't
  // lose it. Default sort is started_at descending (D-18).
  let sortKey = "started_at";
  let sortDir = "desc";

  const table = el("table");
  const thead = el("thead");
  const headerRow = el("tr");

  function rebuildHeader() {
    headerRow.replaceChildren();
    for (const col of columns) {
      const th = el("th", {
        text:
          col.label +
          (col.key === sortKey ? (sortDir === "desc" ? " ▼" : " ▲") : ""),
      });
      if (col.key === sortKey) th.classList.add("sort-active");
      th.style.cursor = "pointer";
      th.addEventListener("click", () => {
        if (sortKey === col.key) {
          sortDir = sortDir === "asc" ? "desc" : "asc";
        } else {
          sortKey = col.key;
          sortDir = "asc";
        }
        rebuildHeader();
        rebuildBody();
      });
      headerRow.append(th);
    }
  }
  rebuildHeader();
  thead.append(headerRow);
  table.append(thead);

  const tbody = el("tbody");

  function rebuildBody() {
    const sorted = sortBy(suites, sortKey, sortDir);
    const rows = sorted.map((suite) => {
      const row = el("tr");

      // Name cell with link to suite drill-down.
      const nameCell = el("td");
      const link = el("a", {
        href: "#/suite/" + encodeURIComponent(suite.id),
        text: suite.name || suite.id || "(unnamed)",
      });
      nameCell.append(link);
      row.append(nameCell);

      row.append(el("td", { text: suite.path == null ? "—" : String(suite.path) }));
      row.append(el("td", { text: suite.started_at == null ? "—" : String(suite.started_at) }));
      row.append(el("td", { text: suite.job_count == null ? "—" : fmtInt(suite.job_count) }));
      row.append(el("td", { text: suite.seed_count == null ? "—" : fmtInt(suite.seed_count) }));
      row.append(el("td", { text: suite.completed_count == null ? "—" : fmtInt(suite.completed_count) }));
      row.append(el("td", {
        // Em dash for null, not the JS string "null".
        text: suite.max_concurrent_jobs == null ? "—" : fmtInt(suite.max_concurrent_jobs),
      }));
      row.append(el("td", { text: suite.id == null ? "—" : String(suite.id) }));
      return row;
    });
    tbody.replaceChildren(...rows);
  }
  rebuildBody();
  table.append(tbody);
  root.append(table);

  setView(root);
}

// ----- View: suite drill-down --------------------------------------
//
// VIZ-02 + VIZ-05: manifest summary + sortable (job, seed) table with
// headline metrics + cross-seed time-series overlay.
//
// Aggregates panel (Pitfall 3 / CRITICAL LANDMINE #2): when
// data.aggregates === null (the phase-2 norm), the section is omitted
// entirely — no header, no placeholder. An HTML comment marks the
// rationale for any future executor reading DevTools.

async function renderSuite(suiteId) {
  let payload;
  try {
    payload = await fetchSuite(suiteId);
  } catch (e) {
    return renderError("Could not load suite " + suiteId + ": " + e.message);
  }
  if (payload && payload.__notFound) {
    const root = el("section");
    root.append(el("p", {
      class: "muted",
      text:
        "Suite not found: " + suiteId +
        ". Rebuild against the current sim-rs/output/ tree.",
    }));
    root.append(el("p", null, el("a", { href: "#/", text: "← back to all suites" })));
    setView(root);
    return;
  }

  const root = el("section");

  const manifest = (payload && payload.manifest) || {};
  const suiteName = payload.name || manifest["suite-name"] || suiteId;

  // Header.
  root.append(el("h1", { text: suiteName }));
  root.append(el("p", { class: "muted", text: payload.path || suiteId }));
  root.append(el("p", null, el("a", { href: "#/", text: "← back to all suites" })));

  // Manifest summary as <dl>. Manifest keys are kebab-case on disk
  // (Pitfall 1 / CLAUDE.md "Serde rename casing is mixed by
  // historical accident") — preserve the disk casing in the UI dt.
  const dl = el("dl");

  const startedAt = manifest["started-at-utc"] || payload.started_at || "—";
  dl.append(el("dt", { text: "started-at-utc" }));
  dl.append(el("dd", { text: String(startedAt) }));

  dl.append(el("dt", { text: "suite-name (manifest)" }));
  dl.append(el("dd", { text: String(manifest["suite-name"] || "—") }));

  // Job and seed counts: count from the populated jobs structure for
  // accuracy across malformed/partial manifests (build.py's index
  // entry uses the same approach).
  const jobs = (payload && payload.jobs) || {};
  const jobNames = Object.keys(jobs).sort();
  let totalSeeds = 0;
  for (const jobName of jobNames) {
    const seedBlock = (jobs[jobName] && jobs[jobName].seeds) || {};
    totalSeeds += Object.keys(seedBlock).length;
  }
  dl.append(el("dt", { text: "job count" }));
  dl.append(el("dd", { text: fmtInt(jobNames.length) }));
  dl.append(el("dt", { text: "seed count" }));
  dl.append(el("dd", { text: fmtInt(totalSeeds) }));

  root.append(dl);

  // Per-(job, seed) table.
  root.append(el("h2", { text: "Jobs and seeds" }));

  // Flatten to rows: one per (job, seed). Pre-derive sortable fields
  // from the nested headline block.
  const rows = [];
  for (const jobName of jobNames) {
    const seedBlock = (jobs[jobName] && jobs[jobName].seeds) || {};
    const seedNames = Object.keys(seedBlock).sort();
    for (const seedName of seedNames) {
      const sb = seedBlock[seedName] || {};
      const h = sb.headline || {};
      rows.push({
        job: jobName,
        seed: seedName,
        status: sb.status || "n/a",
        retained_value: h.retained_value,
        net_utility: h.net_utility,
        retained_value_ratio: h.retained_value_ratio,
        peak_mempool_bytes: h.peak_mempool_bytes,
        components: Array.isArray(h.components) ? h.components : [],
      });
    }
  }

  const seedTable = el("table");
  const seedColumns = [
    { key: "job", label: "job", render: (r) => {
        const cell = el("td");
        cell.append(el("a", {
          href:
            "#/job/" + encodeURIComponent(suiteId) +
            "/" + encodeURIComponent(r.job) +
            "/" + encodeURIComponent(r.seed),
          text: r.job,
        }));
        return cell;
      } },
    { key: "seed", label: "seed", render: (r) => el("td", { text: r.seed }) },
    { key: "status", label: "status", render: (r) => el("td", { text: r.status }) },
    { key: "retained_value", label: "retained_value",
      render: (r) => el("td", { text: fmtInt(r.retained_value) }) },
    { key: "net_utility", label: "net_utility",
      render: (r) => el("td", { text: fmtInt(r.net_utility) }) },
    { key: "retained_value_ratio", label: "retained_value_ratio",
      render: (r) => el("td", { text: fmtRatio(r.retained_value_ratio) }) },
    { key: "peak_mempool_bytes", label: "peak_mempool_bytes",
      render: (r) => el("td", { text: fmtInt(r.peak_mempool_bytes) }) },
    // CRITICAL LANDMINE #3 / Pitfall 5: per-component label, never
    // per-lane. Tooltip text comes from HEADLINE_LATENCY_LABEL.
    { key: "latency_blocks_mean", label: "latency (per component)",
      render: (r) => el("td", {
        text: fmtComponents(r.components),
        title: HEADLINE_LATENCY_LABEL,
      }) },
  ];

  let seedSortKey = "job";
  let seedSortDir = "asc";

  const seedThead = el("thead");
  const seedHeaderRow = el("tr");

  function rebuildSeedHeader() {
    seedHeaderRow.replaceChildren();
    for (const col of seedColumns) {
      const th = el("th", {
        text:
          col.label +
          (col.key === seedSortKey ? (seedSortDir === "desc" ? " ▼" : " ▲") : ""),
      });
      if (col.key === seedSortKey) th.classList.add("sort-active");
      th.style.cursor = "pointer";
      th.addEventListener("click", () => {
        if (seedSortKey === col.key) {
          seedSortDir = seedSortDir === "asc" ? "desc" : "asc";
        } else {
          seedSortKey = col.key;
          seedSortDir = "asc";
        }
        rebuildSeedHeader();
        rebuildSeedBody();
      });
      seedHeaderRow.append(th);
    }
  }

  const seedTbody = el("tbody");

  function rebuildSeedBody() {
    let comparable = rows;
    if (seedSortKey === "latency_blocks_mean") {
      // Lift the first component's mean onto each row so sortBy can
      // compare. Doesn't mutate the original components array.
      comparable = rows.map((r) => ({
        ...r,
        latency_blocks_mean:
          r.components && r.components[0] && r.components[0].latency_blocks_mean,
      }));
    }
    const sorted = sortBy(comparable, seedSortKey, seedSortDir);
    const newRows = sorted.map((r) => {
      const tr = el("tr");
      for (const col of seedColumns) {
        tr.append(col.render(r));
      }
      return tr;
    });
    seedTbody.replaceChildren(...newRows);
  }

  rebuildSeedHeader();
  rebuildSeedBody();
  seedThead.append(seedHeaderRow);
  seedTable.append(seedThead);
  seedTable.append(seedTbody);
  root.append(seedTable);

  // Aggregates section — CRITICAL LANDMINE #2 / Pitfall 3.
  // When data.aggregates === null (every phase-2 suite per Plan 01-02's
  // locked contract), render NOTHING — no header, no placeholder. Drop
  // an HTML comment so a future executor reading DevTools sees why.
  if (payload && payload.aggregates != null) {
    // Reserved for future suites that DO emit an aggregate CSV.
    const aggSection = el("section");
    aggSection.append(el("h2", { text: "Suite aggregates" }));
    const aggTable = el("table");
    const aggBody = el("tbody");
    for (const [k, v] of Object.entries(payload.aggregates)) {
      const tr = el("tr");
      tr.append(el("td", { text: String(k) }));
      tr.append(el("td", { text: typeof v === "number" ? fmtInt(v) : String(v) }));
      aggBody.append(tr);
    }
    aggTable.append(aggBody);
    aggSection.append(aggTable);
    root.append(aggSection);
  } else {
    root.append(document.createComment(
      " Pitfall 3: aggregates is null — phase-2 metrics writer (comparison.rs)" +
      " emits no suite-level *.csv; the historical priority_only_fast_path" +
      "_overall_comparison.csv lives under sim-rs/output/analysis/ and is not" +
      " generated by current phase-2 suites. Render only when aggregates is" +
      " non-null. See Plan 01-02 / Plan 01-05 / Pitfall 3 in RESEARCH.md. "
    ));
  }

  // Cross-seed overlay (VIZ-05 / D-15).
  root.append(renderCrossSeedSection(suiteId, jobNames, jobs));

  setView(root);
}

// Cross-seed overlay section builder.
//
// Lets the user pick a job from the suite and, optionally, a lane;
// fetches every (job, seed) JSON for that job in parallel, filters
// to `metric === "quote_per_byte"` for the chosen lane, and renders
// a single Plot.line chart with `stroke: "seed"`. See RESEARCH.md
// `## Code Examples / Browser-side cross-seed overlay for VIZ-05`.

function renderCrossSeedSection(suiteId, jobNames, jobs) {
  const section = el("section");
  section.append(el("h2", { text: "Cross-seed time-series overlay" }));

  if (jobNames.length === 0) {
    section.append(el("p", {
      class: "muted",
      text: "(no jobs in this suite)",
    }));
    return section;
  }

  const controls = el("p");

  const jobSelect = el("select");
  jobSelect.append(el("option", { value: "", text: "— pick a job —" }));
  for (const j of jobNames) {
    jobSelect.append(el("option", { value: j, text: j }));
  }
  controls.append(el("label", { text: "job: " }, jobSelect));

  const laneSelect = el("select");
  for (const lane of ["priority", "standard"]) {
    laneSelect.append(el("option", { value: lane, text: lane }));
  }
  controls.append(document.createTextNode(" "));
  controls.append(el("label", { text: "lane: " }, laneSelect));

  section.append(controls);

  const chartContainer = el("div", { class: "chart-pane" });
  chartContainer.append(el("p", {
    class: "muted",
    text: "(pick a job to overlay its seeds' quote_per_byte curves)",
  }));
  section.append(chartContainer);

  async function refreshOverlay() {
    const job = jobSelect.value;
    const lane = laneSelect.value;
    if (!job) {
      chartContainer.replaceChildren(el("p", {
        class: "muted",
        text: "(pick a job to overlay its seeds' quote_per_byte curves)",
      }));
      return;
    }
    const seedBlock = (jobs[job] && jobs[job].seeds) || {};
    const seedNames = Object.keys(seedBlock).sort();
    if (seedNames.length === 0) {
      chartContainer.replaceChildren(el("p", {
        class: "muted",
        text: "(no seeds for this job)",
      }));
      return;
    }
    chartContainer.replaceChildren(el("p", {
      class: "muted",
      text: "(loading " + seedNames.length + " seed" +
            (seedNames.length === 1 ? "" : "s") + "...)",
    }));
    try {
      const seedPayloads = await Promise.all(
        seedNames.map((seed) => {
          const url =
            "data/" + encodeURIComponent(suiteId) +
            "/" + encodeURIComponent(job) +
            "-" + encodeURIComponent(seed) + ".json";
          return fetch(url).then((res) => {
            if (!res.ok) throw new Error("HTTP " + res.status + " for " + url);
            return res.json();
          }).then((payload) => ({ seed, payload }));
        })
      );
      const flat = [];
      for (const { seed, payload } of seedPayloads) {
        const ts = Array.isArray(payload.time_series) ? payload.time_series : [];
        for (const r of ts) {
          if (r.metric === "quote_per_byte" && r.lane === lane) {
            flat.push({ slot: r.slot, value: r.value, seed: String(seed) });
          }
        }
      }
      if (flat.length === 0) {
        chartContainer.replaceChildren(el("p", {
          class: "muted",
          text: "(no quote_per_byte records for lane=" + lane +
                " across the " + seedNames.length + " seed" +
                (seedNames.length === 1 ? "" : "s") + " of this job)",
        }));
        return;
      }
      const chart = window.Plot.plot({
        width: 800,
        height: 240,
        color: { legend: true, type: "ordinal" },
        x: { label: "slot" },
        y: { label: "controller quote (lovelace/byte) — lane=" + lane },
        marks: [
          window.Plot.ruleY([0]),
          window.Plot.line(flat, { x: "slot", y: "value", stroke: "seed" }),
        ],
      });
      chartContainer.replaceChildren(chart);
    } catch (e) {
      chartContainer.replaceChildren(el("p", {
        class: "muted",
        text: "(overlay failed: " + e.message + ")",
      }));
    }
  }

  jobSelect.addEventListener("change", refreshOverlay);
  laneSelect.addEventListener("change", refreshOverlay);

  return section;
}

// ----- View: per-(job, seed) detail (stub — Task 2 wires it) -------

async function renderJob(suiteId, job, seed) {
  let payload;
  try {
    const url =
      "data/" +
      encodeURIComponent(suiteId) +
      "/" +
      encodeURIComponent(job) +
      "-" +
      encodeURIComponent(seed) +
      ".json";
    const res = await fetch(url);
    if (res.status === 404) {
      return renderEmptyState(
        "Per-(job, seed) JSON not found for " +
          suiteId + " / " + job + " / " + seed + "."
      );
    }
    if (!res.ok) {
      return renderError(
        "Could not load job " + job + " seed " + seed +
          " (HTTP " + res.status + ")."
      );
    }
    payload = await res.json();
  } catch (e) {
    return renderError("fetch failed: " + e.message);
  }

  const root = el("section");

  // Heading via textContent.
  root.append(el("h1", { text: job + " · seed " + seed }));
  const breadcrumb = el("p", { class: "muted" });
  breadcrumb.append(document.createTextNode(payload.suite_id || suiteId));
  root.append(breadcrumb);
  root.append(el("p", null, el("a", {
    href: "#/suite/" + encodeURIComponent(suiteId),
    text: "← back to suite",
  })));

  // Task 2 of this plan replaces the body below with the headline
  // strip, per-component latency table, and three Plot chart panes.
  root.append(el("p", { class: "muted", text: "(detail view — Task 2 of Plan 01-05 wires the headline strip and charts here)" }));

  setView(root);
}

// ----- Boot ---------------------------------------------------------

window.addEventListener("hashchange", route);
window.addEventListener("DOMContentLoaded", route);

export {
  route,
  renderHome,
  renderSuite,
  renderJob,
  HEADLINE_LATENCY_LABEL,
  fmtInt,
  fmtRatio,
  fmtComponents,
  sortBy,
};
