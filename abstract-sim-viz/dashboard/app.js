"use strict";
// One dashboard, N runs: SIM_RUNS (written by preprocess.py) carries one
// distilled dataset per trace. Flipping runs rebinds DATA and re-renders every
// panel in place, so the same panel layout/zoom serves all runs.
const RUNS = window.SIM_RUNS || [{ name: "run", data: window.SIM_DATA }];
let runIndex = 0;
let DATA = RUNS[runIndex].data;

const state = {
  priceView: "log",     // "log" | "perlane"
  latencyBy: "lane",    // "lane" | "class" — lane answers "does Priority serve faster?"
  latencyX: "submit",   // "submit" | "incl" — bucket latency by submission or inclusion slot
  flowLane: "all",      // "all" | "Priority" | "Standard" — filter the flow panel by lane
  composeBy: "laneclass", // "laneclass" | "aggregate" — fate/value composition breakdown
  p95Band: true,
  convBand: true,       // show ±5% convergence bands on the price panel
  xDomain: null,        // null = full run
  flowSel: null,        // [loSlot, hiSlot] brushed submit window on the flow panel
  hiddenLanes: new Set(),
  hiddenClasses: new Set(),
  hiddenLatLanes: new Set(),
};

const LANE_COLOR = { Standard: "#2563eb", Priority: "#7c3aed" };

// Ordered, distinguishable palette for urgency classes (low → high decay rate).
// Mid-lightness hues that read on both light and dark backgrounds and avoid the
// lane colors (blue/purple) and the shock red. Falls back to a generated ramp if a
// run ever has more classes than the curated set.
function computeClassColors() {
  const classes = DATA.meta.urgencyClasses;
  const base = ["#0d9488", "#22c55e", "#f59e0b", "#db2777"]; // teal → green → amber → pink
  const ramp = classes.length <= base.length
    ? base
    : d3.quantize((t) => d3.interpolateTurbo(0.1 + 0.8 * t), classes.length);
  const map = {};
  classes.forEach((c, i) => { map[c.id] = ramp[i] || base[0]; });
  return map;
}
let classColors = computeClassColors();

// Pinned price y-domains across runs (per lane, 5% padded): when flipping
// between runs, an auto-scaled axis would make a 2x and a 20x excursion look
// identical. Degenerate or single-run domains stay null (auto-scale).
const PRICE_DOMAIN_BY_LANE = (() => {
  if (RUNS.length < 2) return {};
  const dom = {};
  for (const run of RUNS) {
    for (const [lane, steps] of Object.entries((run.data.price || {}).byLane || {})) {
      for (const p of steps) {
        const d = dom[lane] || [Infinity, -Infinity];
        dom[lane] = [Math.min(d[0], p.oldCoeff, p.newCoeff),
                     Math.max(d[1], p.oldCoeff, p.newCoeff)];
      }
    }
  }
  for (const lane of Object.keys(dom)) {
    const [lo, hi] = dom[lane];
    dom[lane] = lo > 0 && lo < hi ? [lo * 0.95, hi * 1.05] : null;
  }
  return dom;
})();

function pinnedPriceDomain(lanes) {
  const ds = lanes.map((l) => PRICE_DOMAIN_BY_LANE[l]).filter(Boolean);
  if (!ds.length) return null;
  return [Math.min(...ds.map((d) => d[0])), Math.max(...ds.map((d) => d[1]))];
}

const el = (id) => document.getElementById(id);
function theme() {
  const dark = document.documentElement.dataset.theme === "dark";
  return dark
    ? { text: "#e6e6e6", axis: "#5b6470", grid: "#1b222c" }
    : { text: "#1a1a1a", axis: "#9ca3af", grid: "#ececec" };
}
function fullDomain() { return [0, DATA.meta.slotCount]; }
function xDomain() { return state.xDomain || fullDomain(); }
function fmt(n, d = 2) { return n == null ? "—" : (+n).toFixed(d); }

// step lookup: a lane's price coefficient in effect at a given slot
function priceAt(lane, slot) {
  const s = DATA.price.byLane[lane] || [];
  if (!s.length) return null;
  let v = s[0].oldCoeff;
  for (const p of s) { if (p.slot <= slot) v = p.newCoeff; else break; }
  return v;
}

// Responsive widths: charts fill their column instead of a fixed 760px.
function focusWidth() { return Math.max(360, el("focus").clientWidth || 760); }
function distWidth() { return Math.max(200, el("panel-dist").clientWidth || 230); }
// Shared right margin for all focus panels so they align on the slot axis (the
// latency panel reserves room here for its right-hand blocks axis).
function focusRight() { return DATA.meta.expectedSlotsPerBlock ? 48 : 12; }

function panelHead(figureId, titleHtml, hintHtml, exportName, basis) {
  const fig = el(figureId);
  const head = document.createElement("div");
  head.className = "panel-head";
  const basisHtml = basis ? ` <span class="basis">${basis}</span>` : "";
  head.innerHTML = `<div><h2>${titleHtml}</h2><span class="hint">${hintHtml}${basisHtml}</span></div>`;
  const btn = document.createElement("button");
  btn.className = "export"; btn.textContent = "⭳ SVG";
  btn.onclick = () => exportSvg(fig, exportName);
  head.appendChild(btn);
  fig.appendChild(head);
  return fig;
}

