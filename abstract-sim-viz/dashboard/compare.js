"use strict";

// Cross-variant sweep comparison. Reads window.SWEEP_DATA, where
// compare.py embeds the Haskell sweep summary.json verbatim. The page
// enriches the per-seed scalar list with a few report-only ratios, then
// presents the result as grouped experiment metrics: mean/spread, range,
// per-seed dots, and delta against a selectable baseline.
const SW = window.SWEEP_DATA;
const S = SW.summary;
const VARIANTS = S.variants || [];

const METRIC_GROUPS = [
  {
    title: "Demand and service",
    metrics: [
      metric("units.total", "Demand units", "count", "neutral"),
      metric("units.served", "Served units", "count", "higher"),
      metric("units.serviceRate", "Service rate", "percent", "higher"),
      metric("units.abandoned", "Abandoned units", "count", "lower"),
      metric("units.unresolved", "Unresolved units", "count", "lower"),
      metric("units.unresolvedRate", "Unresolved rate", "percent", "lower", true),
      metric("inclusion.priority.submitted", "Priority submitted", "count", "neutral"),
      metric("inclusion.priority.included", "Priority included", "count", "higher"),
      metric("inclusion.priority.serviceRate", "Priority inclusion rate", "percent", "higher"),
      metric("inclusion.standard.submitted", "Standard submitted", "count", "neutral"),
      metric("inclusion.standard.included", "Standard included", "count", "higher"),
      metric("inclusion.standard.serviceRate", "Standard inclusion rate", "percent", "higher"),
      metric("inclusion.urgent.submitted", "Urgent submitted", "count", "neutral"),
      metric("inclusion.urgent.included", "Urgent included", "count", "higher"),
      metric("inclusion.urgent.serviceRate", "Urgent inclusion rate", "percent", "higher"),
      metric("load.amplification", "Submission amplification", "ratio", "lower"),
      metric("load.attemptsMax", "Max attempts", "count", "lower"),
      metric("load.postedFeeGrowthMean", "Posted-fee growth", "ratio", "lower"),
    ],
  },
  {
    title: "Value and latency",
    metrics: [
      metric("value.retainedRatio", "Retained value ratio", "percent", "higher", true),
      metric("value.retainedLovelace", "Retained value", "lovelace", "higher"),
      metric("value.lostLovelace", "Lost value", "lovelace", "lower"),
      metric("value.unresolvedLovelace", "Unresolved value", "lovelace", "lower"),
      metric("value.unresolvedShare", "Unresolved value share", "percent", "lower", true),
      metric("latency.meanSlots", "Mean latency, slots", "ratio", "lower"),
      metric("latency.priority.count", "Priority latency observations", "count", "neutral"),
      metric("latency.priority.meanSlots", "Priority mean latency, slots", "ratio", "lower"),
      metric("latency.standard.count", "Standard latency observations", "count", "neutral"),
      metric("latency.standard.meanSlots", "Standard mean latency, slots", "ratio", "lower"),
      metric("latency.urgent.count", "Urgent latency observations", "count", "neutral"),
      metric("latency.urgent.meanSlots", "Urgent mean latency, slots", "ratio", "lower"),
      metric("latency.meanBlocks", "Mean latency, blocks", "ratio", "lower"),
      metric("value.priority.retainedRatio", "Priority retained value ratio", "percent", "higher"),
      metric("value.priority.retainedLovelace", "Priority retained value", "lovelace", "higher"),
      metric("value.priority.lostLovelace", "Priority lost value", "lovelace", "lower"),
      metric("value.priority.unresolvedLovelace", "Priority unresolved value", "lovelace", "lower"),
      metric("value.standard.retainedRatio", "Standard retained value ratio", "percent", "higher"),
      metric("value.standard.retainedLovelace", "Standard retained value", "lovelace", "higher"),
      metric("value.standard.lostLovelace", "Standard lost value", "lovelace", "lower"),
      metric("value.standard.unresolvedLovelace", "Standard unresolved value", "lovelace", "lower"),
      metric("value.urgent.retainedRatio", "Urgent retained value ratio", "percent", "higher"),
      metric("value.urgent.retainedLovelace", "Urgent retained value", "lovelace", "higher"),
      metric("value.urgent.lostLovelace", "Urgent lost value", "lovelace", "lower"),
      metric("value.urgent.unresolvedLovelace", "Urgent unresolved value", "lovelace", "lower"),
    ],
  },
  {
    title: "Throughput and capacity",
    metrics: [
      metric("throughput.txPerSlot", "Transactions per slot", "ratio", "higher"),
      metric("throughput.ebUtilization", "EB utilization", "percent", "higher"),
    ],
  },
  {
    title: "Fees and refunds",
    metrics: [
      metric("revenue.netLovelace", "Net revenue", "lovelace", "higher", true),
      metric("revenue.feesCollectedLovelace", "Fees collected", "lovelace", "higher"),
      metric("revenue.refundsPaidLovelace", "Refunds paid", "lovelace", "neutral"),
      metric("revenue.refundRate", "Refund rate", "percent", "neutral", true),
    ],
  },
  {
    title: "Price behaviour",
    metrics: [
      metric("price.maxJump", "Max price jump", "percent", "lower"),
      metric("price.shockCount", "Price-shock count", "count", "lower"),
      metric("price.oscillationAmplitude", "Oscillation amplitude", "ratio", "lower"),
    ],
  },
];

