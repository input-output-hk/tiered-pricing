module Metrics.Latency (
  LatencyStats (..),
  BlockLatencyStats (..),
  latencyByUrgency,
  latencyByLane,
  latencyByUrgencyLane,
  blockLatencyByUrgency,
  blockLatencyByLane,
  blockLatencyByUrgencyLane,
) where

import Data.List (sort)
import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Metrics.Accumulator
import Transaction (Lane)
import Types (Duration (..), Urgency, diffSlots)

{- | Metric (3): inclusion latency, summarised over served demand units in the
bucket. Latency runs from the unit's *first* submission to on-chain
inclusion, so the waiting hidden inside rejected and retried attempts counts
against the design.
-}
data LatencyStats = LatencyStats
  { latencyCount :: Int
  -- ^ number of served units contributing to this summary
  , latencyMean :: Double
  -- ^ mean inclusion latency, in slots
  , latencyMedian :: Duration
  -- ^ median inclusion latency
  , latencyP95 :: Duration
  -- ^ 95th-percentile inclusion latency
  , latencyMax :: Duration
  -- ^ worst-case inclusion latency
  }
  deriving (Eq, Show)

{- | Inclusion latency measured in actual ranking blocks, from the demand
unit's first submission. Only 'Block.RankingBlockProduced' events advance the
count; EB announcements and certified EB summaries do not.
-}
data BlockLatencyStats = BlockLatencyStats
  { blockLatencyCount :: Int
  -- ^ number of served units contributing to this summary
  , blockLatencyMean :: Double
  -- ^ mean inclusion latency, in actual ranking blocks
  , blockLatencyMedian :: Int
  -- ^ median inclusion latency, in actual ranking blocks
  , blockLatencyP95 :: Int
  -- ^ 95th-percentile inclusion latency, in actual ranking blocks
  , blockLatencyMax :: Int
  -- ^ worst-case inclusion latency, in actual ranking blocks
  }
  deriving (Eq, Show)

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
    (urgency, summarizeBlockLatencies (unitBlockLatenciesWhere acc ((== urgency) . (.unitUrgency))))

blockLatencyByLane :: MetricsAcc -> Map Lane BlockLatencyStats
blockLatencyByLane acc =
  Map.fromList (fmap latencyForLane allLanes)
 where
  latencyForLane lane =
    (lane, summarizeBlockLatencies (unitBlockLatenciesWhere acc ((== lane) . unitLane)))

blockLatencyByUrgencyLane :: MetricsAcc -> Map (Urgency, Lane) BlockLatencyStats
blockLatencyByUrgencyLane acc =
  Map.fromList (fmap latencyForUrgencyLane (observedUrgencyLanes acc))
 where
  latencyForUrgencyLane key@(urgency, lane) =
    (key, summarizeBlockLatencies (unitBlockLatenciesWhere acc (matchesUnitUrgencyLane urgency lane)))

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
summarizeLatencies durations =
  case sort (fmap durationToInt durations) of
    [] ->
      LatencyStats
        { latencyCount = 0
        , latencyMean = 0
        , latencyMedian = Duration 0
        , latencyP95 = Duration 0
        , latencyMax = Duration 0
        }
    xs ->
      LatencyStats
        { latencyCount = n
        , latencyMean = fromIntegral (sum xs) / fromIntegral n
        , latencyMedian = Duration (quantile 0.50 xs)
        , latencyP95 = Duration (quantile 0.95 xs)
        , latencyMax = Duration (last xs)
        }
     where
      n = length xs

durationToInt :: Duration -> Int
durationToInt (Duration n) = n

quantile :: Double -> [Int] -> Int
quantile q xs =
  xs !! index
 where
  n = length xs
  index = min (n - 1) (max 0 (ceiling (q * fromIntegral n) - 1))

summarizeBlockLatencies :: [Int] -> BlockLatencyStats
summarizeBlockLatencies latencies =
  case sort latencies of
    [] ->
      BlockLatencyStats
        { blockLatencyCount = 0
        , blockLatencyMean = 0
        , blockLatencyMedian = 0
        , blockLatencyP95 = 0
        , blockLatencyMax = 0
        }
    xs ->
      BlockLatencyStats
        { blockLatencyCount = n
        , blockLatencyMean = fromIntegral (sum xs) / fromIntegral n
        , blockLatencyMedian = quantile 0.50 xs
        , blockLatencyP95 = quantile 0.95 xs
        , blockLatencyMax = last xs
        }
     where
      n = length xs