function exportSvg(container, filename) {
  const svg = container.querySelector("svg");
  if (!svg) return;
  const clone = svg.cloneNode(true);
  clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  const blob = new Blob([new XMLSerializer().serializeToString(clone)], { type: "image/svg+xml" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url; a.download = filename; a.click();
  URL.revokeObjectURL(url);
}

function renderHeader() {
  const dm = DATA.meta.demand;
  el("subtitle").textContent =
    (RUNS.length > 1 ? `${RUNS[runIndex].name} (run ${runIndex + 1}/${RUNS.length}, [ and ] to flip) · ` : "") +
    `${DATA.meta.slotCount.toLocaleString()} slots · ` +
    `${DATA.meta.totalEvents.toLocaleString()} events · ` +
    (dm ? `${dm.units.toLocaleString()} demand units · ` : "") +
    `source ${DATA.meta.source}`;
  el("footnote").textContent =
    "Load regimes are inferred from observed submissions/slot (the trace omits run config). " +
    "Price is the dynamic coefficient (multiplier on min-fee).";
}

function kpi(label, value, accent) {
  return `<div class="kpi" style="--accent:${accent}">
    <div class="label">${label}</div><div class="value">${value}</div></div>`;
}

function renderKpis() {
  const c = DATA.convergence.byLane;
  const s = DATA.shock.byLane;
  const lanes = DATA.meta.lanes;
  const maxJump = Math.max(...lanes.map((l) => s[l].maxJump));
  const shocks = lanes.reduce((a, l) => a + s[l].shockCount, 0);
  const prio = c.Priority || c[lanes[lanes.length - 1]];
  const std = c.Standard || c[lanes[0]];
  const lat = DATA.latency.byLane || {};
  const sub = DATA.meta.submittedByLane || {};
  const spb = DATA.meta.expectedSlotsPerBlock;
  const blk = (sl) => (spb ? ` (${(sl / spb).toFixed(1)} blk)` : "");
  const drop = (l) => { const tot = sub[l] || 0, inc = (lat[l] || {}).count || 0; return tot ? Math.round(100 * (tot - inc) / tot) : 0; };
  const cards = [
    kpi("Priority conv. time", prio.convergenceTime == null ? "—" : `${prio.convergenceTime} slots`, "#7c3aed"),
    kpi("Max price jump", `${(maxJump * 100).toFixed(1)}%`, "#f59e0b"),
    kpi("# shocks (>10%)", String(shocks), "#ef4444"),
    kpi("Std oscillation", `±${fmt(std.oscillationAmplitude, 3)}`, "#2563eb"),
  ];
  if (lat.Priority) cards.push(kpi("Priority median latency", `${lat.Priority.median} sl${blk(lat.Priority.median)}`, "#7c3aed"));
  if (lat.Standard) cards.push(kpi("Standard median latency", `${lat.Standard.median} sl${blk(lat.Standard.median)}`, "#2563eb"));
  cards.push(kpi("Drop rate · Pri / Std", `${drop("Priority")}% / ${drop("Standard")}%`, "#ef4444"));
  const dm = DATA.meta.demand;
  if (dm && dm.units) {
    cards.unshift(kpi("Demand served", `${(100 * dm.served / dm.units).toFixed(1)}%`, "#16a34a"));
    if (dm.amplification > 1.0005)
      cards.push(kpi("Retry amplification", `${dm.amplification.toFixed(2)}× (${dm.attempts.toLocaleString()} attempts)`, "#d97706"));
  }
  el("kpis").innerHTML = cards.join("");
}

// Latency grouping descriptor — drives the over-time panel, distribution, and table.
function latencyGrouping() {
  if (state.latencyBy === "lane") {
    return {
      title: "Latency / lane",
      tableHead: "lane",
      items: DATA.meta.lanes.map((l) => ({ id: l, label: l, color: LANE_COLOR[l] || "#888" })),
      data: DATA.latency.byLane,
      hidden: state.hiddenLatLanes,
    };
  }
  return {
    title: "Latency / urgency class",
    tableHead: "value ½-life",
    items: DATA.meta.urgencyClasses.map((c) => ({ id: c.id, label: c.label, color: classColors[c.id] })),
    data: DATA.latency.byClass,
    hidden: state.hiddenClasses,
  };
}

function renderLatencyTable() {
  const g = latencyGrouping();
  const spb = DATA.meta.expectedSlotsPerBlock;   // 1/f, expected (matches expectedBlockDelay)
  const blk = (sl) => (spb ? ` <span class="muted">(${(sl / spb).toFixed(1)})</span>` : "");
  const rows = g.items.map((it) => {
    const s = g.data[it.id];
    return `<tr><td style="color:${it.color}">${it.label}</td>
      <td>${s.median}${blk(s.median)}</td><td>${s.p95}${blk(s.p95)}</td><td>${s.max}${blk(s.max)}</td>
      <td>${s.count.toLocaleString()}</td></tr>`;
  }).join("");
  const note = spb
    ? `latency in slots <span class="muted">(expected blocks)</span> · 1 block = ${spb.toFixed(0)} slots (f=${DATA.meta.f})`
    : "latency in slots";
  el("latency-table").innerHTML =
    `<table class="lat"><thead><tr><th>${g.tableHead}</th><th>med</th><th>p95</th><th>max</th><th>n</th></tr></thead>
     <tbody>${rows}</tbody></table>
     <div class="subtitle" style="font-size:10px;margin-top:3px">${note}</div>`;
}

function laneLegend(figureId, lanes, hiddenSet, onToggle) {
  const div = document.createElement("div");
  div.className = "legend";
  lanes.forEach((lane) => {
    const item = document.createElement("span");
    item.className = "item" + (hiddenSet.has(lane) ? " off" : "");
    item.innerHTML = `<span class="swatch" style="background:${LANE_COLOR[lane] || "#888"}"></span>${lane}`;
    item.onclick = () => { onToggle(lane); };
    div.appendChild(item);
  });
  el(figureId).appendChild(div);
}

function convergenceBandMarks(lane) {
  if (!state.convBand) return [];
  const regimes = (DATA.convergence.byLane[lane] || {}).regimes || [];
  return regimes
    .filter((r) => r.band)
    .map((r) => Plot.rect([r], {
      x1: "start", x2: "end", y1: () => r.band[0], y2: () => r.band[1],
      fill: LANE_COLOR[lane] || "#888", fillOpacity: 0.08,
    }));
}

function renderPriceOverlaid(t, lanes) {
  const marks = [Plot.gridY({ stroke: t.grid })];
  lanes.forEach((lane) => marks.push(...convergenceBandMarks(lane)));
  lanes.forEach((lane) =>
    marks.push(Plot.line(DATA.price.byLane[lane], {
      x: "slot", y: "newCoeff", stroke: LANE_COLOR[lane] || "#888",
      strokeWidth: 1.8, curve: "step-after",
    })));
  const pinned = pinnedPriceDomain(lanes);
  return Plot.plot({
    width: focusWidth(), height: 170, marginLeft: 44, marginRight: focusRight(), marginBottom: 18,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), axis: null },
    y: { type: "log", grid: false, label: "coeff ↑", ticks: [1, 2, 4, 8, 16],
         ...(pinned ? { domain: pinned } : {}) },
    marks,
  });
}

