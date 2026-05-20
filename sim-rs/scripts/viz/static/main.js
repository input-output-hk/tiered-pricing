// phase-2 sim viz — browser entry-point (Plan 01-03 static-bundle stub)
//
// Observable Plot 0.6.17 is loaded by the preceding `<script>` tag in
// index.html and is available as the global `window.Plot`. If a future
// iteration wants to use ESM, swap the index.html line and add
// `import * as Plot from "./plot.min.js"` here.
//
// This file ships a hash router with placeholder render functions so
// the index.html ↔ main.js ↔ plot.min.js wiring is testable BEFORE the
// chart-rendering logic (Plan 01-05) lands. Stub renderers draw enough
// markup to confirm routing dispatch + data fetching against the JSON
// contract emitted by Plan 01-02 (`data/index.json`, `data/<id>.json`,
// `data/<id>/<job>-<seed>.json`).
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
        "Bad job URL: expected #/job/<suite_id>/<job>/<seed>, got #/" +
          hash.slice(2)
      );
    }
    // Suite id is the first segment; job is the second; seed is the
    // remainder joined back together (seeds are simple strings today,
    // but the join keeps the parse forward-compatible).
    const [suite, job, ...seedParts] = parts;
    return renderJob(suite, job, seedParts.join("/"));
  }
}

// ----- View: home (suite list) -------------------------------------

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

  const suites = (payload && payload.suites) || [];
  if (suites.length === 0) {
    // Empty-state copy per D-20.
    return renderEmptyState(
      "No suites found under sim-rs/output/. " +
        "Re-run the build after suites land under sim-rs/output/."
    );
  }

  const root = el("section");
  root.append(el("h2", { text: "Suites" }));

  const table = el("table");
  const thead = el("thead");
  const headerRow = el("tr");
  const columns = [
    "name", "path", "started_at",
    "jobs", "seeds", "completed", "max_concurrent",
  ];
  for (const col of columns) {
    headerRow.append(el("th", { text: col }));
  }
  thead.append(headerRow);
  table.append(thead);

  const tbody = el("tbody");
  for (const suite of suites) {
    const row = el("tr");
    const nameCell = el("td");
    const link = el("a", {
      href: "#/suite/" + encodeURIComponent(suite.id),
      text: suite.name || suite.id || "(unnamed)",
    });
    nameCell.append(link);
    row.append(nameCell);
    row.append(el("td", { text: suite.path == null ? "" : String(suite.path) }));
    row.append(el("td", { text: suite.started_at == null ? "" : String(suite.started_at) }));
    row.append(el("td", { text: suite.job_count == null ? "" : String(suite.job_count) }));
    row.append(el("td", { text: suite.seed_count == null ? "" : String(suite.seed_count) }));
    row.append(el("td", { text: suite.completed_count == null ? "" : String(suite.completed_count) }));
    row.append(el("td", {
      text: suite.max_concurrent_jobs == null ? "n/a" : String(suite.max_concurrent_jobs),
    }));
    tbody.append(row);
  }
  table.append(tbody);
  root.append(table);

  setView(root);
}

// ----- View: suite drill-down --------------------------------------

