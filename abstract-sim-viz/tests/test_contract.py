import json
from simviz.ingest import Accumulator
from simviz.contract import urgency_classes, build_sim_data, write_data_js


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
    classes = urgency_classes(acc)      # no block cadence -> half-life shown in slots
    assert [c["rate"] for c in classes] == [5.0e-4, 6.0e-3]
    assert classes[0]["id"] == "Exponential:0.0005"
    assert classes[0]["tag"] == "Exponential"
    # half-life = ln(2)/rate slots; 0.0005 -> 1386 slots, 0.006 -> 116 slots
    assert round(classes[0]["halfLifeSlots"]) == 1386
    assert classes[0]["halfLifeBlocks"] is None
    assert classes[0]["label"] == "t½≈1386 sl"


def _ranking_block(slot):
    return {"tag": "BlockProduced", "slot": slot,
            "summary": {"tag": "RankingBlockProduced", "summary": {}}}


def test_half_life_in_blocks_uses_ranking_block_cadence():
    acc = Accumulator()
    acc.ingest(_submitted(1, 5.0e-4))                 # half-life 1386 slots
    for slot in range(0, 100, 20):                    # 5 ranking blocks over 100 slots
        acc.ingest(_ranking_block(slot))
    acc.ingest(_submitted(2, 5.0e-4, tag="Exponential"))
    # force slot_count = 100 via a late event
    acc.ingest(_included(1, 99))
    data = build_sim_data(acc)
    assert data["meta"]["rbCount"] == 5
    assert data["meta"]["slotsPerBlock"] == 20.0      # 100 slots / 5 RBs
    cls = data["meta"]["urgencyClasses"][0]
    assert round(cls["halfLifeBlocks"], 1) == 69.3     # 1386.3 / 20
    assert cls["label"] == "t½≈69 blk"


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


def test_write_data_js_roundtrip(tmp_path):
    sim_data = {"meta": {"slotCount": 3}, "price": {"byLane": {}}}
    out = tmp_path / "data.js"
    write_data_js(sim_data, str(out))
    text = out.read_text()
    assert text.startswith("window.SIM_DATA = ")
    assert text.rstrip().endswith(";")
    payload = text[len("window.SIM_DATA = "):].rstrip().rstrip(";")
    assert json.loads(payload) == sim_data