function renderPricePerLane(t, lanes) {
  // small multiples: stacked sub-plots, one per lane, each own linear y
  const wrap = document.createElement("div");
  lanes.forEach((lane) => {
    const sub = Plot.plot({
      width: focusWidth(), height: 110, marginLeft: 44, marginRight: focusRight(), marginBottom: 16,
      style: { color: t.text, fontSize: "11px" },
      x: { domain: xDomain(), axis: null },
      y: { grid: false, label: `${lane} ↑`,
           ...(PRICE_DOMAIN_BY_LANE[lane] ? { domain: PRICE_DOMAIN_BY_LANE[lane] } : {}) },
      marks: [
        Plot.gridY({ stroke: t.grid }),
        ...convergenceBandMarks(lane),
        Plot.line(DATA.price.byLane[lane], {
          x: "slot", y: "newCoeff", stroke: LANE_COLOR[lane] || "#888",
          strokeWidth: 1.8, curve: "step-after",
        }),
      ],
    });
    wrap.appendChild(sub);
  });
  return wrap;
}

function renderPricePanel() {
  const t = theme();
  const fig = el("panel-price");
  fig.innerHTML = "";
  panelHead("panel-price", "Price coefficient / lane",
    (state.priceView === "log" ? "log axis" : "per lane") +
      (state.convBand ? " · ±5% convergence band per regime" : ""),
    "price.svg", "x: production slot");
  const lanes = DATA.meta.lanes.filter((l) => !state.hiddenLanes.has(l));
  laneLegend("panel-price", DATA.meta.lanes, state.hiddenLanes, (lane) => {
    state.hiddenLanes.has(lane) ? state.hiddenLanes.delete(lane) : state.hiddenLanes.add(lane);
    renderFocus();
  });
  const node = state.priceView === "log"
    ? renderPriceOverlaid(t, lanes)
    : renderPricePerLane(t, lanes);
  fig.appendChild(node);
}

function renderShockPanel() {
  const t = theme();
  const fig = el("panel-shock");
  fig.innerHTML = "";
  panelHead("panel-shock", "Price shock",
    `|Δ|/old per update · ${(DATA.params.shockThreshold * 100).toFixed(0)}% threshold`,
    "shock.svg", "x: production slot");
  const lanes = DATA.meta.lanes.filter((l) => !state.hiddenLanes.has(l));
  const stems = [];
  lanes.forEach((lane) => {
    const data = DATA.price.byLane[lane];
    stems.push(Plot.ruleX(data, {
      x: "slot", y1: 0, y2: "jump", stroke: LANE_COLOR[lane] || "#888", strokeWidth: 1.5,
    }));
    stems.push(Plot.dot(data.filter((p) => p.jump > DATA.params.shockThreshold), {
      x: "slot", y: "jump", r: 2.5, fill: "#ef4444",
    }));
  });
  const node = Plot.plot({
    width: focusWidth(), height: 110, marginLeft: 44, marginRight: focusRight(), marginBottom: 16,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), axis: null },
    y: { grid: false, label: "jump ↑" },
    marks: [
      Plot.gridY({ stroke: t.grid }),
      Plot.ruleY([DATA.params.shockThreshold], { stroke: "#ef4444", strokeDasharray: "3 3" }),
      ...stems,
    ],
  });
  fig.appendChild(node);
}

