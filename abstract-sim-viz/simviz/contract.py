import json
import math
import os
import random
import re
from datetime import datetime, timezone

from simviz import price as price_mod
from simviz import load as load_mod
from simviz import latency as latency_mod
from simviz.ingest import unit_fate, unit_lane
from simviz.stats import quantile, histogram_bins

DEFAULT_PARAMS = {"shockThreshold": 0.10, "convergenceBandPct": 0.05, "loadChangePct": 0.10}


def half_life_blocks(tag, rate):
    """Ranking blocks for a tx's retained value to fall to 50%, from
    Transaction.retentionRatio. The urgency rate is per expected ranking block, so
    the half-life is native in blocks (no slot conversion). None if undefined
    (rate <= 0).

    Exponential: value = exp(-rate * blocks) -> 50% at ln(2)/rate.
    Linear:      value = 1 - rate * blocks   -> 50% at 0.5/rate.
    """
    if rate is None or rate <= 0:
        return None
    if tag == "Exponential":
        return math.log(2) / rate
    if tag == "Linear":
        return 0.5 / rate
    return None


def _fmt_halflife(value):
    return f"{value:.0f}" if value >= 10 else f"{value:.1f}"


def urgency_label(hl_blocks, hl_slots):
    """Class label as value half-life: blocks (native) when defined, else slots."""
    if hl_blocks is not None:
        return f"t½≈{_fmt_halflife(hl_blocks)} blk"
    if hl_slots is not None:
        return f"t½≈{_fmt_halflife(hl_slots)} sl"
    return "t½ n/a"


def urgency_classes(acc, f=None):
    """Distinct urgency classes present, ordered by rate low -> high, labelled by
    value half-life in blocks (rate is per expected ranking block). The slot
    equivalent uses the active-slot coefficient f, mirroring expectedBlockDelay:
    blocks = f * slots, so slots = blocks / f."""
    keys = {(u["meta"]["tag"], u["meta"]["rate"]) for u in acc.units.values()}
    classes = []
    for tag, rate in sorted(keys, key=lambda k: k[1]):
        hl_blocks = half_life_blocks(tag, rate)
        hl_slots = (hl_blocks / f) if (hl_blocks is not None and f) else None
        classes.append({
            "id": latency_mod.class_id(tag, rate),
            "tag": tag, "rate": rate,
            "halfLifeBlocks": hl_blocks,
            "halfLifeSlots": hl_slots,
            "label": urgency_label(hl_blocks, hl_slots),
        })
    return classes


def _shared_bin_width(all_latencies):
    if not all_latencies:
        return 1
    p99 = quantile(0.99, sorted(all_latencies))
    return max(1, math.ceil(p99 / 30))


def retention_ratio(tag, rate, blocks):
    """Fraction of value retained after `blocks` block-delay, per Transaction.retentionRatio."""
    b = max(0.0, blocks)
    if tag == "Exponential":
        return math.exp(-rate * b)
    if tag == "Linear":
        return max(0.0, 1.0 - rate * b)
    return 1.0


def build_fate(acc, lanes, classes):
    """Per demand unit (one count however many attempts it took): included /
    abandoned (terminal failure: actor gave up after rejection or eviction) /
    unresolved (still in flight at the run horizon), tallied by lane, by
    urgency class, and by lane x class. Units are attributed to the lane that
    served them, else the last lane attempted."""
    cats = ("submitted", "included", "abandoned", "unresolved")
    def blank():
        return {k: 0 for k in cats}
    by_lane = {l: blank() for l in lanes}
    by_class = {c["id"]: blank() for c in classes}
    by_class_lane = {c["id"]: {l: blank() for l in lanes} for c in classes}
    for unit in acc.units.values():
        fate = unit_fate(acc, unit)
        meta = unit["meta"]
        cid, lane = latency_mod.class_id(meta["tag"], meta["rate"]), unit_lane(unit)
        for d in (by_lane.get(lane), by_class.get(cid), by_class_lane.get(cid, {}).get(lane)):
            if d is not None:
                d["submitted"] += 1
                d[fate] += 1
    return {"byLane": by_lane, "byClass": by_class, "byClassLane": by_class_lane}


