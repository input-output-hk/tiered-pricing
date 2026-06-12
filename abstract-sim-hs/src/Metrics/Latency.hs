module Metrics.Latency (
  LatencyStats,
  BlockLatencyStats,
  latencyByUrgency,
  latencyByLane,
  latencyByUrgencyLane,
  blockLatencyByUrgency,
  blockLatencyByLane,
  blockLatencyByUrgencyLane,
) where

import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Metrics.Accumulator
import Metrics.Stats (DistStats, summarize)
import Transaction (Lane)
import Types (Duration (..), Urgency, diffSlots)

{- | Metric (3): inclusion latency, summarised over served demand units in the
bucket. Latency runs from the unit's *first* submission to on-chain
inclusion, so the waiting hidden inside rejected and retried attempts counts
against the design.
-}
type LatencyStats = DistStats Duration

{- | Inclusion latency measured in actual ranking blocks, from the demand
unit's first submission. Only 'Block.RankingBlockProduced' events advance the
count; EB announcements and certified EB summaries do not.
-}
type BlockLatencyStats = DistStats Int

latencyByUrgency :: MetricsAcc -> Map Urgency LatencyStats
latencyByUrgency acc =
  Map.fromList (fmap latencyForUrgency (observedUrgencies acc))
 where
  latencyForUrgency urgency =
    (urgency, summarizeLatencies (unitLatenciesWhere acc ((== urgency) . (.unitUrgency))))

latencyByLane :: MetricsAcc -> Map Lane LatencyStats
latencyByLane acc =
  Map.fromList (fmap latencyForLane allLanes)
 where
  latencyForLane lane =
    (lane, summarizeLatencies (unitLatenciesWhere acc ((== lane) . unitLane)))

latencyByUrgencyLane :: MetricsAcc -> Map (Urgency, Lane) LatencyStats
latencyByUrgencyLane acc =
  Map.fromList (fmap latencyForUrgencyLane (observedUrgencyLanes acc))
 where
  latencyForUrgencyLane key@(urgency, lane) =
    (key, summarizeLatencies (unitLatenciesWhere acc (matchesUnitUrgencyLane urgency lane)))

blockLatencyByUrgency :: MetricsAcc -> Map Urgency BlockLatencyStats
blockLatencyByUrgency acc =
  Map.fromList (fmap latencyForUrgency (observedUrgencies acc))
 where
  latencyForUrgency urgency =
    (urgency, summarize (unitBlockLatenciesWhere acc ((== urgency) . (.unitUrgency))))

blockLatencyByLane :: MetricsAcc -> Map Lane BlockLatencyStats
blockLatencyByLane acc =
  Map.fromList (fmap latencyForLane allLanes)
 where
  latencyForLane lane =
    (lane, summarize (unitBlockLatenciesWhere acc ((== lane) . unitLane)))

blockLatencyByUrgencyLane :: MetricsAcc -> Map (Urgency, Lane) BlockLatencyStats
blockLatencyByUrgencyLane acc =
  Map.fromList (fmap latencyForUrgencyLane (observedUrgencyLanes acc))
 where
  latencyForUrgencyLane key@(urgency, lane) =
    (key, summarize (unitBlockLatenciesWhere acc (matchesUnitUrgencyLane urgency lane)))

unitLatenciesWhere :: MetricsAcc -> (DemandUnit -> Bool) -> [Duration]
unitLatenciesWhere acc predicate =
  [ diffSlots slot unit.unitFirstSubmitted
  | unit <- unitsWhere acc predicate
  , Just (UnitIncluded slot _ _ _) <- [unit.unitOutcome]
  ]

unitBlockLatenciesWhere :: MetricsAcc -> (DemandUnit -> Bool) -> [Int]
unitBlockLatenciesWhere acc predicate =
  [ max 0 (block - unit.unitFirstSubmittedBlock)
  | unit <- unitsWhere acc predicate
  , Just (UnitIncluded _ block _ _) <- [unit.unitOutcome]
  ]

summarizeLatencies :: [Duration] -> LatencyStats
summarizeLatencies =
  fmap Duration . summarize . fmap durationToInt

durationToInt :: Duration -> Int
durationToInt (Duration n) = n
