import json
import os
from preprocess import main
from preprocess import main as run_main


FIXTURE = os.path.join(os.path.dirname(__file__), "fixtures", "tiny.jsonl")


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


def test_tiny_fixture_end_to_end(tmp_path):
    import json
    out = tmp_path / "data.js"
    run_main([FIXTURE, "-o", str(out)])
    data = json.loads(out.read_text()[len("window.SIM_DATA = "):].rstrip().rstrip(";"))

    assert data["meta"]["slotCount"] == 6
    assert data["meta"]["totalEvents"] == 9
    assert data["meta"]["lanes"] == ["Standard", "Priority"]
    assert [c["id"] for c in data["meta"]["urgencyClasses"]] == \
        ["Exponential:0.0005", "Exponential:0.006"]

    assert data["shock"]["byLane"]["Priority"]["maxJump"] == 0.375
    assert data["shock"]["byLane"]["Priority"]["shockCount"] == 1
    assert data["shock"]["byLane"]["Standard"]["shockCount"] == 0

    lat = data["latency"]["byClass"]
    assert lat["Exponential:0.0005"]["count"] == 2
    assert lat["Exponential:0.0005"]["median"] == 1
    assert lat["Exponential:0.0005"]["max"] == 2
    assert lat["Exponential:0.006"]["count"] == 1
    assert lat["Exponential:0.006"]["median"] == 3

    assert len(data["convergence"]["loadRegimes"]) >= 1   # regimes present (noisy at width=1)
