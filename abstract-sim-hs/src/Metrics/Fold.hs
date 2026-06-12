{- | Final metric orchestration.

This module intentionally stays small: event accumulation lives in
"Metrics.Accumulator", public result shapes live in "Metrics.Types", each
metric family owns its calculation kernel, and "Metrics.Slice" owns how
kernels fan out over urgency/lane buckets.
-}
module Metrics.Fold (
  finalizeMetrics,
) where

import Metrics.Accumulator
import Metrics.Demand (demandLoadFrom)
import Metrics.Inclusion (inclusionStats)
import Metrics.Invariants (invariantBreachesFrom)
import Metrics.Latency (blockLatencyStats, latencyStats)
import Metrics.Price (priceChangesFrom, priceShockFrom, priceStabilityFrom)
import Metrics.Revenue (revenueFrom)
import Metrics.Slice (laneDim, sliceBy, urgencyDim, (>*<))
import Metrics.Throughput (rankingBlocksFrom, throughputFrom)
import Metrics.Types
import Metrics.Value (valueOutcome)

finalizeMetrics :: MetricsConfig -> Int -> MetricsAcc -> Metrics
finalizeMetrics metricsConfig slots acc =
  Metrics
    { inclusion = sliceBy urgencyDim inclusionStats acc
    , value = sliceBy urgencyDim valueOutcome acc
    , latency = sliceBy urgencyDim latencyStats acc
    , actualBlockLatency = sliceBy urgencyDim blockLatencyStats acc
    , laneInclusion = sliceBy laneDim inclusionStats acc
    , laneLatency = sliceBy laneDim latencyStats acc
    , laneActualBlockLatency = sliceBy laneDim blockLatencyStats acc
    , urgencyLaneInclusion = sliceBy (urgencyDim >*< laneDim) inclusionStats acc
    , urgencyLaneLatency = sliceBy (urgencyDim >*< laneDim) latencyStats acc
    , urgencyLaneActualBlockLatency = sliceBy (urgencyDim >*< laneDim) blockLatencyStats acc
    , priceShock = priceShockFrom acc
    , priceChanges = priceChangesFrom acc
    , revenue = revenueFrom acc
    , throughput = throughputFrom slots acc
    , rankingBlocks = rankingBlocksFrom acc
    , priceStability =
        priceStabilityFrom
          metricsConfig.metricsLoad
          metricsConfig.metricsPriceConvergenceBandPct
          metricsConfig.metricsLoadChangePct
          slots
          acc
    , invariantBreaches = invariantBreachesFrom acc
    , demandLoad = demandLoadFrom acc
    }
