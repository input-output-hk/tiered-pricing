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


def test_accumulator_last_wins_on_duplicate_txid():
    acc = Accumulator()
    acc.ingest(_submitted(1, 0, "Standard", 5.0e-4))
    acc.ingest(_submitted(1, 5, "Priority", 6.0e-3))   # resubmission, same id
    acc.ingest(_included(1, 3))
    acc.ingest(_included(1, 9))                         # later inclusion wins
    assert acc.submitted_at[1] == 5
    assert acc.tx_meta[1]["lane"] == "Priority"
    assert acc.included_at[1] == 9
