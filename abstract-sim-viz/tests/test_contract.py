import json
from simviz.ingest import Accumulator
from simviz.contract import urgency_classes, build_sim_data, write_data_js


def _submitted(tx_id, rate, tag="Exponential"):
    return {"tag": "TxSubmitted", "slot": 0, "actorId": 0,
            "tx": {"id": tx_id, "lane": "Standard", "submitted": 0, "value": 1,
                   "urgency": {"tag": tag, "rate": rate},
                   "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                            "dependsOn": [], "fee": 1}}}


def test_urgency_classes_half_life_native_in_blocks():
    acc = Accumulator()
    acc.ingest(_submitted(1, 6.0e-3))
    acc.ingest(_submitted(2, 5.0e-4))
    acc.ingest(_submitted(3, 5.0e-4))   # duplicate class
    classes = urgency_classes(acc)      # no f -> slot equivalent omitted
    assert [c["rate"] for c in classes] == [5.0e-4, 6.0e-3]
    assert classes[0]["id"] == "Exponential:0.0005"
    assert classes[0]["tag"] == "Exponential"
    # rate is per block: half-life = ln(2)/rate blocks, exact (no conversion)
    assert round(classes[0]["halfLifeBlocks"]) == 1386
    assert classes[0]["halfLifeSlots"] is None        # no f provided
    assert classes[0]["label"] == "t½≈1386 blk"


def test_urgency_classes_slot_equivalent_uses_f():
    acc = Accumulator()
    acc.ingest(_submitted(1, 0.01))                    # per-block rate
    classes = urgency_classes(acc, f=0.05)
    c = classes[0]
    assert round(c["halfLifeBlocks"], 1) == 69.3       # ln2/0.01, exact in blocks
    assert round(c["halfLifeSlots"]) == 1386           # 69.3 / 0.05
    assert c["label"] == "t½≈69 blk"


def _ranking_block(slot):
    return {"tag": "BlockProduced", "slot": slot,
            "summary": {"tag": "RankingBlockProduced", "summary": {}}}


def test_block_conversion_pins_f_and_keeps_realized_as_sanity():
    # Realized cadence (25 slots/block) deliberately differs from expected (1/f = 20)
    # to prove conversions use the pinned f, not the realized RB count.
    acc = Accumulator()
    acc.ingest(_submitted(1, 0.01))
    for slot in range(0, 80, 20):                      # 4 ranking blocks
        acc.ingest(_ranking_block(slot))
    acc.ingest(_included(1, 99))                       # slot_count = 100
    data = build_sim_data(acc, f=0.05)
    assert data["meta"]["f"] == 0.05
    assert data["meta"]["expectedSlotsPerBlock"] == 20.0   # 1/f, used for conversion
    assert data["meta"]["rbCount"] == 4
    assert data["meta"]["realizedSlotsPerBlock"] == 25.0   # 100/4, sanity only
    cls = data["meta"]["urgencyClasses"][0]
    assert round(cls["halfLifeBlocks"], 1) == 69.3         # independent of cadence
    assert round(cls["halfLifeSlots"]) == 1386             # 69.3 / f (NOT * 25)


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
    assert data["meta"]["submittedByLane"] == {"Standard": 1}
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
    assert "overTime" in data["latency"]["byClass"][cls_id]      # by submission slot
    assert "overTimeIncl" in data["latency"]["byClass"][cls_id]  # by inclusion slot

    # flow: tx1 submitted slot 0, included slot 2 via RB, lane Standard
    assert data["flow"]["links"] == [[0, 2, 0, 0]]   # [submit, incl, route=RB(0), lane=Standard(0)]
    assert data["flow"]["total"] == 1
    assert data["flow"]["sampleRate"] == 1.0


def test_blocks_section_counts_rb_tx_vs_cert():
    acc = Accumulator()
    acc.ingest(_submitted(1, 0.01))
    acc.ingest({"tag": "BlockProduced", "slot": 10, "summary": {
        "tag": "RankingBlockProduced", "summary": {"block": {"tag": "PraosBlock", "txIds": [1, 2]}}}})
    acc.ingest({"tag": "BlockProduced", "slot": 20, "summary": {
        "tag": "RankingBlockProduced", "summary": {"block": {"tag": "CertifyingBlock", "ebId": 3}}}})
    data = build_sim_data(acc, f=0.05)
    assert data["blocks"]["rbTotal"] == 2
    assert data["blocks"]["rbWithTxs"] == 1
    assert data["blocks"]["rbWithCert"] == 1
    assert data["blocks"]["rbSeries"] == [
        {"slot": 10, "kind": "txs", "fill": None}, {"slot": 20, "kind": "cert", "fill": None}]


def test_write_data_js_roundtrip(tmp_path):
    sim_data = {"meta": {"slotCount": 3}, "price": {"byLane": {}}}
    out = tmp_path / "data.js"
    write_data_js(sim_data, str(out))
    text = out.read_text()
    assert text.startswith("window.SIM_DATA = ")
    assert text.rstrip().endswith(";")
    payload = text[len("window.SIM_DATA = "):].rstrip().rstrip(";")
    assert json.loads(payload) == sim_data
