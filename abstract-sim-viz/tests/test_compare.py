import json
from compare import main


def test_compare_embeds_summary_verbatim(tmp_path):
    summary = {
        "description": "test sweep",
        "slots": 2000,
        "seeds": 2,
        "variants": [
            {"name": "a", "config": "config/a.json",
             "runs": [{"seed": 0, "scalars": {"units.serviceRate": 0.9}}],
             "aggregates": {"units.serviceRate":
                            {"mean": 0.9, "stddev": 0.0, "min": 0.9, "max": 0.9}}},
        ],
    }
    src = tmp_path / "summary.json"
    src.write_text(json.dumps(summary))
    out = tmp_path / "compare-data.js"
    main([str(src), "-o", str(out)])
    text = out.read_text()
    assert text.startswith("window.SWEEP_DATA = ")
    payload = json.loads(text[len("window.SWEEP_DATA = "):].rstrip().rstrip(";"))
    assert payload["summary"] == summary
    assert "generatedAt" in payload and "source" in payload
