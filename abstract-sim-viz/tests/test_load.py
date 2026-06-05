from collections import defaultdict
from simviz.load import bucket_width, load_buckets


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
