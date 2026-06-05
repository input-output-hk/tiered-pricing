import json
import math
from datetime import datetime, timezone

from simviz import price as price_mod
from simviz import load as load_mod
from simviz import latency as latency_mod
from simviz.stats import quantile, histogram_bins

DEFAULT_PARAMS = {"shockThreshold": 0.10, "convergenceBandPct": 0.05, "loadChangePct": 0.10}


def half_life_slots(tag, rate):
    """Slots for a tx's retained value to fall to 50%, from the decay model in
    Transaction.retentionRatio. None if undefined (rate <= 0).

    Exponential: value = exp(-rate * slots) -> 50% at ln(2)/rate.
    Linear:      value = 1 - rate * slots   -> 50% at 0.5/rate.
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


def urgency_label(half_life_blocks, half_life_slots_val):
    """Class label as value half-life: blocks when block cadence is known, else slots."""
    if half_life_blocks is not None:
        return f"t½≈{_fmt_halflife(half_life_blocks)} blk"
    if half_life_slots_val is not None:
        return f"t½≈{_fmt_halflife(half_life_slots_val)} sl"
    return "t½ n/a"


def urgency_classes(acc, slots_per_block=None):
    """Distinct urgency classes present, ordered by rate low -> high, labelled by
    value half-life (in blocks when the block cadence is known)."""
    keys = {(m["tag"], m["rate"]) for m in acc.tx_meta.values()}
    classes = []
    for tag, rate in sorted(keys, key=lambda k: k[1]):
        hl_slots = half_life_slots(tag, rate)
        hl_blocks = (hl_slots / slots_per_block) if (hl_slots is not None and slots_per_block) else None
        classes.append({
            "id": latency_mod.class_id(tag, rate),
            "tag": tag, "rate": rate,
            "halfLifeSlots": hl_slots,
            "halfLifeBlocks": hl_blocks,
            "label": urgency_label(hl_blocks, hl_slots),
        })
    return classes


def _shared_bin_width(all_latencies):
    if not all_latencies:
        return 1
    p99 = quantile(0.99, sorted(all_latencies))
    return max(1, math.ceil(p99 / 30))


def build_sim_data(acc, params=None, target_buckets=300, source="events.jsonl"):
    params = {**DEFAULT_PARAMS, **(params or {})}
    slot_count = acc.slot_count
    width = load_mod.bucket_width(slot_count, target_buckets)
    slots_per_block = (slot_count / acc.rb_count) if acc.rb_count else None

    present = set(acc.price_changes.keys())
    lanes = [l for l in ["Standard", "Priority"] if l in present] or sorted(present)
    classes = urgency_classes(acc, slots_per_block)

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

    grouped = latency_mod.join_latencies(acc)
    all_lat = [lat for pairs in grouped.values() for (_, lat) in pairs]
    bin_w = _shared_bin_width(all_lat)
    latency_by_class = {}
    for cls in classes:
        cid = cls["id"]
        pairs = grouped.get(cid, [])
        lats = [lat for (_, lat) in pairs]
        stats = latency_mod.class_stats(lats)
        stats["histogram"] = {"binWidth": bin_w, "bins": histogram_bins(lats, bin_w)}
        stats["overTime"] = latency_mod.over_time(pairs, width, slot_count)
        latency_by_class[cid] = stats

    return {
        "meta": {
            "source": source,
            "generatedAt": datetime.now(timezone.utc).isoformat(),
            "slotCount": slot_count,
            "totalEvents": acc.total_events,
            "rbCount": acc.rb_count,
            "slotsPerBlock": slots_per_block,
            "lanes": lanes,
            "urgencyClasses": classes,
        },
        "params": params,
        "price": {"byLane": price_by_lane},
        "shock": {"byLane": shock_by_lane},
        "convergence": {"loadRegimes": regimes, "byLane": conv_by_lane},
        "latency": {"byClass": latency_by_class},
        "load": load_obj,
    }


def write_data_js(sim_data, path):
    """Serialise SIM_DATA as a JS global so the dashboard works from file://."""
    payload = json.dumps(sim_data, separators=(",", ":"))
    with open(path, "w") as fh:
        fh.write("window.SIM_DATA = " + payload + ";\n")
