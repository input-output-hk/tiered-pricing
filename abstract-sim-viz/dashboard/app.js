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

// ordered blue ramp: index 0 (lowest rate) darkest
const classColors = (() => {
  const n = DATA.meta.urgencyClasses.length;
  const ramp = d3.quantize(d3.interpolateBlues, Math.max(n, 2)).reverse();
  const map = {};
  DATA.meta.urgencyClasses.forEach((c, i) => { map[c.id] = ramp[i] || "#3182bd"; });
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
    width: 760, height: 150, marginLeft: 44, marginRight: 12, marginBottom: 18,
    style: { color: t.text, fontSize: "10px" },
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
      width: 760, height: 90, marginLeft: 44, marginRight: 12, marginBottom: 16,
      style: { color: t.text, fontSize: "10px" },
      x: { domain: xDomain(), axis: null },
      y: { grid: true, label: `${lane} ↑` },
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
    width: 760, height: 90, marginLeft: 44, marginRight: 12, marginBottom: 16,
    style: { color: t.text, fontSize: "10px" },
    x: { domain: xDomain(), axis: null },
    y: { grid: true, label: "jump ↑", percent: false },
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
      x: "slot", y1: "median", y2: "p95", fill: classColors[c.id], fillOpacity: 0.12, curve: "monotone-x",
    })));
  }
  classes.forEach((c) => marks.push(Plot.line(DATA.latency.byClass[c.id].overTime, {
    x: "slot", y: "median", stroke: classColors[c.id], strokeWidth: 1.6, curve: "monotone-x",
  })));
  const node = Plot.plot({
    width: 760, height: 130, marginLeft: 44, marginRight: 12, marginBottom: 26,
    style: { color: t.text, fontSize: "10px" },
    x: { domain: xDomain(), label: "slot →" },
    y: { grid: false, label: "latency ↑" },
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
    width: 230, height: 260, marginLeft: 36, marginBottom: 50, marginRight: 8,
    style: { color: t.text, fontSize: "10px" },
    x: { domain: rows.map((r) => r.label), label: null, tickRotate: -30 },
    y: { grid: true, label: "latency ↑" },
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

function renderAll() {
  renderHeader();
  renderKpis();
  renderLatencyTable();
  // panels added in later tasks:
  if (typeof renderFocus === "function") renderFocus();
  if (typeof renderContext === "function") renderContext();
  if (typeof renderDistribution === "function") renderDistribution();
}

renderAll();
