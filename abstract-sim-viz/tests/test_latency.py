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