function latencyLegend(figureId, g) {
  const div = document.createElement("div");
  div.className = "legend";
  g.items.forEach((it) => {
    const item = document.createElement("span");
    item.className = "item" + (g.hidden.has(it.id) ? " off" : "");
    item.innerHTML = `<span class="swatch" style="background:${it.color}"></span>${it.label}`;
    item.onclick = () => {
      g.hidden.has(it.id) ? g.hidden.delete(it.id) : g.hidden.add(it.id);
      renderLatencyTimePanel(); renderDistribution(); renderLatencyTable();
    };
    div.appendChild(item);
  });
  el(figureId).appendChild(div);
}

function renderLatencyTimePanel() {
  const t = theme();
  const fig = el("panel-latency");
  fig.innerHTML = "";
  const g = latencyGrouping();
  const spb = DATA.meta.expectedSlotsPerBlock;   // 1/f, expected (matches expectedBlockDelay)
  const byIncl = state.latencyX === "incl";
  const otKey = byIncl ? "overTimeIncl" : "overTime";
  panelHead("panel-latency", g.title,
    (spb ? "median · slots (left) · expected blocks (right)" : "median · slots")
      + " · " + (state.p95Band ? "median→p95 band" : "median only"),
    "latency-time.svg",
    byIncl ? "x: production slot (by inclusion)" : "x: submission slot");
  latencyLegend("panel-latency", g);
  const items = g.items.filter((it) => !g.hidden.has(it.id));
  const marks = [Plot.gridY({ stroke: t.grid })];
  if (state.p95Band) {
    items.forEach((it) => marks.push(Plot.areaY(g.data[it.id][otKey], {
      x: "slot", y1: "median", y2: "p95", fill: it.color, fillOpacity: 0.1, curve: "monotone-x",
    })));
  }
  items.forEach((it) => marks.push(Plot.line(g.data[it.id][otKey], {
    x: "slot", y: "median", stroke: it.color, strokeWidth: 2.6, curve: "monotone-x",
  })));
  if (spb) {
    marks.push(Plot.axisY({ anchor: "right", tickFormat: (d) => (d / spb).toFixed(1), label: "blocks ↑" }));
  }
  const node = Plot.plot({
    width: focusWidth(), height: 190, marginLeft: 44, marginRight: focusRight(), marginBottom: 26,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), label: (byIncl ? "production" : "submission") + " slot →" },
    y: { grid: false, label: "latency (slots) ↑" },
    marks,
  });
  fig.appendChild(node);
}

function renderRbTime() {
  const t = theme();
  const fig = el("panel-rb-time");
  fig.innerHTML = "";
  panelHead("panel-rb-time", "RB content over time",
    "txs green / cert amber · darker = fuller (cert shaded by the EB it certifies) · runs = solid stretches",
    "rb-time.svg", "x: production slot");
  const series = (DATA.blocks || {}).rbSeries || [];
  if (!series.length) {
    const p = document.createElement("div");
    p.className = "subtitle"; p.style.fontSize = "10px";
    p.textContent = "no ranking blocks in trace";
    fig.appendChild(p);
    return;
  }
  // hue by content (green = txs, amber = cert); lightness by fullness (darker = fuller).
  // tx fullness = the RB's own utilisation; cert fullness = the certified EB's utilisation.
  const segColor = (d) => {
    const t = 0.35 + 0.6 * Math.max(0, Math.min(1, d.fill == null ? 0 : d.fill));
    return d.kind === "cert" ? d3.interpolateOranges(t) : d3.interpolateGreens(t);
  };
  // each RB spans until the next RB, so adjacent same-kind blocks read as one run
  const segs = series.map((d, i) => ({
    x1: d.slot,
    x2: i + 1 < series.length ? series[i + 1].slot : DATA.meta.slotCount,
    color: segColor(d),
  }));
  const node = Plot.plot({
    width: focusWidth(), height: 34, marginLeft: 44, marginRight: focusRight(),
    marginTop: 2, marginBottom: 4,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), axis: null },
    y: { axis: null, domain: [0, 1] },
    color: { type: "identity" },
    // stroke = fill closes the sub-pixel seams between adjacent segments
    marks: [Plot.rect(segs, { x1: "x1", x2: "x2", y1: 0, y2: 1, fill: "color", stroke: "color", strokeWidth: 1 })],
  });
  fig.appendChild(node);
}

function renderFocus() {
  renderPricePanel();
  renderShockPanel();
  renderRbTime();
  renderLatencyTimePanel();
  renderFlow();
  positionCohortSel();
}

function positionCohortSel() {
  const box = el("cohort-sel");
  if (!box) return;
  const sel = state.flowSel, m = focusXMapping();
  if (!sel || !m) { box.style.display = "none"; return; }
  const a = m.slotToLocal(Math.min(sel[0], sel[1])), b = m.slotToLocal(Math.max(sel[0], sel[1]));
  box.style.left = `${a}px`;
  box.style.width = `${Math.max(1, b - a)}px`;
  box.style.height = `${el("focus").clientHeight}px`;
  box.style.display = "block";
}

