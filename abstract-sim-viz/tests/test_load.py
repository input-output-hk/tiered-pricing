from collections import defaultdict
from simviz.load import bucket_width, load_buckets, smooth_rate, detect_regimes


def test_bucket_width_adaptive():
    assert bucket_width(6, target_buckets=300) == 1
    assert bucket_width(2000, target_buckets=300) == 7      # ceil(2000/300)
    assert bucket_width(0, target_buckets=300) == 1         # never below 1


def test_load_buckets_counts_per_window():
    subs = defaultdict(int, {0: 2, 1: 1, 3: 4})
    incs = defaultdict(int, {1: 1, 2: 2})
    buckets = load_buckets(subs, incs, slot_count=4, width=2)
    assert buckets == [
        {"slot": 0, "submissions": 3, "inclusions": 1},     # slots 0,1
        {"slot": 2, "submissions": 4, "inclusions": 2},     # slots 2,3
    ]


def test_smooth_rate_trailing_average():
    subs = defaultdict(int, {0: 4, 1: 0, 2: 2})
    # window 2: slot0 -> 4/1, slot1 -> (4+0)/2, slot2 -> (0+2)/2
    assert smooth_rate(subs, slot_count=3, window=2) == [4.0, 2.0, 1.0]


def test_detect_regimes_step_change():
    rate = [2.0] * 5 + [40.0] * 5
    regimes = detect_regimes(rate, change_pct=0.10)
    assert [(r["start"], r["end"]) for r in regimes] == [(0, 5), (5, 10)]
    assert regimes[0]["meanArrival"] == 2.0
    assert regimes[1]["meanArrival"] == 40.0


def test_detect_regimes_single_when_flat():
    regimes = detect_regimes([2.0, 2.0, 2.0], change_pct=0.10)
    assert [(r["start"], r["end"]) for r in regimes] == [(0, 3)]


def test_detect_regimes_empty():
    assert detect_regimes([], change_pct=0.10) == []