const HERO_GROUPS = [
  {
    title: "Overall best",
    cards: [
      hero("Best for overall latency", "latency.meanSlots"),
      hero("Best for overall inclusion", "units.serviceRate"),
      hero("Best for overall retained value", "value.retainedRatio"),
    ],
  },
  {
    title: "Priority lane best",
    cards: [
      hero("Best for priority lane latency", "latency.priority.meanSlots", "latency.priority.count", [], [
        comparison("baseline urgent", "latency.urgent.meanSlots"),
        comparison("baseline overall", "latency.meanSlots"),
      ]),
      hero("Best for priority lane inclusion", "inclusion.priority.serviceRate", "inclusion.priority.submitted", [], [
        comparison("baseline urgent", "inclusion.urgent.serviceRate"),
        comparison("baseline overall", "units.serviceRate"),
      ]),
      hero("Best for priority lane retained value", "value.priority.retainedRatio", null, [
        "value.priority.retainedLovelace",
        "value.priority.lostLovelace",
      ], [
        comparison("baseline urgent", "value.urgent.retainedRatio"),
        comparison("baseline overall", "value.retainedRatio"),
      ]),
    ],
  },
  {
    title: "Standard lane best",
    cards: [
      hero("Best for standard lane latency", "latency.standard.meanSlots", "latency.standard.count", [], [
        comparison("baseline overall", "latency.meanSlots"),
      ]),
      hero("Best for standard lane inclusion", "inclusion.standard.serviceRate", "inclusion.standard.submitted", [], [
        comparison("baseline overall", "units.serviceRate"),
      ]),
      hero("Best for standard lane retained value", "value.standard.retainedRatio", null, [
        "value.standard.retainedLovelace",
        "value.standard.lostLovelace",
      ], [
        comparison("baseline overall", "value.retainedRatio"),
      ]),
    ],
  },
];

const METRIC_BY_KEY = new Map(
  METRIC_GROUPS.flatMap((group) => group.metrics.map((m) => [m.key, m]))
);

const DERIVED = {
  "units.unresolvedRate": (s) => ratio(read(s, "units.unresolved"), read(s, "units.total")),
  "value.retainedRatio": (s) => {
    const retained = read(s, "value.retainedLovelace");
    const lost = read(s, "value.lostLovelace");
    return ratio(retained, retained + lost);
  },
  "value.unresolvedShare": (s) => {
    const retained = read(s, "value.retainedLovelace");
    const lost = read(s, "value.lostLovelace");
    const unresolved = read(s, "value.unresolvedLovelace");
    return ratio(unresolved, retained + lost + unresolved);
  },
  "revenue.netLovelace": (s) =>
    read(s, "revenue.feesCollectedLovelace") - read(s, "revenue.refundsPaidLovelace"),
  "revenue.refundRate": (s) =>
    ratio(read(s, "revenue.refundsPaidLovelace"), read(s, "revenue.feesCollectedLovelace")),
};

const el = (id) => document.getElementById(id);

function metric(key, label, format, direction, derived = false) {
  return { key, label, format, direction, derived };
}

