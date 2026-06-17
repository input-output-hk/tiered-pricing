from simviz.ingest import unit_lane


def class_id(tag, rate):
    """Stable id for an urgency class, e.g. 'Exponential:0.0005'."""
    return f"{tag}:{rate}"


def _join(acc, key_of, latency_of, x_of):
    """Map key_of(unit) -> list of (x, latency) for served
    demand units. Latency runs from the unit's *first* submission to on-chain
    inclusion, so the waiting hidden inside rejected and retried attempts
    counts against the design."""
    out = {}
    for unit in acc.units.values():
        if unit["includedAt"] is None:
            continue
        latency = latency_of(unit)
        x = x_of(unit)
        if latency is None or x is None:
            continue
        out.setdefault(key_of(unit), []).append((x, latency))
    return out


def _slot_latency(unit):
    return unit["includedAt"] - unit["firstSubmitted"]


def _block_latency(unit):
    included = unit.get("includedBlock")
    first = unit.get("firstSubmittedBlock")
    if included is None or first is None:
        return None
    return max(0, included - first)


def _submit_slot(unit):
    return unit["firstSubmitted"]


def _include_slot(unit):
    return unit["includedAt"]


def join_latencies(acc):
    """Latencies grouped by urgency class id (tests actor bidding logic)."""
    return _join(acc, lambda u: class_id(u["meta"]["tag"], u["meta"]["rate"]),
                 _slot_latency, _submit_slot)


def join_latencies_by_lane(acc):
    """Latencies grouped by lane (tests whether the Priority lane serves faster).
    Units are attributed to the lane that served them."""
    return _join(acc, unit_lane, _slot_latency, _submit_slot)


def join_block_latencies(acc):
    """Actual produced-ranking-block latencies grouped by urgency class id."""
    return _join(acc, lambda u: class_id(u["meta"]["tag"], u["meta"]["rate"]),
                 _block_latency, _submit_slot)


def join_block_latencies_by_lane(acc):
    """Actual produced-ranking-block latencies grouped by serving lane."""
    return _join(acc, unit_lane, _block_latency, _submit_slot)


def join_block_latencies_by_inclusion(acc):
    """Actual block latencies grouped by urgency class, keyed by inclusion slot."""
    return _join(acc, lambda u: class_id(u["meta"]["tag"], u["meta"]["rate"]),
                 _block_latency, _include_slot)


def join_block_latencies_by_lane_inclusion(acc):
    """Actual block latencies grouped by serving lane, keyed by inclusion slot."""
    return _join(acc, unit_lane, _block_latency, _include_slot)


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