function renderDistribution() {
  const t = theme();
  const fig = el("panel-dist");
  fig.innerHTML = "";
  panelHead("panel-dist", "Latency distribution", "IQR · median · p95 · max", "latency-dist.svg");
  const g = latencyGrouping();
  const rows = g.items.filter((it) => !g.hidden.has(it.id)).map((it) => {
    const s = g.data[it.id];
    return { id: it.id, label: it.label, color: it.color,
             p25: s.p25, p75: s.p75, median: s.median, p95: s.p95, max: s.max };
  });
  const node = Plot.plot({
    width: distWidth(), height: 300, marginLeft: 40, marginBottom: 56, marginRight: 8,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: rows.map((r) => r.label), label: null, tickRotate: -30 },
    y: { grid: false, label: "latency (slots) ↑" },
    marks: [
      Plot.gridY({ stroke: t.grid }),
      Plot.ruleX(rows, { x: "label", y1: "p75", y2: "p95", stroke: (d) => d.color, strokeWidth: 1 }),
      Plot.barY(rows, { x: "label", y1: "p25", y2: "p75", fill: (d) => d.color, fillOpacity: 0.3,
        stroke: (d) => d.color }),
      Plot.tickY(rows, { x: "label", y: "median", stroke: (d) => d.color, strokeWidth: 2 }),
      Plot.dot(rows, { x: "label", y: "max", fill: (d) => d.color, r: 2 }),
    ],
  });
  fig.appendChild(node);
}

function renderRb() {
  const t = theme();
  const fig = el("panel-rb");
  fig.innerHTML = "";
  panelHead("panel-rb", "RB content", "ranking blocks: carry txs vs certify an EB", "rb-content.svg");
  const b = DATA.blocks || {};
  const total = b.rbTotal || 0;
  if (!total) {
    const p = document.createElement("div");
    p.className = "subtitle"; p.style.fontSize = "11px";
    p.textContent = "no ranking blocks in trace";
    fig.appendChild(p);
    return;
  }
  const TX = "#16a34a", CERT = "#d97706";
  const rows = [
    { kind: "carry txs", n: b.rbWithTxs, color: TX },
    { kind: "certify EB", n: b.rbWithCert, color: CERT },
  ];
  const node = Plot.plot({
    width: distWidth(), height: 58, marginLeft: 8, marginRight: 8, marginTop: 6, marginBottom: 26,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: [0, total], label: `ranking blocks (n=${total})` },
    color: { domain: rows.map((r) => r.kind), range: rows.map((r) => r.color) },
    marks: [Plot.barX(rows, Plot.stackX({ x: "n", fill: "kind" }))],
  });
  fig.appendChild(node);
  const pct = (n) => Math.round((100 * n) / total);
  const summary = document.createElement("div");
  summary.className = "subtitle"; summary.style.fontSize = "10px"; summary.style.marginTop = "2px";
  summary.innerHTML = rows
    .map((r) => `<span style="color:${r.color}">■</span> ${r.kind}: <b>${r.n}</b> (${pct(r.n)}%)`)
    .join(" &nbsp; ");
  fig.appendChild(summary);
}

// shared 100%-stacked-bar composition (fate, value). Two modes:
//  - "laneclass": Priority vs Standard within each urgency class (de-confounds selection bias)
//  - "aggregate": by lane and by class separately (lane rows are composition-confounded)
function renderComposition(figId, title, hint, exportName, src, cats, colors, pctLabel) {
  const t = theme();
  const fig = el(figId);
  fig.innerHTML = "";
  const laneClass = state.composeBy === "laneclass";
  panelHead(figId, title,
    hint + (laneClass ? " · Priority vs Standard within each class" : " · ⚠ lane rows are composition-confounded"),
    exportName);
  const rows = [];
  const add = (label, d) => cats.forEach((k) => rows.push({ group: label, kind: k, n: d[k] }));
  let groups;
  if (laneClass) {
    const abbr = (l) => (l === "Priority" ? "Pri" : l === "Standard" ? "Std" : l);
    groups = [];
    DATA.meta.urgencyClasses.forEach((c) => {
      const short = c.halfLifeBlocks == null ? c.label : `${Math.round(c.halfLifeBlocks)}b`;
      DATA.meta.lanes.forEach((l) => {
        const lbl = `${abbr(l)} ${short}`;
        groups.push(lbl);
        add(lbl, src.byClassLane[c.id][l]);
      });
    });
  } else {
    groups = [...DATA.meta.lanes, ...DATA.meta.urgencyClasses.map((c) => c.label)];
    DATA.meta.lanes.forEach((l) => add(l, src.byLane[l]));
    DATA.meta.urgencyClasses.forEach((c) => add(c.label, src.byClass[c.id]));
  }
  const node = Plot.plot({
    width: distWidth(), height: 42 + 22 * groups.length,
    marginLeft: laneClass ? 56 : 84, marginRight: 8, marginTop: 6, marginBottom: 26,
    style: { color: t.text, fontSize: "11px" },
    x: { percent: true, label: pctLabel },
    y: { domain: groups, label: null },
    color: { domain: cats, range: cats.map((k) => colors[k]), legend: true },
    marks: [
      Plot.barX(rows, { y: "group", x: "n", fill: "kind", offset: "normalize", order: cats }),
      Plot.ruleX([0, 1]),
    ],
  });
  fig.appendChild(node);
}

function renderFate() {
  renderComposition("panel-fate", "Demand fate",
    "share of demand units (one per intent, however many attempts) · served vs abandoned (gave up after rejection/eviction) / unresolved (in flight at run end)", "fate.svg",
    DATA.fate, ["included", "abandoned", "unresolved"],
    { included: "#16a34a", abandoned: "#ef4444", unresolved: "#9ca3af" },
    "% of demand units");
}

function renderValue() {
  renderComposition("panel-value", "Value retained vs lost",
    "share of demand-unit value · retained at inclusion (decayed from first submission) vs lost (decay + abandonment); unresolved = in flight at run end", "value.svg",
    DATA.value, ["retained", "lost", "unresolved"],
    { retained: "#16a34a", lost: "#ef4444", unresolved: "#9ca3af" },
    "% of value");
}