def build_value(acc, lanes, classes, f):
    """Retained vs lost demand-unit value by lane and urgency class. Every unit
    lands in exactly one column: served units retain value *
    retentionRatio(f * latency from FIRST submission) — so retry wait counts —
    abandoned units lose their full value, and units still in flight at the
    run horizon are reported as unresolved rather than lost."""
    def blank():
        return {"total": 0, "retained": 0, "lost": 0, "unresolved": 0}
    by_lane = {l: blank() for l in lanes}
    by_class = {c["id"]: blank() for c in classes}
    by_class_lane = {c["id"]: {l: blank() for l in lanes} for c in classes}
    for unit in acc.units.values():
        v = unit["value"] or 0
        meta = unit["meta"]
        fate = unit_fate(acc, unit)
        ret = lost = unresolved = 0
        if fate == "included":
            blocks = f * max(0, unit["includedAt"] - unit["firstSubmitted"])
            r = retention_ratio(meta["tag"], meta["rate"], blocks)
            ret, lost = round(v * r), round(v * (1.0 - r))
        elif fate == "abandoned":
            lost = v
        else:
            unresolved = v
        cid, lane = latency_mod.class_id(meta["tag"], meta["rate"]), unit_lane(unit)
        for d in (by_lane.get(lane), by_class.get(cid), by_class_lane.get(cid, {}).get(lane)):
            if d is not None:
                d["total"] += v
                d["retained"] += ret
                d["lost"] += lost
                d["unresolved"] += unresolved
    cells = [*by_lane.values(), *by_class.values()]
    for per_lane in by_class_lane.values():
        cells.extend(per_lane.values())
    for d in cells:
        d["retainedPct"] = (100.0 * d["retained"] / d["total"]) if d["total"] else 0.0
    return {"byLane": by_lane, "byClass": by_class, "byClassLane": by_class_lane}


def build_fairness(acc):
    """Fairness/starvation over demand units: Jain's index over per-actor
    served-unit counts; starvedTxs = units still in flight at the run horizon
    (never served, never abandoned)."""
    sub_by_actor, inc_by_actor = {}, {}
    starved = 0
    for unit in acc.units.values():
        a = unit["actor"]
        fate = unit_fate(acc, unit)
        if fate == "unresolved":
            starved += 1
        if a is None:
            continue
        sub_by_actor[a] = sub_by_actor.get(a, 0) + 1
        if fate == "included":
            inc_by_actor[a] = inc_by_actor.get(a, 0) + 1
    actors = sorted(sub_by_actor)
    counts = [inc_by_actor.get(a, 0) for a in actors]
    s, ss, n = sum(counts), sum(c * c for c in counts), len(counts)
    jain = (s * s) / (n * ss) if (n and ss) else 1.0
    return {
        "jainIndex": jain,
        "nActors": n,
        "starvedTxs": starved,
        "actors": [
            {"id": a, "submitted": sub_by_actor[a], "included": inc_by_actor.get(a, 0),
             "rate": (inc_by_actor.get(a, 0) / sub_by_actor[a]) if sub_by_actor[a] else 0.0}
            for a in actors
        ],
    }


def build_flow_sample(acc, cap=15000, seed=0):
    """Per-tx submission->inclusion links for the brush-to-link panel, as compact
    [submitSlot, inclusionSlot, routeCode (0=RB,1=EB), laneCode (0=Standard,1=Priority)].

    Sampled UNIFORMLY (every route/lane at the same rate), so on-screen proportions
    match reality and stay consistent with the RB-content panel. RB-route inclusions
    are genuinely rare (~2% of txs), so green arcs will be sparse — that is correct.
    """
    pairs = []
    for tx_id, submit in acc.submitted_at.items():
        inc = acc.included_at.get(tx_id)
        meta = acc.tx_meta.get(tx_id)
        route = acc.included_route.get(tx_id)
        if inc is None or meta is None or route is None:
            continue
        lane_code = 1 if meta["lane"] == "Priority" else 0
        pairs.append([submit, inc, (0 if route == "IncludedInRb" else 1), lane_code])
    total = len(pairs)
    links = pairs if total <= cap else random.Random(seed).sample(pairs, cap)
    links.sort(key=lambda r: r[0])
    return {
        "links": links,
        "total": total,
        "sampleRate": (len(links) / total) if total else 0.0,
    }


