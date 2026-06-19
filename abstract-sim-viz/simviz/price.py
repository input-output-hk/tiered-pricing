import math

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


def settled_coefficient_range(series, band_pct):
    """Mirror priceStabilityFrom: peak-to-peak range after settling.

    A lane settles at the earliest coefficient from which every later coefficient
    stays within the band around the final coefficient. If it never settles, the
    full-run coefficient range is reported instead.
    """
    if not series:
        return 0.0

    coeff_path = [(0, series[0]["oldCoeff"])]
    coeff_path.extend((p["slot"], p["newCoeff"]) for p in series)
    final_coeff = coeff_path[-1][1]

    settled_tail = None
    for i in range(0, len(coeff_path) - 1):
        suffix = coeff_path[i:]
        if all(_within_band(band_pct, final_coeff, coeff) for _, coeff in suffix):
            settled_tail = suffix
            break

    coeffs = [coeff for _, coeff in (settled_tail or coeff_path)]
    return max(coeffs) - min(coeffs)


def oscillation_stats(series, deadband_pct):
    """True price oscillation: significant repeated direction reversals.

    The deadband is relative to each step's old coefficient. Moves inside the
    deadband are ignored, and same-direction significant moves collapse into a
    single segment before reversals, amplitudes, and excess log travel are read.
    """
    moves = _significant_oscillation_moves(series, deadband_pct)
    if not moves:
        return _empty_oscillation()

    directions = _compressed_directions(moves)
    endpoints = _segment_endpoints(moves)
    reversal_count = max(0, len(directions) - 1)
    travel = sum(_log_distance(a, b) for a, b in zip(endpoints, endpoints[1:]))
    net = _log_distance(endpoints[0], endpoints[-1]) if len(endpoints) > 1 else 0.0
    amplitudes = [
        max(a, b, c) - min(a, b, c)
        for a, b, c in zip(endpoints, endpoints[1:], endpoints[2:])
    ]
    return {
        "oscillationReversalCount": reversal_count,
        "oscillationCycleCount": reversal_count // 2,
        "maxOscillationAmplitude": max(amplitudes) if amplitudes else 0.0,
        "oscillationExcessTravel": max(0.0, travel - net),
    }


def oscillation_reversals(series, deadband_pct):
    """Significant direction-reversal markers for annotating the price trace."""
    moves = _significant_oscillation_moves(series, deadband_pct)
    if not moves:
        return []

    reversals = []
    direction = moves[0]["direction"]
    for move in moves[1:]:
        if move["direction"] == direction:
            continue
        reversals.append({
            "slot": move["slot"],
            "coeff": move["old"],
            "fromDirection": _direction_label(direction),
            "toDirection": _direction_label(move["direction"]),
        })
        direction = move["direction"]
    return reversals


def _empty_oscillation():
    return {
        "oscillationReversalCount": 0,
        "oscillationCycleCount": 0,
        "maxOscillationAmplitude": 0.0,
        "oscillationExcessTravel": 0.0,
    }


def _significant_oscillation_moves(series, deadband_pct):
    if not series:
        return []

    moves = []
    old = series[0]["oldCoeff"]
    for point in series:
        new = point["newCoeff"]
        if old > 0 and new > 0 and relative_jump(old, new) > max(0.0, deadband_pct):
            if new > old:
                moves.append({"slot": point["slot"], "direction": 1, "old": old, "new": new})
            elif new < old:
                moves.append({"slot": point["slot"], "direction": -1, "old": old, "new": new})
        old = new
    return moves


def _direction_label(direction):
    return "up" if direction > 0 else "down"


def _compressed_directions(moves):
    directions = [moves[0]["direction"]]
    for move in moves[1:]:
        if move["direction"] != directions[-1]:
            directions.append(move["direction"])
    return directions


def _segment_endpoints(moves):
    points = [moves[0]["old"]]
    direction = moves[0]["direction"]
    end = moves[0]["new"]
    for move in moves[1:]:
        if move["direction"] == direction:
            end = move["new"]
        else:
            points.append(move["old"])
            direction = move["direction"]
            end = move["new"]
    points.append(end)
    return points


def _log_distance(a, b):
    if a <= 0 or b <= 0:
        return 0.0
    return abs(math.log(b) - math.log(a))
