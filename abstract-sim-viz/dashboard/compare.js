"use strict";
// Cross-variant sweep comparison: one small multiple per headline scalar,
// per-seed values as dots, the across-seed mean as a tick, ±1 stddev as a
// band. Reads window.SWEEP_DATA (a sweep summary.json embedded verbatim by
// compare.py) — design comparisons should be made on means *with the spread
// in hand*: overlapping bands mean "more seeds", not a conclusion.
const SW = window.SWEEP_DATA;
const S = SW.summary;
const VARIANTS = S.variants || [];

// the scalars highlighted in the header table; charts cover everything
const TABLE_KEYS = [
  "units.serviceRate",
  "latency.meanSlots",
  "load.amplification",
  "throughput.txPerSlot",
  "price.oscillationAmplitude",
];

const el = (id) => document.getElementById(id);

function theme() {
  const dark = document.documentElement.dataset.theme === "dark";
  return dark
    ? { text: "#e6e6e6", axis: "#5b6470", grid: "#1b222c" }
    : { text: "#1a1a1a", axis: "#9ca3af", grid: "#ececec" };
}

// compact value formatting: SI suffixes for big magnitudes (lovelace totals),
// sensible precision for rates and slots
function fmtVal(v) {
  if (v == null || Number.isNaN(v)) return "—";
  const a = Math.abs(v);
  if (a >= 1e12) return (v / 1e12).toFixed(2) + "T";
  if (a >= 1e9) return (v / 1e9).toFixed(2) + "B";
  if (a >= 1e6) return (v / 1e6).toFixed(2) + "M";
  if (a >= 1e4) return (v / 1e3).toFixed(1) + "k";
  if (a >= 100) return v.toFixed(1);
  return v.toFixed(3);
}

function renderHeader() {
  const bits = [];
  if (S.description) bits.push(S.description);
  bits.push(`${VARIANTS.length} variants × ${S.seeds} seeds × ${S.slots.toLocaleString()} slots`);
  bits.push(`source ${SW.source}`);
  el("subtitle").textContent = bits.join(" · ");
  el("footnote").textContent =
    "Dots are per-seed runs; tick = mean across seeds; band = ±1 sample stddev. " +
    "Each variant's exact config and every run's full trace live in the sweep output directory.";
}

function renderTable() {
  const head = TABLE_KEYS
    .map((k) => `<th>${k}</th>`)
    .join("");
  const rows = VARIANTS.map((v) => {
    const cells = TABLE_KEYS.map((k) => {
      const a = (v.aggregates || {})[k];
      if (!a) return "<td>—</td>";
      return `<td>${fmtVal(a.mean)} <span class="sd">±${fmtVal(a.stddev)}</span></td>`;
    }).join("");
    return `<tr><td><b>${v.name}</b></td>${cells}</tr>`;
  }).join("");
  el("compare-table").innerHTML =
    `<table><thead><tr><th>variant</th>${head}</tr></thead><tbody>${rows}</tbody></table>`;
}

function scalarKeys() {
  // JSON object order is preserved: this is the Sweep.headlineScalars order
  const first = VARIANTS[0];
  return first ? Object.keys(first.aggregates || {}) : [];
}

function chartFor(key) {
  const t = theme();
  const names = VARIANTS.map((v) => v.name);
  const dots = [];
  const bands = [];
  const means = [];
  for (const v of VARIANTS) {
    for (const run of v.runs || []) {
      const val = (run.scalars || {})[key];
      if (val != null) dots.push({ variant: v.name, value: val, seed: run.seed });
    }
    const a = (v.aggregates || {})[key];
    if (a) {
      bands.push({ variant: v.name, lo: a.mean - a.stddev, hi: a.mean + a.stddev });
      means.push({ variant: v.name, mean: a.mean });
    }
  }
  const fig = document.createElement("figure");
  fig.className = "panel";
  const head = document.createElement("div");
  head.className = "panel-head";
  head.innerHTML = `<div><h2>${key}</h2></div>`;
  fig.appendChild(head);
  fig.appendChild(Plot.plot({
    width: 380, height: 36 + 26 * Math.max(1, names.length),
    marginLeft: 130, marginRight: 16, marginTop: 6, marginBottom: 28,
    style: { color: t.text, fontSize: "11px" },
    x: { label: null, tickFormat: (d) => fmtVal(d), nice: true },
    y: { domain: names, label: null },
    marks: [
      Plot.ruleY(bands, { y: "variant", x1: "lo", x2: "hi",
        stroke: "#94a3b8", strokeWidth: 7, strokeOpacity: 0.35 }),
      Plot.dot(dots, { y: "variant", x: "value", fill: "#2563eb",
        r: 3, fillOpacity: 0.65, title: (d) => `seed ${d.seed}: ${d.value}` }),
      Plot.tickX(means, { y: "variant", x: "mean", stroke: "#111827", strokeWidth: 2 }),
    ],
  }));
  return fig;
}

function renderCharts() {
  const container = el("compare-charts");
  container.innerHTML = "";
  for (const key of scalarKeys()) {
    container.appendChild(chartFor(key));
  }
}

function renderAll() {
  renderHeader();
  renderTable();
  renderCharts();
}

el("toggle-theme").onclick = () => {
  const root = document.documentElement;
  root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
  renderAll();
};

renderAll();
