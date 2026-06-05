from simviz.ingest import Accumulator
from simviz.latency import class_id, join_latencies


def _submitted(tx_id, slot, lane, rate, tag="Exponential"):
    return {"tag": "TxSubmitted", "slot": slot, "actorId": 0,
            "tx": {"id": tx_id, "lane": lane, "submitted": slot, "value": 1,
                   "urgency": {"tag": tag, "rate": rate},
                   "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                            "dependsOn": [], "fee": 1}}}


def _included(tx_id, slot):
    return {"tag": "TxIncluded", "slot": slot, "txId": tx_id,
            "inclusionPoint": {"tag": "IncludedInRb"}}


def test_class_id():
    assert class_id("Exponential", 5.0e-4) == "Exponential:0.0005"


def test_join_latencies_groups_by_class_and_skips_unincluded():
    acc = Accumulator()
    for e in [
        _submitted(1, 0, "Standard", 5.0e-4), _included(1, 2),   # latency 2
        _submitted(2, 0, "Priority", 6.0e-3), _included(2, 3),   # latency 3
        _submitted(3, 4, "Standard", 5.0e-4), _included(3, 5),   # latency 1
        _submitted(4, 1, "Standard", 5.0e-4),                    # never included -> skipped
    ]:
        acc.ingest(e)
    grouped = join_latencies(acc)
    assert sorted(grouped["Exponential:0.0005"]) == [(0, 2), (4, 1)]
    assert grouped["Exponential:0.006"] == [(0, 3)]
    # Verify tx 4 (never included) is absent - only txes with both submit+include appear
    assert len(grouped) == 2  # only 2 classes present


def test_join_latencies_by_lane():
    from simviz.latency import join_latencies_by_lane
    acc = Accumulator()
    for e in [
        _submitted(1, 0, "Standard", 0.01), _included(1, 10),   # latency 10
        _submitted(2, 0, "Priority", 0.01), _included(2, 3),    # latency 3
        _submitted(3, 5, "Priority", 0.04), _included(3, 7),    # latency 2
    ]:
        acc.ingest(e)
    grouped = join_latencies_by_lane(acc)
    assert sorted(grouped["Standard"]) == [(0, 10)]
    assert sorted(grouped["Priority"]) == [(0, 3), (5, 2)]


def test_class_stats():
    from simviz.latency import class_stats
    stats = class_stats([1, 2])
    assert stats["count"] == 2
    assert stats["mean"] == 1.5
    assert stats["median"] == 1          # quantile(0.5, [1,2]) -> idx 0
    assert stats["p95"] == 2
    assert stats["max"] == 2
    empty = class_stats([])
    assert empty["count"] == 0 and empty["max"] == 0


def test_over_time_buckets_by_submit_slot():
    from simviz.latency import over_time
    pairs = [(0, 5), (1, 7), (4, 1), (5, 3)]   # (submit_slot, latency)
    out = over_time(pairs, width=2, slot_count=6)
    assert out == [
        {"slot": 0, "median": 5, "p95": 7, "n": 2},   # bucket 0: latencies [5,7]
        {"slot": 4, "median": 1, "p95": 3, "n": 2},   # bucket 4: latencies [1,3]
    ]
