from simviz.stats import quantile, mean, relative_jump, histogram_bins


def test_quantile_matches_haskell_rule():
    # quantile q xs = xs[min(n-1, ceil(q*n)-1)]
    assert quantile(0.50, [1, 2, 3, 4]) == 2        # ceil(2)-1 = 1
    assert quantile(0.95, [1, 2, 3, 4]) == 4        # ceil(3.8)-1 = 3
    assert quantile(0.50, [10, 20, 30]) == 20       # ceil(1.5)-1 = 1
    assert quantile(0.25, [1, 2, 3, 4]) == 1        # ceil(1)-1 = 0


def test_quantile_empty_is_zero():
    assert quantile(0.5, []) == 0


def test_mean():
    assert mean([]) == 0.0
    assert mean([1, 2, 3]) == 2.0


def test_relative_jump():
    assert relative_jump(16, 10) == 0.375
    assert relative_jump(0, 5) == 0.0      # old <= 0 -> 0
    assert relative_jump(4, 4) == 0.0


def test_histogram_bins_basic():
    bins = histogram_bins([0, 1, 5, 9], 5)
    assert bins == [
        {"lo": 0, "hi": 5, "n": 2},    # 0, 1
        {"lo": 5, "hi": 10, "n": 2},   # 5, 9
    ]


def test_histogram_bins_empty():
    assert histogram_bins([], 5) == []
    assert histogram_bins([1, 2], 0) == []
