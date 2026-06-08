from simviz.ingest import iter_events, Accumulator


def test_iter_events_yields_inner_event_objects(tmp_path):
    f = tmp_path / "trace.jsonl"
    f.write_text(
        '{"event":{"tag":"TxAdmitted","slot":0,"txId":1},"eventNo":0}\n'
        '\n'  # blank line is skipped
        '{"event":{"tag":"TxAdmitted","slot":1,"txId":2},"eventNo":1}\n'
    )
    events = list(iter_events(str(f)))
    assert events == [
        {"tag": "TxAdmitted", "slot": 0, "txId": 1},
        {"tag": "TxAdmitted", "slot": 1, "txId": 2},
    ]


def _submitted(tx_id, slot, lane, rate, tag="Exponential"):
    return {
        "tag": "TxSubmitted", "slot": slot, "actorId": 0,
        "tx": {
            "id": tx_id, "lane": lane, "submitted": slot, "value": 100,
            "urgency": {"tag": tag, "rate": rate},
            "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                     "dependsOn": [], "fee": 1},
        },
    }


def _included(tx_id, slot):
    return {"tag": "TxIncluded", "slot": slot, "txId": tx_id,
            "inclusionPoint": {"tag": "IncludedInRb"}}


def _price(lane, slot, old, new, util):
    return {"tag": "PriceUpdated", "slot": slot, "lane": lane,
            "oldCoeff": old, "newCoeff": new, "utilisation": util}


def test_accumulator_records_state():
    acc = Accumulator()
    for e in [
        _submitted(1, 0, "Standard", 5.0e-4),
        _included(1, 2),
        _price("Priority", 1, 16, 10, 0.4),
    ]:
        acc.ingest(e)
    assert acc.submitted_at == {1: 0}
    assert acc.included_at == {1: 2}
    assert acc.tx_meta[1] == {"tag": "Exponential", "rate": 5.0e-4, "lane": "Standard"}
    assert acc.submissions_per_slot[0] == 1
    assert acc.inclusions_per_slot[2] == 1
    assert acc.price_changes["Priority"][0]["newCoeff"] == 10
    assert acc.slot_count == 3          # max slot 2 -> +1
    assert acc.total_events == 3


def _rb(slot, block_tag):
    return {"tag": "BlockProduced", "slot": slot,
            "summary": {"tag": "RankingBlockProduced", "summary": {"block": {"tag": block_tag}}}}


def test_accumulator_counts_rb_content():
    acc = Accumulator()
    acc.ingest(_rb(10, "PraosBlock"))       # carries txs
    acc.ingest(_rb(20, "CertifyingBlock"))  # certifies an EB
    acc.ingest(_rb(30, "PraosBlock"))
    assert acc.rb_count == 3
    assert acc.rb_tx_count == 2
    assert acc.rb_cert_count == 1
    assert acc.rb_series == [
        {"slot": 10, "kind": "txs", "fill": None},   # no capacity in fixture -> fill None
        {"slot": 20, "kind": "cert", "fill": None},
        {"slot": 30, "kind": "txs", "fill": None},
    ]


def test_rb_fullness_is_binding_utilisation():
    acc = Accumulator()
    acc.ingest({"tag": "BlockProduced", "slot": 5, "summary": {
        "tag": "RankingBlockProduced", "summary": {
            "block": {"tag": "PraosBlock"},
            "usedBytes": 45, "capacityBytes": 90,        # 0.5 by bytes
            "usedExUnits": 20, "capacityExUnits": 100}}})  # 0.2 by ex-units
    assert acc.rb_series[0]["fill"] == 0.5               # binding = max(0.5, 0.2)


def test_cert_block_shaded_by_certified_eb_fullness():
    acc = Accumulator()
    # EB id 7 announced 30% full (bytes), 0% by ex-units -> binding 0.3
    acc.ingest({"tag": "BlockProduced", "slot": 5, "summary": {
        "tag": "EndorserBlockAnnounced", "summary": {
            "id": 7, "usedBytes": 30, "capacityBytes": 100,
            "usedExUnits": 0, "capacityExUnits": 100}}})
    # a later RB certifies EB 7
    acc.ingest({"tag": "BlockProduced", "slot": 18, "summary": {
        "tag": "RankingBlockProduced", "summary": {"block": {"tag": "CertifyingBlock", "ebId": 7}}}})
    assert acc.eb_fullness[7] == 0.3
    assert acc.rb_series[0] == {"slot": 18, "kind": "cert", "fill": 0.3}


def test_accumulator_last_wins_on_duplicate_txid():
    acc = Accumulator()
    acc.ingest(_submitted(1, 0, "Standard", 5.0e-4))
    acc.ingest(_submitted(1, 5, "Priority", 6.0e-3))   # resubmission, same id
    acc.ingest(_included(1, 3))
    acc.ingest(_included(1, 9))                         # later inclusion wins
    assert acc.submitted_at[1] == 5
    assert acc.tx_meta[1]["lane"] == "Priority"
    assert acc.included_at[1] == 9
