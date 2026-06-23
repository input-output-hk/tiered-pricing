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


from simviz.price import (
    price_at_or_before,
    convergence_for_lane,
    oscillation_reversals,
    oscillation_stats,
    settled_coefficient_range,
)


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


def test_settled_coefficient_range_excludes_transient():
    series = [
        {"slot": 10, "oldCoeff": 1.0, "newCoeff": 2.0},
        {"slot": 20, "oldCoeff": 2.0, "newCoeff": 4.0},
        {"slot": 30, "oldCoeff": 4.0, "newCoeff": 8.0},
        {"slot": 40, "oldCoeff": 8.0, "newCoeff": 8.25},
        {"slot": 50, "oldCoeff": 8.25, "newCoeff": 8.0},
        {"slot": 60, "oldCoeff": 8.0, "newCoeff": 8.25},
    ]
    assert settled_coefficient_range(series, 0.05) == 0.25
    assert settled_coefficient_range([], 0.05) == 0.0


def test_settled_coefficient_range_reports_full_range_when_unsettled():
    series = [
        {"slot": 10, "oldCoeff": 1.0, "newCoeff": 9.0},
        {"slot": 20, "oldCoeff": 9.0, "newCoeff": 1.0},
        {"slot": 30, "oldCoeff": 1.0, "newCoeff": 9.0},
        {"slot": 40, "oldCoeff": 9.0, "newCoeff": 1.0},
        {"slot": 50, "oldCoeff": 1.0, "newCoeff": 9.0},
    ]
    assert settled_coefficient_range(series, 0.05) == 8.0


def test_oscillation_stats_empty_and_monotone():
    assert oscillation_stats([], 0.05) == {
        "oscillationReversalCount": 0,
        "oscillationCycleCount": 0,
        "maxOscillationAmplitude": 0.0,
        "oscillationExcessTravel": 0.0,
    }
    series = [
        {"slot": 10, "oldCoeff": 1.0, "newCoeff": 2.0},
        {"slot": 20, "oldCoeff": 2.0, "newCoeff": 4.0},
        {"slot": 30, "oldCoeff": 4.0, "newCoeff": 8.0},
    ]
    assert oscillation_stats(series, 0.05)["oscillationCycleCount"] == 0
    assert oscillation_stats(series, 0.05)["oscillationReversalCount"] == 0


def test_oscillation_stats_burst_recovery_is_not_completed_cycle():
    series = [
        {"slot": 10, "oldCoeff": 1.0, "newCoeff": 2.0},
        {"slot": 20, "oldCoeff": 2.0, "newCoeff": 1.0},
    ]
    stats = oscillation_stats(series, 0.05)
    assert stats["oscillationReversalCount"] == 1
    assert stats["oscillationCycleCount"] == 0
    assert stats["maxOscillationAmplitude"] == 1.0
    assert round(stats["oscillationExcessTravel"], 12) == round(2 * 0.6931471805599453, 12)


def test_oscillation_stats_repeated_reversal_counts_cycle():
    series = [
        {"slot": 10, "oldCoeff": 1.0, "newCoeff": 2.0},
        {"slot": 20, "oldCoeff": 2.0, "newCoeff": 1.0},
        {"slot": 30, "oldCoeff": 1.0, "newCoeff": 2.0},
    ]
    stats = oscillation_stats(series, 0.05)
    assert stats["oscillationReversalCount"] == 2
    assert stats["oscillationCycleCount"] == 1
    assert stats["maxOscillationAmplitude"] == 1.0


def test_oscillation_reversal_markers_track_collapsed_segments():
    series = [
        {"slot": 10, "oldCoeff": 1.0, "newCoeff": 1.5},
        {"slot": 20, "oldCoeff": 1.5, "newCoeff": 2.0},
        {"slot": 30, "oldCoeff": 2.0, "newCoeff": 1.0},
        {"slot": 40, "oldCoeff": 1.0, "newCoeff": 0.9},
        {"slot": 50, "oldCoeff": 0.9, "newCoeff": 1.8},
    ]
    assert oscillation_reversals(series, 0.05) == [
        {"slot": 30, "coeff": 2.0, "fromDirection": "up", "toDirection": "down"},
        {"slot": 50, "coeff": 0.9, "fromDirection": "down", "toDirection": "up"},
    ]


def test_oscillation_stats_ignores_deadband_moves():
    series = [
        {"slot": 10, "oldCoeff": 1.0, "newCoeff": 1.04},
        {"slot": 20, "oldCoeff": 1.04, "newCoeff": 1.0},
        {"slot": 30, "oldCoeff": 1.0, "newCoeff": 2.0},
    ]
    assert oscillation_stats(series, 0.05)["oscillationReversalCount"] == 0
    assert oscillation_reversals(series, 0.05) == []