async function renderSuite(suiteId) {
  let payload;
  try {
    const res = await fetch("data/" + encodeURIComponent(suiteId) + ".json");
    if (res.status === 404) {
      return renderEmptyState(
        "Suite not found: " + suiteId +
          ". Rebuild against the current sim-rs/output/ tree."
      );
    }
    if (!res.ok) {
      return renderError(
        "Could not load suite " + suiteId + " (HTTP " + res.status + ")."
      );
    }
    payload = await res.json();
  } catch (e) {
    return renderError("fetch failed: " + e.message);
  }

  const root = el("section");

  // Heading (suite name set via textContent — see DOM helpers).
  const manifest = (payload && payload.manifest) || {};
  const suiteName =
    manifest["suite-name"] || payload.suite_name || suiteId;
  root.append(el("h2", { text: suiteName }));

  // Manifest summary as <dl>.
  const dl = el("dl");
  dl.append(el("dt", { text: "suite-id" }));
  dl.append(el("dd", { text: payload.suite_id || suiteId }));
  dl.append(el("dt", { text: "started-at-utc" }));
  dl.append(el("dd", {
    text: manifest["started-at-utc"] || payload.started_at || "n/a",
  }));
  dl.append(el("dt", { text: "job_count" }));
  dl.append(el("dd", { text: String(payload.job_count == null ? "" : payload.job_count) }));
  dl.append(el("dt", { text: "seed_count" }));
  dl.append(el("dd", { text: String(payload.seed_count == null ? "" : payload.seed_count) }));
  root.append(dl);

  // (job, seed) table.
  root.append(el("h3", { text: "Jobs and seeds" }));
  const table = el("table");
  const thead = el("thead");
  const headerRow = el("tr");
  for (const col of ["job", "seed", "status", "headline (Plan 01-05)"]) {
    headerRow.append(el("th", { text: col }));
  }
  thead.append(headerRow);
  table.append(thead);

  const tbody = el("tbody");
  const jobs = (payload && payload.jobs) || {};
  for (const [jobName, jobBlock] of Object.entries(jobs)) {
    const seeds = (jobBlock && jobBlock.seeds) || {};
    for (const [seedName, seedBlock] of Object.entries(seeds)) {
      const row = el("tr");
      const jobCell = el("td");
      const link = el("a", {
        href:
          "#/job/" +
          encodeURIComponent(suiteId) +
          "/" +
          encodeURIComponent(jobName) +
          "/" +
          encodeURIComponent(seedName),
        text: jobName,
      });
      jobCell.append(link);
      row.append(jobCell);
      row.append(el("td", { text: seedName }));
      row.append(el("td", { text: (seedBlock && seedBlock.status) || "n/a" }));
      // Headline-metric cell is a placeholder; Plan 01-05 fills it in.
      row.append(el("td", { class: "muted", text: "TBD - Plan 01-05" }));
      tbody.append(row);
    }
  }
  table.append(tbody);
  root.append(table);

  // Aggregates panel: payload.aggregates is `null` on every phase-2
  // suite per CRITICAL LANDMINE #2 / Plan 01-02's locked contract.
  // Emit an HTML comment marker so a future executor reading DevTools
  // knows why the section is absent. Use createComment which appends
  // a Comment node without parsing HTML.
  root.append(document.createComment(
    " aggregates: null - no priority_only_fast_path_overall_comparison.csv in phase-2 "
  ));
  if (payload && payload.aggregates != null) {
    // Reserved for future suites that DO emit an aggregate CSV.
    const aggSection = el("section");
    aggSection.append(el("h3", { text: "Suite aggregates (Plan 01-05)" }));
    aggSection.append(el("p", { class: "muted", text: "TBD - Plan 01-05" }));
    root.append(aggSection);
  }

  setView(root);
}

// ----- View: per-(job, seed) detail --------------------------------

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
  const heading = el("h2");
  heading.append(document.createTextNode(job + " - seed " + seed));
  root.append(heading);

  const breadcrumb = el("p", { class: "muted" });
  breadcrumb.append(el("a", {
    href: "#/suite/" + encodeURIComponent(suiteId),
    text: "← back to " + suiteId,
  }));
  root.append(breadcrumb);

  // Headline strip placeholder (Plan 01-05 fills in concrete cards).
  const strip = el("div", { class: "headline-strip" });
  const placeholderLabels = [
    "retained_value",
    "net_utility",
    "retained_value_ratio",
    "peak_mempool_bytes",
    HEADLINE_LATENCY_LABEL,
  ];
  for (const label of placeholderLabels) {
    const card = el("div", { class: "headline-card" });
    card.append(el("div", { class: "label", text: label }));
    const value = payload && payload[label] != null
      ? String(payload[label])
      : "TBD";
    card.append(el("div", { class: "value", text: value }));
    strip.append(card);
  }
  root.append(strip);

  // Three chart panes (empty containers — Plan 01-05 mounts Plot
  // figures into them). The 'derived_quote per block' label is a stub:
  // time_series.csv carries no per-block `derived_quote` column today;
  // Plan 01-05 renders `c_priority` + `c_standard` here, lane-coloured
  // (RESEARCH.md `## System Architecture Diagram` enumerates the
  // available columns; VIZ-04 ties the label to the existing fields).
  // TODO Plan 01-05 / VIZ-04: rename or wire derived_quote here.
  const paneLabels = [
    "controller quote per lane",
    "mempool bytes",
    "derived_quote per block",
  ];
  for (const label of paneLabels) {
    const pane = el("div", { class: "chart-pane" });
    pane.append(el("h3", { text: label }));
    pane.append(el("p", { class: "muted", text: "TBD - Plan 01-05" }));
    root.append(pane);
  }

  // Per-component latency placeholder. The UI label must be exactly
  // HEADLINE_LATENCY_LABEL (CRITICAL LANDMINE #3 / Pitfall 5;
  // ComponentSummary mixes both lanes' observations into one list
  // per component, so the per-lane phrasing is incorrect).
  const latencyPane = el("div", { class: "chart-pane" });
  latencyPane.append(el("h3", { text: HEADLINE_LATENCY_LABEL }));
  latencyPane.append(el("p", { class: "muted", text: "TBD - Plan 01-05" }));
  root.append(latencyPane);

  setView(root);
}

// ----- Boot ---------------------------------------------------------

window.addEventListener("hashchange", route);
window.addEventListener("DOMContentLoaded", route);

export { route, renderHome, renderSuite, renderJob, HEADLINE_LATENCY_LABEL };
