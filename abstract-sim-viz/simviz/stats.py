import math


def mean(xs):
    return sum(xs) / len(xs) if xs else 0.0


def relative_jump(old_coeff, new_coeff):
    """Mirror Metrics.Accumulator.relativeJump: |new-old|/old, 0 if old <= 0."""
    if old_coeff <= 0:
        return 0.0
    return abs(new_coeff - old_coeff) / old_coeff


def quantile(q, sorted_xs):
    """Mirror Metrics.Latency.quantile: xs[min(n-1, max(0, ceil(q*n)-1))]."""
    n = len(sorted_xs)
    if n == 0:
        return 0
    idx = min(n - 1, max(0, math.ceil(q * n) - 1))
    return sorted_xs[idx]


def histogram_bins(values, bin_width):
    """Fixed-width bins over [0, max(values)]; returns [{lo, hi, n}]."""
    if not values or bin_width <= 0:
        return []
    n_bins = int(max(values) // bin_width) + 1
    counts = [0] * n_bins
    for v in values:
        idx = int(v // bin_width)
        idx = 0 if idx < 0 else min(idx, n_bins - 1)
        counts[idx] += 1
    return [
        {"lo": i * bin_width, "hi": (i + 1) * bin_width, "n": counts[i]}
        for i in range(n_bins)
    ]
