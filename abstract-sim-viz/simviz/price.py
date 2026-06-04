from simviz.stats import relative_jump


def price_series(acc, lane):
    """Ordered price-update trace for a lane, with relative jump per step."""
    changes = sorted(acc.price_changes.get(lane, []), key=lambda e: e["slot"])
    return [
        {
            "slot": e["slot"],
            "oldCoeff": e["oldCoeff"],
            "newCoeff": e["newCoeff"],
            "utilisation": e["utilisation"],
            "jump": relative_jump(e["oldCoeff"], e["newCoeff"]),
        }
        for e in changes
    ]


def shock_stats(series, threshold):
    """Mirror priceShockFrom: max jump and count of jumps strictly over threshold."""
    jumps = [p["jump"] for p in series]
    return {
        "maxJump": max(jumps) if jumps else 0.0,
        "shockCount": sum(1 for j in jumps if j > threshold),
    }