function renderFairness() {
  const t = theme();
  const fig = el("panel-fairness");
  fig.innerHTML = "";
  const F = DATA.fairness || { jainIndex: 1, nActors: 0, starvedTxs: 0, actors: [] };
  panelHead("panel-fairness", "Fairness / starvation",
    "Jain index over per-actor served demand units · starved = in flight at run end", "fairness.svg");
  const head = document.createElement("div");
  head.className = "subtitle"; head.style.fontSize = "11px"; head.style.margin = "2px 0 4px";
  head.innerHTML =
    `<div><b>Jain index ${F.jainIndex.toFixed(3)}</b> over ${F.nActors} actor${F.nActors === 1 ? "" : "s"}</div>`
    + `<div><span style="color:#ef4444">${(F.starvedTxs || 0).toLocaleString()}</span> demand units unresolved`
    + ` <span class="muted">(in flight at run end: never served, never abandoned)</span></div>`;
  fig.appendChild(head);
  const rows = (F.actors || []).slice().sort((a, b) => a.rate - b.rate)
    .map((a) => ({ label: `actor ${a.id}`, rate: a.rate }));
  const node = Plot.plot({
    width: focusWidth(), height: 24 + 22 * Math.max(1, rows.length),
    marginLeft: 60, marginRight: 8, marginTop: 4, marginBottom: 24,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: [0, 1], percent: true, label: "inclusion rate" },
    y: { domain: rows.map((r) => r.label), label: null },
    marks: [
      Plot.barX(rows, { y: "label", x: "rate", fill: "#2563eb", fillOpacity: 0.6 }),
      Plot.ruleX([0, 1], { stroke: t.grid }),
    ],
  });
  fig.appendChild(node);
}

const LOAD_DIMS = { width: 760, height: 70, marginLeft: 44, marginRight: 12, marginTop: 6, marginBottom: 18 };

function renderContext() {
  const t = theme();
  const fig = el("panel-load");
  fig.innerHTML = "";
  panelHead("panel-load", "Load (submissions/slot)", "brush to zoom the panels above · double-click to reset", "load.svg", "x: production slot");
  const node = Plot.plot({
    width: focusWidth(), height: LOAD_DIMS.height,
    marginLeft: LOAD_DIMS.marginLeft, marginRight: LOAD_DIMS.marginRight,
    marginTop: LOAD_DIMS.marginTop, marginBottom: LOAD_DIMS.marginBottom,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: fullDomain(), label: "slot →" },
    y: { axis: null },
    marks: [
      Plot.areaY(DATA.load.buckets, { x: "slot", y: "submissions", fill: t.axis, fillOpacity: 0.25 }),
      Plot.ruleX((DATA.convergence.loadRegimes || []).slice(1).map((r) => r.start),
        { stroke: t.axis, strokeDasharray: "2 2" }),
    ],
  });
  fig.appendChild(node);
  attachBrush(node);
}

function attachBrush(svgNode) {
  const x = svgNode.scale("x");                       // { domain:[d0,d1], range:[r0,r1] }
  const [r0, r1] = x.range, [d0, d1] = x.domain;
  const pxToSlot = (px) => d0 + ((px - r0) / (r1 - r0)) * (d1 - d0);
  const slotToPx = (s) => r0 + ((s - d0) / (d1 - d0)) * (r1 - r0);

  const brush = d3.brushX()
    .extent([[r0, LOAD_DIMS.marginTop], [r1, LOAD_DIMS.height - LOAD_DIMS.marginBottom]])
    .on("end", (event) => {
      if (!event.sourceEvent) return;
      if (!event.selection) {                         // cleared (e.g. double-click)
        if (state.xDomain) { state.xDomain = null; renderFocus(); }
        return;
      }
      const [a, b] = event.selection;
      state.xDomain = [Math.max(d0, pxToSlot(a)), Math.min(d1, pxToSlot(b))];
      renderFocus();
    });

  const g = d3.select(svgNode).append("g").attr("class", "brush").call(brush);
  if (state.xDomain) {                                // preserve current selection on re-render
    g.call(brush.move, [slotToPx(state.xDomain[0]), slotToPx(state.xDomain[1])]);
  }
}

