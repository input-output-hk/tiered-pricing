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
    runs = _runs_payload(out.read_text())
    payload = runs[0]["data"]
    assert payload["meta"]["slotCount"] == 3
    assert payload["meta"]["totalEvents"] == 2


def test_tiny_fixture_end_to_end(tmp_path):
    out = tmp_path / "data.js"
    run_main([FIXTURE, "-o", str(out)])
    data = _runs_payload(out.read_text())[0]["data"]

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


def _runs_payload(text):
    """Parse the SIM_RUNS line of a data.js produced by the CLI."""
    first, second = text.rstrip().split("\n")
    assert first.startswith("window.SIM_RUNS = ")
    assert second == "window.SIM_DATA = window.SIM_RUNS[0].data;"
    return json.loads(first[len("window.SIM_RUNS = "):].rstrip(";"))


def test_cli_multiple_traces_one_run_each(tmp_path):
    for name in ["flat-fee-seed0", "single-lane-seed0"]:
        trace = tmp_path / f"{name}.events.jsonl"
        trace.write_text(_line({"tag": "TxSubmitted", "slot": 0, "actorId": 0,
                                "tx": {"id": 1, "lane": "Standard", "submitted": 0, "value": 1,
                                       "urgency": {"tag": "Exponential", "rate": 5.0e-4},
                                       "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                                                "dependsOn": [], "fee": 1}}}, 0) + "\n")
    out = tmp_path / "data.js"
    main([str(tmp_path / "flat-fee-seed0.events.jsonl"),
          str(tmp_path / "single-lane-seed0.events.jsonl"),
          "-o", str(out)])
    runs = _runs_payload(out.read_text())
    assert [r["name"] for r in runs] == ["flat-fee-seed0", "single-lane-seed0"]
    assert all(r["data"]["meta"]["slotCount"] == 1 for r in runs)


def test_cli_single_trace_uses_same_runs_contract(tmp_path):
    out = tmp_path / "data.js"
    run_main([FIXTURE, "-o", str(out)])
    runs = _runs_payload(out.read_text())
    assert [r["name"] for r in runs] == ["tiny"]
    assert runs[0]["data"]["meta"]["slotCount"] == 6


def test_runs_carry_variant_and_seed(tmp_path):
    # sweep traces are named <variant>-seed<N>: the bundle splits that into
    # a variant group and a numeric seed so the dashboard can offer them as
    # separate selectors
    for name in ["priority-only-open-seed0", "priority-only-open-seed10", "adhoc"]:
        (tmp_path / f"{name}.events.jsonl").write_text(
            _line({"tag": "TxSubmitted", "slot": 0, "actorId": 0,
                   "tx": {"id": 1, "lane": "Standard", "submitted": 0, "value": 1,
                          "urgency": {"tag": "Exponential", "rate": 5.0e-4},
                          "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                                   "dependsOn": [], "fee": 1}}}, 0) + "\n")
    out = tmp_path / "data.js"
    main([str(tmp_path / "priority-only-open-seed0.events.jsonl"),
          str(tmp_path / "priority-only-open-seed10.events.jsonl"),
          str(tmp_path / "adhoc.events.jsonl"),
          "-o", str(out)])
    runs = _runs_payload(out.read_text())
    assert [(r["variant"], r["seed"]) for r in runs] == [
        ("priority-only-open", 0),
        ("priority-only-open", 10),
        ("adhoc", None),
    ]


def test_cli_duplicate_basenames_are_disambiguated(tmp_path):
    (tmp_path / "a").mkdir()
    (tmp_path / "b").mkdir()
    for d in ["a", "b"]:
        (tmp_path / d / "run.events.jsonl").write_text(
            _line({"tag": "TxSubmitted", "slot": 0, "actorId": 0,
                   "tx": {"id": 1, "lane": "Standard", "submitted": 0, "value": 1,
                          "urgency": {"tag": "Exponential", "rate": 5.0e-4},
                          "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                                   "dependsOn": [], "fee": 1}}}, 0) + "\n")
    out = tmp_path / "data.js"
    main([str(tmp_path / "a" / "run.events.jsonl"),
          str(tmp_path / "b" / "run.events.jsonl"),
          "-o", str(out)])
    names = [r["name"] for r in _runs_payload(out.read_text())]
    assert len(set(names)) == 2
    assert all("run" in n for n in names)
