from simviz.ingest import Accumulator
from simviz.price import price_series, shock_stats


def _price(lane, slot, old, new, util=0.0):
    return {"tag": "PriceUpdated", "slot": slot, "lane": lane,
            "oldCoeff": old, "newCoeff": new, "utilisation": util}


def test_price_series_sorted_with_jumps():
    acc = Accumulator()
    acc.ingest(_price("Priority", 2, 10, 10.5, 0.5))
    acc.ingest(_price("Priority", 1, 16, 10, 0.4))     # out of order on purpose
    series = price_series(acc, "Priority")
    assert [p["slot"] for p in series] == [1, 2]
    assert series[0]["jump"] == 0.375                  # |10-16|/16
    assert round(series[1]["jump"], 3) == 0.05         # |10.5-10|/10


def test_shock_stats():
    series = [{"jump": 0.375}, {"jump": 0.05}, {"jump": 0.2}]
    assert shock_stats(series, 0.10) == {"maxJump": 0.375, "shockCount": 2}
    assert shock_stats([], 0.10) == {"maxJump": 0.0, "shockCount": 0}
