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
