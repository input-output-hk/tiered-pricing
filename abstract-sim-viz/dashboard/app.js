"use strict";
const DATA = window.SIM_DATA;

const state = {
  priceView: "log",     // "log" | "perlane"
  latencyBy: "lane",    // "lane" | "class" — lane answers "does Priority serve faster?"
  p95Band: true,
  convBand: true,       // show ±5% convergence bands on the price panel
  xDomain: null,        // null = full run
  hiddenLanes: new Set(),
  hiddenClasses: new Set(),
  hiddenLatLanes: new Set(),
};

const LANE_COLOR = { Standard: "#2563eb", Priority: "#7c3aed" };

// Ordered, distinguishable palette for urgency classes (low → high decay rate).
// Mid-lightness hues that read on both light and dark backgrounds and avoid the
// lane colors (blue/purple) and the shock red. Falls back to a generated ramp if a
// run ever has more classes than the curated set.
const classColors = (() => {
  const classes = DATA.meta.urgencyClasses;
  const base = ["#0d9488", "#22c55e", "#f59e0b", "#db2777"]; // teal → green → amber → pink
  const ramp = classes.length <= base.length
    ? base
    : d3.quantize((t) => d3.interpolateTurbo(0.1 + 0.8 * t), classes.length);
  const map = {};
  classes.forEach((c, i) => { map[c.id] = ramp[i] || base[0]; });
  return map;
})();

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

// Responsive widths: charts fill their column instead of a fixed 760px.
function focusWidth() { return Math.max(360, el("focus").clientWidth || 760); }
function distWidth() { return Math.max(200, el("panel-dist").clientWidth || 230); }
// Shared right margin for all focus panels so they align on the slot axis (the
// latency panel reserves room here for its right-hand blocks axis).
function focusRight() { return DATA.meta.expectedSlotsPerBlock ? 48 : 12; }

function panelHead(figureId, titleHtml, hintHtml, exportName) {
  const fig = el(figureId);
  const head = document.createElement("div");
  head.className = "panel-head";
  head.innerHTML = `<div><h2>${titleHtml}</h2><span class="hint">${hintHtml}</span></div>`;
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
  el("subtitle").textContent =
    `${DATA.meta.slotCount.toLocaleString()} slots · ` +
    `${DATA.meta.totalEvents.toLocaleString()} events · source ${DATA.meta.source}`;
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
  el("kpis").innerHTML = [
    kpi("Priority conv. time", prio.convergenceTime == null ? "—" : `${prio.convergenceTime} slots`, "#7c3aed"),
    kpi("Max price jump", `${(maxJump * 100).toFixed(1)}%`, "#f59e0b"),
    kpi("# shocks (>10%)", String(shocks), "#ef4444"),
    kpi("Std oscillation", `±${fmt(std.oscillationAmplitude, 3)}`, "#2563eb"),
  ].join("");
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
  return Plot.plot({
    width: focusWidth(), height: 170, marginLeft: 44, marginRight: focusRight(), marginBottom: 18,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), axis: null },
    y: { type: "log", grid: false, label: "coeff ↑", ticks: [1, 2, 4, 8, 16] },
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
      y: { grid: false, label: `${lane} ↑` },
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
    "price.svg");
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
    "shock.svg");
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
  panelHead("panel-latency", g.title,
    (spb ? "median · slots (left) · expected blocks (right)" : "median · slots")
      + " · " + (state.p95Band ? "median→p95 band" : "median only"),
    "latency-time.svg");
  latencyLegend("panel-latency", g);
  const items = g.items.filter((it) => !g.hidden.has(it.id));
  const marks = [Plot.gridY({ stroke: t.grid })];
  if (state.p95Band) {
    items.forEach((it) => marks.push(Plot.areaY(g.data[it.id].overTime, {
      x: "slot", y1: "median", y2: "p95", fill: it.color, fillOpacity: 0.1, curve: "monotone-x",
    })));
  }
  items.forEach((it) => marks.push(Plot.line(g.data[it.id].overTime, {
    x: "slot", y: "median", stroke: it.color, strokeWidth: 2.6, curve: "monotone-x",
  })));
  if (spb) {
    marks.push(Plot.axisY({ anchor: "right", tickFormat: (d) => (d / spb).toFixed(1), label: "blocks ↑" }));
  }
  const node = Plot.plot({
    width: focusWidth(), height: 190, marginLeft: 44, marginRight: focusRight(), marginBottom: 26,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), label: "slot →" },
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
    "each ranking block · txs (green) vs cert (amber) · runs show as solid stretches",
    "rb-time.svg");
  const series = (DATA.blocks || {}).rbSeries || [];
  if (!series.length) {
    const p = document.createElement("div");
    p.className = "subtitle"; p.style.fontSize = "10px";
    p.textContent = "no ranking blocks in trace";
    fig.appendChild(p);
    return;
  }
  const TX = "#16a34a", CERT = "#d97706";
  // each RB occupies the span until the next RB, so adjacent same-kind blocks read as one run
  const segs = series.map((d, i) => ({
    x1: d.slot,
    x2: i + 1 < series.length ? series[i + 1].slot : DATA.meta.slotCount,
    kind: d.kind,
  }));
  const node = Plot.plot({
    width: focusWidth(), height: 34, marginLeft: 44, marginRight: focusRight(),
    marginTop: 2, marginBottom: 4,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), axis: null },
    y: { axis: null, domain: [0, 1] },
    color: { domain: ["txs", "cert"], range: [TX, CERT] },
    marks: [Plot.rect(segs, { x1: "x1", x2: "x2", y1: 0, y2: 1, fill: "kind" })],
  });
  fig.appendChild(node);
}

function renderFocus() {
  renderPricePanel();
  renderShockPanel();
  renderRbTime();
  renderLatencyTimePanel();
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

const LOAD_DIMS = { width: 760, height: 70, marginLeft: 44, marginRight: 12, marginTop: 6, marginBottom: 18 };

function renderContext() {
  const t = theme();
  const fig = el("panel-load");
  fig.innerHTML = "";
  panelHead("panel-load", "Load (submissions/slot)", "brush to zoom the panels above · double-click to reset", "load.svg");
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

function setupCrosshair() {
  const focus = el("focus");
  const line = el("crosshair");
  const readout = el("crosshair-readout");
  focus.addEventListener("pointermove", (ev) => {
    const m = focusXMapping();
    if (!m || !m.inRange(ev.clientX)) { line.style.display = "none"; readout.style.display = "none"; return; }
    const slot = Math.round(m.pxToSlot(ev.clientX));
    const localX = m.slotToLocal(slot);
    line.style.left = `${localX}px`;
    line.style.height = `${focus.clientHeight}px`;
    line.style.display = "block";
    readout.style.left = `${localX + 4}px`;
    readout.textContent = `slot ${slot}`;
    readout.style.display = "block";
  });
  focus.addEventListener("pointerleave", () => {
    line.style.display = "none"; readout.style.display = "none";
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
  if (typeof renderFocus === "function") renderFocus();
  if (typeof renderContext === "function") renderContext();
  if (typeof renderDistribution === "function") renderDistribution();
  if (typeof renderRb === "function") renderRb();
}

setupControls();
renderAll();
setupCrosshair();

// Re-fit charts to the window when it resizes (debounced).
let _resizeTimer;
window.addEventListener("resize", () => {
  clearTimeout(_resizeTimer);
  _resizeTimer = setTimeout(renderAll, 150);
});
