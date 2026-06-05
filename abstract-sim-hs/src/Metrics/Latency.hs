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
import Transaction (Lane, Tx (..))
import Types (Duration (..), Urgency)

{- | Metric (3): inclusion latency, summarised over included txs in the bucket.
Latency is slots between submission and on-chain inclusion.
-}
data LatencyStats = LatencyStats
  { latencyCount :: Int
  -- ^ number of included txs contributing to this summary
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

{- | Inclusion latency measured in actual ranking blocks.

Only 'RankingBlockProduced' events advance this count; EB announcements and
certified EB summaries do not.
-}
data BlockLatencyStats = BlockLatencyStats
  { blockLatencyCount :: Int
  -- ^ number of included txs contributing to this summary
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
    (urgency, summarizeLatencies (latenciesWhere acc ((== urgency) . txUrgency)))

latencyByLane :: MetricsAcc -> Map Lane LatencyStats
latencyByLane acc =
  Map.fromList (fmap latencyForLane allLanes)
 where
  latencyForLane lane =
    (lane, summarizeLatencies (latenciesWhere acc ((== lane) . txLane)))

latencyByUrgencyLane :: MetricsAcc -> Map (Urgency, Lane) LatencyStats
latencyByUrgencyLane acc =
  Map.fromList (fmap latencyForUrgencyLane (observedUrgencyLanes acc))
 where
  latencyForUrgencyLane key@(urgency, lane) =
    (key, summarizeLatencies (latenciesWhere acc (matchesUrgencyLane urgency lane)))

blockLatencyByUrgency :: MetricsAcc -> Map Urgency BlockLatencyStats
blockLatencyByUrgency acc =
  Map.fromList (fmap latencyForUrgency (observedUrgencies acc))
 where
  latencyForUrgency urgency =
    (urgency, summarizeBlockLatencies (blockLatenciesWhere acc ((== urgency) . txUrgency)))

blockLatencyByLane :: MetricsAcc -> Map Lane BlockLatencyStats
blockLatencyByLane acc =
  Map.fromList (fmap latencyForLane allLanes)
 where
  latencyForLane lane =
    (lane, summarizeBlockLatencies (blockLatenciesWhere acc ((== lane) . txLane)))

blockLatencyByUrgencyLane :: MetricsAcc -> Map (Urgency, Lane) BlockLatencyStats
blockLatencyByUrgencyLane acc =
  Map.fromList (fmap latencyForUrgencyLane (observedUrgencyLanes acc))
 where
  latencyForUrgencyLane key@(urgency, lane) =
    (key, summarizeBlockLatencies (blockLatenciesWhere acc (matchesUrgencyLane urgency lane)))

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