function hero(label, key, supportKey = null, supportKeys = [], comparisons = null) {
  return { label, key, supportKey, supportKeys, comparisons: comparisons || [comparison("baseline", key)] };
}

function comparison(label, key) {
  return { label, key };
}

function read(scalars, key) {
  const value = scalars ? scalars[key] : null;
  return Number.isFinite(value) ? value : 0;
}

function ratio(num, den) {
  return den > 0 ? num / den : 0;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function theme() {
  const dark = document.documentElement.dataset.theme === "dark";
  return dark
    ? { text: "#e6e6e6", axis: "#5b6470", grid: "#1b222c", mean: "#f8fafc" }
    : { text: "#1a1a1a", axis: "#9ca3af", grid: "#ececec", mean: "#111827" };
}

function enrichSweepData() {
  for (const variant of VARIANTS) {
    variant._runs = (variant.runs || []).map((run) => {
      const scalars = { ...(run.scalars || {}) };
      for (const [key, derive] of Object.entries(DERIVED)) {
        scalars[key] = derive(scalars);
      }
      return { ...run, scalars };
    });
    const aggregates = { ...(variant.aggregates || {}) };
    for (const key of Object.keys(DERIVED)) {
      aggregates[key] = summaryStats(variant._runs.map((run) => run.scalars[key]));
    }
    variant._aggregates = aggregates;
  }
}

function summaryStats(values) {
  const xs = values.filter((v) => Number.isFinite(v));
  if (xs.length === 0) return { mean: 0, stddev: 0, min: 0, max: 0 };
  const mean = xs.reduce((a, b) => a + b, 0) / xs.length;
  const variance =
    xs.length < 2 ? 0 : xs.reduce((acc, x) => acc + (x - mean) ** 2, 0) / (xs.length - 1);
  return {
    mean,
    stddev: Math.sqrt(variance),
    min: Math.min(...xs),
    max: Math.max(...xs),
  };
}

function fmtMetric(key, value) {
  const def = METRIC_BY_KEY.get(key) || { format: "ratio" };
  if (value == null || Number.isNaN(value)) return "-";
  if (def.format === "percent") return fmtPercent(value);
  if (def.format === "count") return fmtCount(value);
  if (def.format === "lovelace") return fmtCompact(value);
  return fmtCompact(value);
}

function fmtPercent(v) {
  const pct = v * 100;
  const abs = Math.abs(pct);
  if (abs >= 100) return pct.toFixed(1) + "%";
  if (abs >= 10) return pct.toFixed(2) + "%";
  return pct.toFixed(3) + "%";
}

function fmtCount(v) {
  return Number.isInteger(v) && Math.abs(v) < 10000 ? String(v) : fmtCompact(v);
}

function fmtCompact(v) {
  const abs = Math.abs(v);
  if (abs >= 1e12) return (v / 1e12).toFixed(2) + "T";
  if (abs >= 1e9) return (v / 1e9).toFixed(2) + "B";
  if (abs >= 1e6) return (v / 1e6).toFixed(2) + "M";
  if (abs >= 1e4) return (v / 1e3).toFixed(1) + "k";
  if (abs >= 100) return v.toFixed(1);
  if (abs >= 10) return v.toFixed(2);
  return v.toFixed(3);
}

function signed(text, value) {
  return value > 0 ? "+" + text : text;
}

function deltaText(key, value, baseline) {
  const diff = value - baseline;
  const def = METRIC_BY_KEY.get(key) || { format: "ratio" };
  if (Math.abs(diff) < 1e-12) return "baseline";
  if (def.format === "percent") return signed((diff * 100).toFixed(2) + " pp", diff);
  const pct = baseline !== 0 ? diff / Math.abs(baseline) : null;
  const abs = signed(fmtMetric(key, diff), diff);
  return pct == null ? abs : `${abs} (${signed(fmtPercent(pct), pct)})`;
}

function deltaClass(key, value, baseline) {
  const def = METRIC_BY_KEY.get(key) || { direction: "neutral" };
  const diff = value - baseline;
  if (def.direction === "neutral" || Math.abs(diff) < 1e-12) return "";
  const good = def.direction === "higher" ? diff > 0 : diff < 0;
  return good ? "good" : "bad";
}

function availableGroups() {
  return METRIC_GROUPS.map((group) => ({
    ...group,
    metrics: group.metrics.filter((m) => hasMetric(m.key)),
  })).filter((group) => group.metrics.length > 0);
}

function hasMetric(key) {
  return VARIANTS.some((variant) => variant._aggregates && variant._aggregates[key]);
}

function aggregate(variant, key) {
  return variant && variant._aggregates ? variant._aggregates[key] : null;
}

function baselineVariant() {
  const selected = el("baseline-select").value;
  return VARIANTS.find((variant) => variant.name === selected) || VARIANTS[0];
}

function initBaselineSelect() {
  const select = el("baseline-select");
  select.innerHTML = VARIANTS.map((variant, index) => {
    const selected = index === 0 ? " selected" : "";
    return `<option value="${escapeHtml(variant.name)}"${selected}>baseline: ${escapeHtml(variant.name)}</option>`;
  }).join("");
  select.onchange = renderAll;
}

function renderHeader() {
  const bits = [];
  if (S.description) bits.push(S.description);
  bits.push(`${VARIANTS.length} variants x ${S.seeds} seeds x ${S.slots.toLocaleString()} slots`);
  bits.push(`source ${SW.source}`);
  el("subtitle").textContent = bits.join(" · ");
  el("footnote").textContent =
    "Cells show mean across seeds, sample stddev, range, and delta against the selected baseline. " +
    "Charts show per-seed dots, mean ticks, stddev bands, and a dashed baseline mean.";
}

function renderKpis() {
  const baseline = baselineVariant();
  const groups = HERO_GROUPS.map((group) => {
    const cards = group.cards.filter((card) => hasMetric(card.key)).map((card) => renderHeroCard(card, baseline)).join("");
    if (!cards) return "";
    return `
      <section class="kpi-group">
        <h2>${escapeHtml(group.title)}</h2>
        <div class="kpi-grid">${cards}</div>
      </section>`;
  }).join("");
  el("compare-kpis").innerHTML = groups;
}

function renderHeroCard(card, baseline) {
  const def = METRIC_BY_KEY.get(card.key);
  const ranked = VARIANTS.map((variant) => ({ variant, stats: aggregate(variant, card.key) }))
    .filter((row) => row.stats && hasSupport(card, row.variant));
  ranked.sort((a, b) => {
    if (def.direction === "lower") return a.stats.mean - b.stats.mean;
    return b.stats.mean - a.stats.mean;
  });
  const best = ranked[0];
  if (!best) return "";
  const comparisons = card.comparisons.map((comp) => {
    const baseStats = aggregate(baseline, comp.key);
    if (!baseStats) return "";
    const delta = deltaText(card.key, best.stats.mean, baseStats.mean);
    const cls = deltaClass(card.key, best.stats.mean, baseStats.mean);
    return `<div class="delta ${cls}">vs ${escapeHtml(comp.label)}: ${escapeHtml(delta)}</div>`;
  }).join("");
  return `
    <div class="kpi">
      <div class="label">${escapeHtml(card.label)}</div>
      <div class="value">${fmtMetric(card.key, best.stats.mean)}</div>
      <div class="winner">${escapeHtml(best.variant.name)}</div>
      ${comparisons}
    </div>`;
}

function hasSupport(card, variant) {
  if (card.supportKey) {
    const stats = aggregate(variant, card.supportKey);
    return stats && stats.mean > 0;
  }
  if (card.supportKeys.length > 0) {
    return card.supportKeys.some((key) => {
      const stats = aggregate(variant, key);
      return stats && Math.abs(stats.mean) > 0;
    });
  }
  return true;
}

function renderReport() {
  const baseline = baselineVariant();
  el("compare-report").innerHTML = availableGroups().map((group) => {
    const header = VARIANTS.map((variant) => `<th>${escapeHtml(variant.name)}</th>`).join("");
    const rows = group.metrics.map((def) => {
      const cells = VARIANTS.map((variant) => metricCell(def.key, aggregate(variant, def.key), aggregate(baseline, def.key))).join("");
      const derived = def.derived ? ' <span class="basis">derived</span>' : "";
      return `<tr><td>${escapeHtml(def.label)}${derived}<div class="muted">${escapeHtml(def.key)}</div></td>${cells}</tr>`;
    }).join("");
    return `
      <section class="metric-group">
        <h3>${escapeHtml(group.title)}</h3>
        <table>
          <thead><tr><th>metric</th>${header}</tr></thead>
          <tbody>${rows}</tbody>
        </table>
      </section>`;
  }).join("");
}

function metricCell(key, stats, baselineStats) {
  if (!stats) return "<td>-</td>";
  const baseline = baselineStats ? baselineStats.mean : 0;
  const cls = deltaClass(key, stats.mean, baseline);
  const delta = baselineStats ? deltaText(key, stats.mean, baseline) : "-";
  return `
    <td>
      <div class="metric-cell">
        <div class="main">${fmtMetric(key, stats.mean)}</div>
        <div class="spread">sd ${fmtMetric(key, stats.stddev)} · ${fmtMetric(key, stats.min)}..${fmtMetric(key, stats.max)}</div>
        <div class="delta ${cls}">vs baseline ${escapeHtml(delta)}</div>
      </div>
    </td>`;
}

function chartFor(def) {
  const t = theme();
  const names = VARIANTS.map((variant) => variant.name);
  const baseline = baselineVariant();
  const baselineStats = aggregate(baseline, def.key);
  const dots = [];
  const bands = [];
  const means = [];
  for (const variant of VARIANTS) {
    for (const run of variant._runs || []) {
      const val = run.scalars[def.key];
      if (val != null) dots.push({ variant: variant.name, value: val, seed: run.seed });
    }
    const stats = aggregate(variant, def.key);
    if (stats) {
      bands.push({ variant: variant.name, lo: stats.mean - stats.stddev, hi: stats.mean + stats.stddev });
      means.push({ variant: variant.name, mean: stats.mean });
    }
  }
  const fig = document.createElement("figure");
  fig.className = "panel";
  const head = document.createElement("div");
  head.className = "panel-head";
  head.innerHTML = `<div><h2>${escapeHtml(def.label)}</h2><div class="hint">${escapeHtml(def.key)}</div></div>`;
  fig.appendChild(head);
  fig.appendChild(Plot.plot({
    width: 420,
    height: 40 + 26 * Math.max(1, names.length),
    marginLeft: 150,
    marginRight: 16,
    marginTop: 6,
    marginBottom: 30,
    style: { color: t.text, fontSize: "11px" },
    x: { label: null, tickFormat: (d) => fmtMetric(def.key, d), nice: true },
    y: { domain: names, label: null },
    marks: [
      baselineStats ? Plot.ruleX([{ x: baselineStats.mean }], {
        x: "x", stroke: "#ef4444", strokeDasharray: "3,3", strokeOpacity: 0.8,
      }) : null,
      Plot.ruleY(bands, {
        y: "variant", x1: "lo", x2: "hi", stroke: "#94a3b8",
        strokeWidth: 7, strokeOpacity: 0.35,
      }),
      Plot.dot(dots, {
        y: "variant", x: "value", fill: "#2563eb", r: 3, fillOpacity: 0.65,
        title: (d) => `${d.variant} seed ${d.seed}: ${fmtMetric(def.key, d.value)}`,
      }),
      Plot.tickX(means, { y: "variant", x: "mean", stroke: t.mean, strokeWidth: 2 }),
    ].filter(Boolean),
  }));
  return fig;
}

function renderCharts() {
  const groups = availableGroups();
  el("chart-groups").innerHTML = groups.map((group, groupIndex) => `
    <section class="chart-group">
      <h3>${escapeHtml(group.title)}</h3>
      <div id="compare-charts-${groupIndex}" class="compare-charts"></div>
    </section>`).join("");

  groups.forEach((group, groupIndex) => {
    const container = document.getElementById(`compare-charts-${groupIndex}`);
    for (const def of group.metrics) {
      container.appendChild(chartFor(def));
    }
  });
}

function renderAll() {
  renderHeader();
  renderKpis();
  renderReport();
  renderCharts();
}

el("toggle-theme").onclick = () => {
  const root = document.documentElement;
  root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
  renderAll();
};

enrichSweepData();
initBaselineSelect();
renderAll();
