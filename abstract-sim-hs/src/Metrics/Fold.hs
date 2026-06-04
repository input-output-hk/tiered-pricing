{- | Final metric orchestration.

This module intentionally stays small: event accumulation lives in
"Metrics.Accumulator", public result shapes live in "Metrics.Types", and each
metric family owns its own calculation module.
-}
module Metrics.Fold (
  finalizeMetrics,
  fromEvents,
  fromEventsForSlots,
) where

import Metrics.Accumulator
import Metrics.Fairness (fairnessFrom)
import Metrics.Inclusion (inclusionByLane, inclusionByUrgency, inclusionByUrgencyLane)
import Metrics.Invariants (invariantBreachesFrom)
import Metrics.Latency (latencyByLane, latencyByUrgency, latencyByUrgencyLane)
import Metrics.Price (priceChangesFrom, priceShockFrom, priceStabilityFrom)
import Metrics.Revenue (revenueFrom)
import Metrics.Throughput (rankingBlocksFrom, throughputFrom)
import Metrics.Types
import Metrics.Value (valueByUrgency)
import Event (SimEvent)

fromEvents :: [SimEvent] -> Metrics
fromEvents events =
  fromEventsForSlots (observedSlots events) events

fromEventsForSlots :: Int -> [SimEvent] -> Metrics
fromEventsForSlots slots events =
  finalizeMetrics metricsConfigDefault slots (recordMetricsEvents emptyMetricsAcc events)

finalizeMetrics :: MetricsConfig -> Int -> MetricsAcc -> Metrics
finalizeMetrics metricsConfig slots acc =
  Metrics
    { inclusion = inclusionByUrgency acc
    , value = valueByUrgency acc
    , latency = latencyByUrgency acc
    , laneInclusion = inclusionByLane acc
    , laneLatency = latencyByLane acc
    , urgencyLaneInclusion = inclusionByUrgencyLane acc
    , urgencyLaneLatency = latencyByUrgencyLane acc
    , priceShock = priceShockFrom acc
    , priceChanges = priceChangesFrom acc
    , revenue = revenueFrom acc
    , throughput = throughputFrom slots acc
    , rankingBlocks = rankingBlocksFrom acc
    , fairness = fairnessFrom acc
    , priceStability =
        priceStabilityFrom
          metricsConfig.metricsLoad
          metricsConfig.metricsPriceConvergenceBandPct
          metricsConfig.metricsLoadChangePct
          slots
          acc
    , invariantBreaches = invariantBreachesFrom acc
    }
