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


from simviz.price import price_at_or_before, convergence_for_lane, oscillation_amplitude


def test_price_at_or_before():
    series = [{"slot": 1, "oldCoeff": 16, "newCoeff": 10},
              {"slot": 5, "oldCoeff": 10, "newCoeff": 12}]
    assert price_at_or_before(series, 0) == 16     # before first change -> first oldCoeff
    assert price_at_or_before(series, 1) == 10     # newCoeff of change at slot 1
    assert price_at_or_before(series, 4) == 10
    assert price_at_or_before(series, 9) == 12
    assert price_at_or_before([], 3) is None


def test_convergence_converges_within_band():
    # one regime [0, 10). Price settles at 10 (reference at slot 9 = 10).
    series = [{"slot": 1, "oldCoeff": 16, "newCoeff": 10},
              {"slot": 2, "oldCoeff": 10, "newCoeff": 10}]
    regimes = [{"start": 0, "end": 10, "meanArrival": 2.0}]
    results, conv_time = convergence_for_lane(series, regimes, 0.05)
    assert results[0]["convergenceSlot"] == 1      # from slot 1 onward all within 5% of 10
    assert conv_time == 1                           # 1 - regimeStart(0)


def test_convergence_none_without_price_data():
    # The reference is the regime's final settled price, so any regime WITH price data
    # always converges (at worst at its last change). convergence_time is therefore None
    # only when there is no price data (no reference) or a degenerate regime.
    results, conv_time = convergence_for_lane(
        [], [{"start": 0, "end": 5, "meanArrival": 2.0}], 0.05)
    assert conv_time is None
    assert results[0]["convergenceSlot"] is None


def test_oscillation_amplitude():
    series = [{"oldCoeff": 16, "newCoeff": 10}, {"oldCoeff": 10, "newCoeff": 12}]
    assert oscillation_amplitude(series) == 6      # max(16,10,10,12) - min(...) = 16 - 10
    assert oscillation_amplitude([]) == 0.0
