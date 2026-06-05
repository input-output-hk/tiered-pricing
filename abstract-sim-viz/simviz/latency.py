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
