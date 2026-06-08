import json
import math
import random
from datetime import datetime, timezone

from simviz import price as price_mod
from simviz import load as load_mod
from simviz import latency as latency_mod
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
    keys = {(m["tag"], m["rate"]) for m in acc.tx_meta.values()}
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

    present = set(acc.price_changes.keys())
    lanes = [l for l in ["Standard", "Priority"] if l in present] or sorted(present)
    classes = urgency_classes(acc, f)

    submitted_by_lane = {}
    for m in acc.tx_meta.values():
        submitted_by_lane[m["lane"]] = submitted_by_lane.get(m["lane"], 0) + 1

    price_by_lane = {lane: price_mod.price_series(acc, lane) for lane in lanes}
    shock_by_lane = {
        lane: price_mod.shock_stats(price_by_lane[lane], params["shockThreshold"])
        for lane in lanes
    }

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
            "oscillationAmplitude": price_mod.oscillation_amplitude(price_by_lane[lane]),
            "regimes": regime_results,
        }

    grouped_class = latency_mod.join_latencies(acc)
    grouped_lane = latency_mod.join_latencies_by_lane(acc)
    all_lat = [lat for pairs in grouped_class.values() for (_, lat) in pairs]
    bin_w = _shared_bin_width(all_lat)  # shared across class AND lane groupings for comparability

    def stats_for(grouped, keys):
        # iterate the known keys so every class/lane appears (zero-filled if no inclusions)
        out = {}
        for key in keys:
            pairs = grouped.get(key, [])
            lats = [lat for (_, lat) in pairs]
            s = latency_mod.class_stats(lats)
            s["histogram"] = {"binWidth": bin_w, "bins": histogram_bins(lats, bin_w)}
            s["overTime"] = latency_mod.over_time(pairs, width, slot_count)  # by submission slot
            s["overTimeIncl"] = latency_mod.over_time(   # by inclusion slot (submit + latency)
                [(sub + lat, lat) for (sub, lat) in pairs], width, slot_count)
            out[key] = s
        return out

    latency_by_class = stats_for(grouped_class, [c["id"] for c in classes])
    latency_by_lane = stats_for(grouped_lane, lanes)

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
            "submittedByLane": submitted_by_lane,      # for drop-rate KPIs
            "urgencyClasses": classes,
        },
        "params": params,
        "price": {"byLane": price_by_lane},
        "shock": {"byLane": shock_by_lane},
        "convergence": {"loadRegimes": regimes, "byLane": conv_by_lane},
        "latency": {"byClass": latency_by_class, "byLane": latency_by_lane},
        "load": load_obj,
        "flow": build_flow_sample(acc),
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
