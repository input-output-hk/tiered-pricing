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