def build_sim_data(acc, params=None, target_buckets=300, source="events.jsonl", f=0.05):
    params = {**DEFAULT_PARAMS, **(params or {})}
    slot_count = acc.slot_count
    width = load_mod.bucket_width(slot_count, target_buckets)
    # Slots <-> blocks uses the configured active-slot coefficient f (expected),
    # matching the sim's expectedBlockDelay (blocks = f * slots). The realized RB
    # cadence is kept only as a sanity check, not used for conversions.
    expected_spb = (1.0 / f) if f else None
    realized_spb = (slot_count / acc.rb_count) if acc.rb_count else None

    classes = urgency_classes(acc, f)

    submitted_by_lane = {}
    fates = []
    for unit in acc.units.values():
        lane = unit_lane(unit)
        submitted_by_lane[lane] = submitted_by_lane.get(lane, 0) + 1
        fates.append(unit_fate(acc, unit))

    # Every lane that saw traffic or a price update: a static lane (e.g. the
    # standard lane of a priority-only design) re-prices never but still
    # carries transactions, so latency/fate/KPI views must include it.
    present = set(acc.price_changes.keys()) | set(submitted_by_lane.keys())
    lanes = [l for l in ["Standard", "Priority"] if l in present] or sorted(present)
    n_units = len(acc.units)
    demand = {
        "units": n_units,
        "attempts": acc.attempt_count,
        "served": fates.count("included"),
        "abandoned": fates.count("abandoned"),
        "unresolved": fates.count("unresolved"),
        "amplification": (acc.attempt_count / n_units) if n_units else 0.0,
        "attemptsMax": acc.attempts_max,
    }

    price_by_lane = {lane: price_mod.price_series(acc, lane) for lane in lanes}
    shock_by_lane = {
        lane: price_mod.shock_stats(price_by_lane[lane], params["shockThreshold"])
        for lane in lanes
    }
    oscillation_by_lane = {}
    for lane in lanes:
        oscillation_by_lane[lane] = price_mod.oscillation_stats(
            price_by_lane[lane], params["convergenceBandPct"])
        oscillation_by_lane[lane]["reversals"] = price_mod.oscillation_reversals(
            price_by_lane[lane], params["convergenceBandPct"])

    rate = load_mod.smooth_rate(acc.submissions_per_slot, slot_count, width)
    regimes = load_mod.detect_regimes(rate, params["loadChangePct"])
    load_obj = {
        "bucketWidth": width,
        "buckets": load_mod.load_buckets(
            acc.submissions_per_slot, acc.inclusions_per_slot, slot_count, width),
    }

    conv_by_lane = {}
    for lane in lanes:
        regime_results, conv_time = price_mod.convergence_for_lane(
            price_by_lane[lane], regimes, params["convergenceBandPct"])
        conv_by_lane[lane] = {
            "convergenceTime": conv_time,
            "settledCoefficientRange": price_mod.settled_coefficient_range(
                price_by_lane[lane], params["convergenceBandPct"]),
            "regimes": regime_results,
        }

    grouped_class = latency_mod.join_latencies(acc)
    grouped_lane = latency_mod.join_latencies_by_lane(acc)
    grouped_block_class = latency_mod.join_block_latencies(acc)
    grouped_block_lane = latency_mod.join_block_latencies_by_lane(acc)
    grouped_block_class_incl = latency_mod.join_block_latencies_by_inclusion(acc)
    grouped_block_lane_incl = latency_mod.join_block_latencies_by_lane_inclusion(acc)
    all_lat = [lat for pairs in grouped_class.values() for (_, lat) in pairs]
    bin_w = _shared_bin_width(all_lat)  # shared across class AND lane groupings for comparability

    def stats_for(grouped, grouped_blocks, grouped_blocks_incl, keys):
        # iterate the known keys so every class/lane appears (zero-filled if no inclusions)
        out = {}
        for key in keys:
            pairs = grouped.get(key, [])
            lats = [lat for (_, lat) in pairs]
            block_pairs = grouped_blocks.get(key, [])
            block_lats = [lat for (_, lat) in block_pairs]
            s = latency_mod.class_stats(lats)
            s["blocks"] = latency_mod.class_stats(block_lats)
            s["histogram"] = {"binWidth": bin_w, "bins": histogram_bins(lats, bin_w)}
            s["overTime"] = latency_mod.over_time(pairs, width, slot_count)  # by submission slot
            s["overTimeIncl"] = latency_mod.over_time(   # by inclusion slot (submit + latency)
                [(sub + lat, lat) for (sub, lat) in pairs], width, slot_count)
            s["overTimeBlocks"] = latency_mod.over_time(block_pairs, width, slot_count)
            s["overTimeInclBlocks"] = latency_mod.over_time(
                grouped_blocks_incl.get(key, []), width, slot_count)
            out[key] = s
        return out

    latency_by_class = stats_for(
        grouped_class, grouped_block_class, grouped_block_class_incl, [c["id"] for c in classes])
    latency_by_lane = stats_for(
        grouped_lane, grouped_block_lane, grouped_block_lane_incl, lanes)

    return {
        "meta": {
            "source": source,
            "generatedAt": datetime.now(timezone.utc).isoformat(),
            "slotCount": slot_count,
            "totalEvents": acc.total_events,
            "f": f,
            "expectedSlotsPerBlock": expected_spb,    # 1/f, used for slot<->block conversion
            "rbCount": acc.rb_count,
            "realizedSlotsPerBlock": realized_spb,     # sanity check only
            "lanes": lanes,
            "submittedByLane": submitted_by_lane,      # demand units, for drop-rate KPIs
            "urgencyClasses": classes,
            "demand": demand,                          # demand units vs attempts (retry load)
        },
        "params": params,
        "price": {"byLane": price_by_lane},
        "shock": {"byLane": shock_by_lane},
        "oscillation": {"byLane": oscillation_by_lane},
        "convergence": {"loadRegimes": regimes, "byLane": conv_by_lane},
        "latency": {"byClass": latency_by_class, "byLane": latency_by_lane},
        "load": load_obj,
        "flow": build_flow_sample(acc),
        "fate": build_fate(acc, lanes, classes),
        "value": build_value(acc, lanes, classes, f),
        "fairness": build_fairness(acc),
        "blocks": {
            "rbTotal": acc.rb_count,
            "rbWithTxs": acc.rb_tx_count,     # RBs carrying transactions (PraosBlock)
            "rbWithCert": acc.rb_cert_count,  # RBs certifying an EB (CertifyingBlock)
            "rbSeries": acc.rb_series,        # [{slot, kind}] per RB, in order (over-time strip)
        },
    }


