from simviz.ingest import Accumulator
from simviz.contract import urgency_classes, build_sim_data


def _submitted(tx_id, rate, tag="Exponential"):
    return {"tag": "TxSubmitted", "slot": 0, "actorId": 0,
            "tx": {"id": tx_id, "lane": "Standard", "submitted": 0, "value": 1,
                   "urgency": {"tag": tag, "rate": rate},
                   "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                            "dependsOn": [], "fee": 1}}}


def test_urgency_classes_ordered_low_to_high_rate():
    acc = Accumulator()
    acc.ingest(_submitted(1, 6.0e-3))
    acc.ingest(_submitted(2, 5.0e-4))
    acc.ingest(_submitted(3, 5.0e-4))   # duplicate class
    classes = urgency_classes(acc)
    assert [c["rate"] for c in classes] == [5.0e-4, 6.0e-3]
    assert classes[0]["id"] == "Exponential:0.0005"
    assert classes[0]["label"] == "Exp λ=0.0005"
    assert classes[0]["tag"] == "Exponential"


def _price(lane, slot, old, new, util=0.0):
    return {"tag": "PriceUpdated", "slot": slot, "lane": lane,
            "oldCoeff": old, "newCoeff": new, "utilisation": util}


def _included(tx_id, slot):
    return {"tag": "TxIncluded", "slot": slot, "txId": tx_id,
            "inclusionPoint": {"tag": "IncludedInRb"}}


def test_build_sim_data_structure_and_values():
    acc = Accumulator()
    for e in [
        _submitted(1, 5.0e-4), _included(1, 2),
        _price("Priority", 1, 16, 10, 0.4),     # jump 0.375 -> shock
        _price("Standard", 1, 1, 1, 0.2),
    ]:
        acc.ingest(e)
    data = build_sim_data(acc, source="trace.jsonl")

    # meta
    assert data["meta"]["slotCount"] == 3       # max slot 2 + 1
    assert data["meta"]["totalEvents"] == 4
    assert data["meta"]["lanes"] == ["Standard", "Priority"]
    assert len(data["meta"]["urgencyClasses"]) == 1
    assert "generatedAt" in data["meta"]

    # params
    assert data["params"]["shockThreshold"] == 0.10

    # price + shock
    assert data["price"]["byLane"]["Priority"][0]["jump"] == 0.375
    assert data["shock"]["byLane"]["Priority"]["shockCount"] == 1
    assert data["shock"]["byLane"]["Standard"]["shockCount"] == 0

    # convergence + load + latency keys present
    assert "loadRegimes" in data["convergence"]
    assert "Priority" in data["convergence"]["byLane"]
    assert data["load"]["bucketWidth"] >= 1
    cls_id = data["meta"]["urgencyClasses"][0]["id"]
    assert data["latency"]["byClass"][cls_id]["count"] == 1
    assert data["latency"]["byClass"][cls_id]["max"] == 2
