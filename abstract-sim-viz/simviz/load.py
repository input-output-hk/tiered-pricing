import math
from simviz.stats import mean


def bucket_width(slot_count, target_buckets=300):
    return max(1, math.ceil(slot_count / target_buckets)) if slot_count > 0 else 1


def load_buckets(submissions_per_slot, inclusions_per_slot, slot_count, width):
    buckets = []
    for start in range(0, slot_count, width):
        end = min(start + width, slot_count)
        subs = sum(submissions_per_slot.get(s, 0) for s in range(start, end))
        incs = sum(inclusions_per_slot.get(s, 0) for s in range(start, end))
        buckets.append({"slot": start, "submissions": subs, "inclusions": incs})
    return buckets


def smooth_rate(submissions_per_slot, slot_count, window):
    """Trailing moving average of submissions/slot over `window` slots."""
    window = max(1, window)
    rate = []
    run = 0
    for s in range(slot_count):
        run += submissions_per_slot.get(s, 0)
        if s >= window:
            run -= submissions_per_slot.get(s - window, 0)
        denom = min(s + 1, window)
        rate.append(run / denom)
    return rate


def _material_change(change_pct, old_rate, new_rate):
    """Mirror materialLoadChange."""
    if old_rate == new_rate:
        return False
    if old_rate <= 0:
        return new_rate > 0
    return abs(new_rate - old_rate) / old_rate > max(0.0, change_pct)


def detect_regimes(rate_series, change_pct):
    """Segment a (smoothed) rate series into load regimes.

    Mirrors loadRegimes' material-change logic, but compares each slot's rate against
    the rate at the START of the current regime (not the immediately previous slot) so
    that Poisson slot-to-slot noise in the observed series doesn't over-segment.
    """
    n = len(rate_series)
    if n == 0:
        return []
    regimes = []
    start = 0
    base = rate_series[0]
    for s in range(1, n):
        if _material_change(change_pct, base, rate_series[s]):
            regimes.append({"start": start, "end": s,
                            "meanArrival": mean(rate_series[start:s])})
            start = s
            base = rate_series[s]
    regimes.append({"start": start, "end": n,
                    "meanArrival": mean(rate_series[start:n])})
    return regimes