function renderFlow() {
  const t = theme();
  const fig = el("panel-flow");
  fig.innerHTML = "";
  const flow = DATA.flow || {};
  const links = flow.links || [];
  panelHead("panel-flow", "Submission ⇄ inclusion",
    "select a window above · top = submitted, bottom = included · shows txs submitted-in (→ later) and included-in (← earlier) · green RB / amber EB · uniform ~"
      + (100 * (flow.sampleRate || 0)).toFixed(0) + "% sample",
    "flow.svg", "links both clocks");
  // shares the focus x-axis exactly (same margins + xDomain) so it aligns with the column
  const W = focusWidth(), H = 150, ml = 44, mr = focusRight(), topY = 26, botY = H - 28;
  const [d0, d1] = xDomain();
  const x0 = ml, x1 = W - mr;
  const sx = (slot) => x0 + (slot - d0) / (d1 - d0) * (x1 - x0);
  const RB = "#16a34a", EB = "#d97706", CAP = 600;
  const p = [
    `<line x1="${x0}" y1="${topY}" x2="${x1}" y2="${topY}" stroke="${t.axis}"/>`,
    `<line x1="${x0}" y1="${botY}" x2="${x1}" y2="${botY}" stroke="${t.axis}"/>`,
    `<text x="${x0 + 2}" y="${topY - 4}" font-size="9" fill="${t.text}">submitted ↧</text>`,
    `<text x="${x0 + 2}" y="${botY + 13}" font-size="9" fill="${t.text}">included</text>`,
  ];
  let note = "drag across the time panels above to pick a window";
  const sel = state.flowSel;
  if (sel && links.length) {
    const lo = Math.min(sel[0], sel[1]), hi = Math.max(sel[0], sel[1]);
    const inSub = (dd) => dd[0] >= lo && dd[0] <= hi;   // submitted in window
    const inInc = (dd) => dd[1] >= lo && dd[1] <= hi;   // included in window
    let win = links.filter((dd) => inSub(dd) || inInc(dd));
    if (state.flowLane !== "all") {
      const lc = state.flowLane === "Priority" ? 1 : 0;
      win = win.filter((dd) => dd[3] === lc);
    }
    const subN = win.reduce((a, dd) => a + (inSub(dd) ? 1 : 0), 0);
    const incN = win.reduce((a, dd) => a + (inInc(dd) ? 1 : 0), 0);
    if (win.length > CAP) { const k = Math.ceil(win.length / CAP); win = win.filter((_, i) => i % k === 0); }
    const cy = (topY + botY) / 2 + 18;
    for (const dd of win) {
      const a = sx(dd[0]), b = sx(dd[1]);
      if ((a < x0 - 1 || a > x1 + 1) && (b < x0 - 1 || b > x1 + 1)) continue;
      const col = dd[2] === 0 ? RB : EB;
      p.push(`<path d="M${a.toFixed(1)},${topY} Q${((a + b) / 2).toFixed(1)},${cy.toFixed(1)} ${b.toFixed(1)},${botY}" fill="none" stroke="${col}" stroke-width="1" stroke-opacity="0.28"/>`);
      p.push(`<circle cx="${a.toFixed(1)}" cy="${topY}" r="1.3" fill="${col}" fill-opacity="0.7"/>`);
      p.push(`<circle cx="${b.toFixed(1)}" cy="${botY}" r="1.3" fill="${col}" fill-opacity="0.7"/>`);
    }
    note = `${subN} submitted in-window (→ incl. later) · ${incN} included in-window (← subm. earlier)`
      + (state.flowLane !== "all" ? ` · ${state.flowLane} only` : "");
  }
  p.push(`<text x="${x1}" y="13" font-size="9" fill="${t.text}" text-anchor="end">${note}</text>`);
  fig.insertAdjacentHTML("beforeend",
    `<svg viewBox="0 0 ${W} ${H}" width="100%" height="${H}" style="color:${t.text}">${p.join("")}</svg>`);
}

function focusXMapping() {
  // all focus panels share the same x domain + left/right margins + width,
  // so one mapping (taken from the price panel's svg) applies to all.
  const svg = el("panel-price").querySelector("svg");
  if (!svg) return null;
  const x = svg.scale("x");
  const rect = el("focus").getBoundingClientRect();
  const svgRect = svg.getBoundingClientRect();
  const offsetLeft = svgRect.left - rect.left;        // svg position within #focus
  const [r0, r1] = x.range, [d0, d1] = x.domain;
  return {
    pxToSlot: (clientX) => {
      const local = clientX - svgRect.left;
      return d0 + ((local - r0) / (r1 - r0)) * (d1 - d0);
    },
    slotToLocal: (s) => offsetLeft + r0 + ((s - d0) / (d1 - d0)) * (r1 - r0),
    inRange: (clientX) => {
      const local = clientX - svgRect.left;
      return local >= r0 && local <= r1;
    },
  };
}

function setupFocusInteractions() {
  // One column-wide interaction over all the time-aligned panels (incl. the flow panel):
  //  - hover  -> synced crosshair line + slot readout
  //  - drag   -> select a submit-slot window that drives the submission->inclusion panel
  //  - click  -> clear the selection
  const focus = el("focus");
  const line = el("crosshair"), readout = el("crosshair-readout"), box = el("cohort-sel");
  let dragging = false, startSlot = null, startClientX = 0;
  const onPlot = (target) => !target.closest("button, .legend, .panel-head");

  focus.addEventListener("pointerdown", (ev) => {
    const m = focusXMapping();
    if (!m || !m.inRange(ev.clientX) || !onPlot(ev.target)) return;
    ev.preventDefault();
    dragging = true; startClientX = ev.clientX; startSlot = m.pxToSlot(ev.clientX);
    focus.setPointerCapture && focus.setPointerCapture(ev.pointerId);
  });

  focus.addEventListener("pointermove", (ev) => {
    const m = focusXMapping();
    if (!m) return;
    if (m.inRange(ev.clientX)) {
      const slot = Math.round(m.pxToSlot(ev.clientX)), lx = m.slotToLocal(slot);
      line.style.left = `${lx}px`; line.style.height = `${focus.clientHeight}px`; line.style.display = "block";
      const px = DATA.meta.lanes.map((l) => { const v = priceAt(l, slot); return `${l[0]} ${v == null ? "—" : v.toFixed(2)}`; }).join(" · ");
      readout.style.left = `${lx + 4}px`;
      readout.textContent = `slot ${slot} · price ${px}`;
      readout.style.display = "block";
    } else { line.style.display = "none"; readout.style.display = "none"; }
    if (dragging) {
      const a = m.slotToLocal(startSlot), b = m.slotToLocal(m.pxToSlot(ev.clientX));
      box.style.left = `${Math.min(a, b)}px`; box.style.width = `${Math.abs(b - a)}px`;
      box.style.height = `${focus.clientHeight}px`; box.style.display = "block";
    }
  });

  focus.addEventListener("pointerup", (ev) => {
    if (!dragging) return;
    dragging = false;
    const m = focusXMapping();
    if (m && Math.abs(ev.clientX - startClientX) > 3) {
      const end = m.pxToSlot(ev.clientX);
      state.flowSel = [Math.min(startSlot, end), Math.max(startSlot, end)];
    } else {
      state.flowSel = null;   // a click (no drag) clears the selection
    }
    renderFlow();
    positionCohortSel();
  });

  focus.addEventListener("pointerleave", () => {
    line.style.display = "none"; readout.style.display = "none";
  });
}

