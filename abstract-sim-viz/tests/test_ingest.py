from simviz.ingest import iter_events


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
