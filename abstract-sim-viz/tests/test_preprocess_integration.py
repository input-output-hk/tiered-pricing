import json
from preprocess import main


def _line(obj, n):
    return json.dumps({"event": obj, "eventNo": n})


def test_cli_writes_data_js(tmp_path):
    trace = tmp_path / "events.jsonl"
    trace.write_text("\n".join([
        _line({"tag": "TxSubmitted", "slot": 0, "actorId": 0,
               "tx": {"id": 1, "lane": "Standard", "submitted": 0, "value": 1,
                      "urgency": {"tag": "Exponential", "rate": 5.0e-4},
                      "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                               "dependsOn": [], "fee": 1}}}, 0),
        _line({"tag": "TxIncluded", "slot": 2, "txId": 1,
               "inclusionPoint": {"tag": "IncludedInRb"}}, 1),
    ]) + "\n")
    out = tmp_path / "data.js"
    main([str(trace), "-o", str(out)])
    text = out.read_text()
    assert text.startswith("window.SIM_DATA = ")
    payload = json.loads(text[len("window.SIM_DATA = "):].rstrip().rstrip(";"))
    assert payload["meta"]["slotCount"] == 3
    assert payload["meta"]["totalEvents"] == 2