function switchRun(i) {
  const next = (i + RUNS.length) % RUNS.length;
  if (next === runIndex) return;
  const sameLength = RUNS[next].data.meta.slotCount === DATA.meta.slotCount;
  runIndex = next;
  DATA = RUNS[next].data;
  classColors = computeClassColors();
  // a brushed slot window only carries over between equal-length runs
  if (!sameLength) { state.xDomain = null; state.flowSel = null; }
  el("run-select").value = String(next);
  renderAll();
}

function setupRunNav() {
  if (RUNS.length < 2) return; // single run: keep the header clean
  el("run-nav").hidden = false;
  const select = el("run-select");
  RUNS.forEach((run, i) => {
    const option = document.createElement("option");
    option.value = String(i);
    option.textContent = run.name;
    select.appendChild(option);
  });
  select.onchange = () => switchRun(+select.value);
  el("run-prev").onclick = () => switchRun(runIndex - 1);
  el("run-next").onclick = () => switchRun(runIndex + 1);
  document.addEventListener("keydown", (ev) => {
    if (/^(SELECT|INPUT|TEXTAREA)$/.test(ev.target.tagName)) return;
    if (ev.key === "[") switchRun(runIndex - 1);
    if (ev.key === "]") switchRun(runIndex + 1);
  });
}

function setupControls() {
  el("toggle-theme").onclick = () => {
    const root = document.documentElement;
    root.dataset.theme = root.dataset.theme === "dark" ? "light" : "dark";
    renderAll();                          // re-render so plot colors follow the theme
  };
  el("toggle-price-view").onclick = () => {
    state.priceView = state.priceView === "log" ? "perlane" : "log";
    el("toggle-price-view").textContent =
      "Price view: " + (state.priceView === "log" ? "overlaid (log)" : "per lane");
    renderFocus();
  };
  el("toggle-latency-by").onclick = () => {
    state.latencyBy = state.latencyBy === "lane" ? "class" : "lane";
    el("toggle-latency-by").textContent =
      "Latency by: " + (state.latencyBy === "lane" ? "lane" : "urgency class");
    renderLatencyTimePanel(); renderDistribution(); renderLatencyTable();
  };
  el("toggle-latency-x").onclick = () => {
    state.latencyX = state.latencyX === "submit" ? "incl" : "submit";
    el("toggle-latency-x").textContent =
      "Latency x: " + (state.latencyX === "submit" ? "submission" : "inclusion");
    renderLatencyTimePanel();
  };
  el("toggle-flow-lane").onclick = () => {
    state.flowLane = state.flowLane === "all" ? "Priority" : state.flowLane === "Priority" ? "Standard" : "all";
    el("toggle-flow-lane").textContent = "Flow: " + (state.flowLane === "all" ? "all lanes" : state.flowLane);
    renderFlow();
  };
  el("toggle-compose").onclick = () => {
    state.composeBy = state.composeBy === "laneclass" ? "aggregate" : "laneclass";
    el("toggle-compose").textContent = "Compose: " + (state.composeBy === "laneclass" ? "lane×class" : "aggregate");
    renderFate(); renderValue();
  };
  el("toggle-p95").onclick = () => {
    state.p95Band = !state.p95Band;
    el("toggle-p95").textContent = "p95 band: " + (state.p95Band ? "on" : "off");
    renderFocus();
  };
  el("toggle-conv-band").onclick = () => {
    state.convBand = !state.convBand;
    el("toggle-conv-band").textContent = "Conv. band: " + (state.convBand ? "on" : "off");
    renderFocus();
  };
}

function renderAll() {
  renderHeader();
  renderKpis();
  renderLatencyTable();
  // panels added in later tasks:
  if (typeof renderFocus === "function") renderFocus();   // includes the flow panel + cohort rect
  if (typeof renderContext === "function") renderContext();
  if (typeof renderDistribution === "function") renderDistribution();
  if (typeof renderRb === "function") renderRb();
  if (typeof renderFate === "function") renderFate();
  if (typeof renderValue === "function") renderValue();
  if (typeof renderFairness === "function") renderFairness();
}

setupRunNav();
setupControls();
renderAll();
setupFocusInteractions();

// Re-fit charts to the window when it resizes (debounced).
let _resizeTimer;
window.addEventListener("resize", () => {
  clearTimeout(_resizeTimer);
  _resizeTimer = setTimeout(renderAll, 150);
});