def write_data_js(sim_data, path):
    """Serialise SIM_DATA as a JS global so the dashboard works from file://."""
    payload = json.dumps(sim_data, separators=(",", ":"))
    with open(path, "w") as fh:
        fh.write("window.SIM_DATA = " + payload + ";\n")


def run_name(path):
    """A short display name for a trace: its basename minus trace suffixes
    (two-lane-reserved-seed0.events.jsonl -> two-lane-reserved-seed0)."""
    name = os.path.basename(path)
    for suffix in (".events.jsonl", ".jsonl"):
        if name.endswith(suffix):
            return name[: -len(suffix)]
    return name


def run_names(paths):
    """Per-trace display names; basename collisions fall back to the path
    relative to the traces' common prefix, so every name stays unique."""
    names = [run_name(p) for p in paths]
    if len(set(names)) == len(names):
        return names
    common = os.path.commonpath([os.path.abspath(p) for p in paths])
    return [
        os.path.relpath(os.path.abspath(os.path.splitext(p)[0]), common).replace(os.sep, "/")
        for p in paths
    ]


def split_seed(name):
    """Split a sweep-style run name into (variant, seed): the dashboard
    offers them as separate selectors. Names without a -seed<N> suffix are
    their own single-run variant."""
    match = re.fullmatch(r"(.+)-seed(\d+)", name)
    if match:
        return match.group(1), int(match.group(2))
    return name, None


def write_runs_js(runs, path):
    """Serialise [(name, sim_data)] as the dashboard's run bundle. SIM_DATA
    aliases the first run so anything reading the old single-run global still
    works."""
    payload = json.dumps(
        [
            {"name": name, "variant": variant, "seed": seed, "data": sim_data}
            for name, sim_data in runs
            for variant, seed in [split_seed(name)]
        ],
        separators=(",", ":"))
    with open(path, "w") as fh:
        fh.write("window.SIM_RUNS = " + payload + ";\n")
        fh.write("window.SIM_DATA = window.SIM_RUNS[0].data;\n")
