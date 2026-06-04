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


def price_at_or_before(series, slot):
    """Mirror priceAtOrBefore: newCoeff of last change <= slot; else first oldCoeff; else None."""
    prior = [p for p in series if p["slot"] <= slot]
    if prior:
        return prior[-1]["newCoeff"]
    if series:
        return series[0]["oldCoeff"]
    return None


def _within_band(band_pct, reference, price):
    return abs(price - reference) <= abs(reference) * max(0.0, band_pct)


def convergence_for_lane(series, regimes, band_pct):
    """Per-regime convergence + lane summary, mirroring convergenceTimeFrom/convergenceInRegime.

    Returns (regime_results, convergence_time) where convergence_time is the max across
    regimes of (convergenceSlot - regimeStart), or None if any regime never converges.
    """
    regime_results = []
    times = []
    any_unconverged = False
    for regime in regimes:
        start, end = regime["start"], regime["end"]
        reference = price_at_or_before(series, max(0, end - 1)) if end > start else None
        band = None
        conv_slot = None
        if reference is not None and end > start:
            band = [reference * (1 - band_pct), reference * (1 + band_pct)]
            in_regime = [p for p in series if start <= p["slot"] < end]
            candidates = [start] + [p["slot"] for p in in_regime]
            for cand in candidates:
                cand_price = price_at_or_before(series, cand)
                if cand_price is None:
                    continue
                future = [p["newCoeff"] for p in in_regime if p["slot"] > cand]
                if all(_within_band(band_pct, reference, x) for x in [cand_price] + future):
                    conv_slot = cand
                    break
        regime_results.append({
            "start": start, "end": end,
            "reference": reference, "band": band,
            "convergenceSlot": conv_slot,
        })
        if conv_slot is None:
            any_unconverged = True
        else:
            times.append(conv_slot - start)
    convergence_time = None if (any_unconverged or not times) else max(times)
    return regime_results, convergence_time


def oscillation_amplitude(series):
    """Mirror amplitude: peak-to-peak (max-min) of all old+new coeffs across the run."""
    coeffs = []
    for p in series:
        coeffs.append(p["oldCoeff"])
        coeffs.append(p["newCoeff"])
    return (max(coeffs) - min(coeffs)) if coeffs else 0.0
