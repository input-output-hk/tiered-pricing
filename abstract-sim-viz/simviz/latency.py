from simviz.ingest import unit_lane


def class_id(tag, rate):
    """Stable id for an urgency class, e.g. 'Exponential:0.0005'."""
    return f"{tag}:{rate}"


def _join(acc, key_of):
    """Map key_of(unit) -> list of (first_submit_slot, latency_slots) for served
    demand units. Latency runs from the unit's *first* submission to on-chain
    inclusion, so the waiting hidden inside rejected and retried attempts
    counts against the design."""
    out = {}
    for unit in acc.units.values():
        inc = unit["includedAt"]
        if inc is None:
            continue
        first = unit["firstSubmitted"]
        out.setdefault(key_of(unit), []).append((first, inc - first))
    return out


def join_latencies(acc):
    """Latencies grouped by urgency class id (tests actor bidding logic)."""
    return _join(acc, lambda u: class_id(u["meta"]["tag"], u["meta"]["rate"]))


def join_latencies_by_lane(acc):
    """Latencies grouped by lane (tests whether the Priority lane serves faster).
    Units are attributed to the lane that served them."""
    return _join(acc, unit_lane)


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
