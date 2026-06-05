"use strict";
const DATA = window.SIM_DATA;

const state = {
  priceView: "log",     // "log" | "perlane"
  p95Band: true,
  xDomain: null,        // null = full run
  hiddenLanes: new Set(),
  hiddenClasses: new Set(),
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

function renderLatencyTable() {
  const rows = DATA.meta.urgencyClasses.map((c) => {
    const s = DATA.latency.byClass[c.id];
    return `<tr><td style="color:${classColors[c.id]}">${c.label}</td>
      <td>${s.median}</td><td>${s.p95}</td><td>${s.max}</td><td>${s.count.toLocaleString()}</td></tr>`;
  }).join("");
  el("latency-table").innerHTML =
    `<table class="lat"><thead><tr><th>class</th><th>med</th><th>p95</th><th>max</th><th>n</th></tr></thead>
     <tbody>${rows}</tbody></table>`;
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
    width: focusWidth(), height: 170, marginLeft: 44, marginRight: 12, marginBottom: 18,
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
      width: focusWidth(), height: 110, marginLeft: 44, marginRight: 12, marginBottom: 16,
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
    state.priceView === "log" ? "log axis · ±5% convergence band" : "per lane · ±5% convergence band",
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
    width: focusWidth(), height: 110, marginLeft: 44, marginRight: 12, marginBottom: 16,
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

function classLegend(figureId) {
  const div = document.createElement("div");
  div.className = "legend";
  DATA.meta.urgencyClasses.forEach((c) => {
    const item = document.createElement("span");
    item.className = "item" + (state.hiddenClasses.has(c.id) ? " off" : "");
    item.innerHTML = `<span class="swatch" style="background:${classColors[c.id]}"></span>${c.label}`;
    item.onclick = () => {
      state.hiddenClasses.has(c.id) ? state.hiddenClasses.delete(c.id) : state.hiddenClasses.add(c.id);
      renderFocus(); renderDistribution();
    };
    div.appendChild(item);
  });
  el(figureId).appendChild(div);
}

function renderLatencyTimePanel() {
  const t = theme();
  const fig = el("panel-latency");
  fig.innerHTML = "";
  panelHead("panel-latency", "Latency / urgency class",
    "median, slots · " + (state.p95Band ? "median→p95 band on" : "median only"),
    "latency-time.svg");
  classLegend("panel-latency");
  const classes = DATA.meta.urgencyClasses.filter((c) => !state.hiddenClasses.has(c.id));
  const marks = [Plot.gridY({ stroke: t.grid })];
  if (state.p95Band) {
    classes.forEach((c) => marks.push(Plot.areaY(DATA.latency.byClass[c.id].overTime, {
      x: "slot", y1: "median", y2: "p95", fill: classColors[c.id], fillOpacity: 0.1, curve: "monotone-x",
    })));
  }
  classes.forEach((c) => marks.push(Plot.line(DATA.latency.byClass[c.id].overTime, {
    x: "slot", y: "median", stroke: classColors[c.id], strokeWidth: 2.6, curve: "monotone-x",
  })));
  const node = Plot.plot({
    width: focusWidth(), height: 190, marginLeft: 44, marginRight: 12, marginBottom: 26,
    style: { color: t.text, fontSize: "11px" },
    x: { domain: xDomain(), label: "slot →" },
    y: { grid: false, label: "latency (slots) ↑" },
    marks,
  });
  fig.appendChild(node);
}

function renderFocus() {
  renderPricePanel();
  renderShockPanel();
  renderLatencyTimePanel();
}

function renderDistribution() {
  const t = theme();
  const fig = el("panel-dist");
  fig.innerHTML = "";
  panelHead("panel-dist", "Latency distribution", "IQR · median · p95 · max", "latency-dist.svg");
  const classes = DATA.meta.urgencyClasses.filter((c) => !state.hiddenClasses.has(c.id));
  const rows = classes.map((c) => {
    const s = DATA.latency.byClass[c.id];
    return { id: c.id, label: c.label, color: classColors[c.id],
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
  el("toggle-p95").onclick = () => {
    state.p95Band = !state.p95Band;
    el("toggle-p95").textContent = "p95 band: " + (state.p95Band ? "on" : "off");
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
