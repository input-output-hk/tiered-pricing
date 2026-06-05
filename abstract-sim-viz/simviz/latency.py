def class_id(tag, rate):
    """Stable id for an urgency class, e.g. 'Exponential:0.0005'."""
    return f"{tag}:{rate}"


def join_latencies(acc):
    """Map class_id -> list of (submit_slot, latency_slots) for txs with both events."""
    out = {}
    for tx_id, submit_slot in acc.submitted_at.items():
        inc = acc.included_at.get(tx_id)
        meta = acc.tx_meta.get(tx_id)
        if inc is None or meta is None:
            continue
        cid = class_id(meta["tag"], meta["rate"])
        out.setdefault(cid, []).append((submit_slot, inc - submit_slot))
    return out


from simviz.stats import quantile, mean


def class_stats(latencies):
    xs = sorted(latencies)
    n = len(xs)
    if n == 0:
        return {"count": 0, "mean": 0.0, "median": 0,
                "p25": 0, "p75": 0, "p95": 0, "max": 0}
    return {
        "count": n,
        "mean": mean(xs),
        "median": quantile(0.50, xs),
        "p25": quantile(0.25, xs),
        "p75": quantile(0.75, xs),
        "p95": quantile(0.95, xs),
        "max": xs[-1],
    }


def over_time(pairs, width, slot_count):
    """Bucket (submit_slot, latency) pairs by submit slot; per bucket emit median/p95/n."""
    buckets = {}
    for submit_slot, lat in pairs:
        key = (submit_slot // width) * width
        buckets.setdefault(key, []).append(lat)
    out = []
    for start in sorted(buckets):
        xs = sorted(buckets[start])
        out.append({"slot": start, "median": quantile(0.5, xs),
                    "p95": quantile(0.95, xs), "n": len(xs)})
    return out
