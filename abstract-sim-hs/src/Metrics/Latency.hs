module Metrics.Latency (
  LatencyStats,
  BlockLatencyStats,
  latencyStats,
  blockLatencyStats,
) where

import Metrics.Accumulator (DemandUnit (..), UnitOutcome (..))
import Metrics.Stats (DistStats, summarize)
import Types (Duration (..), diffSlots)

{- | Metric (3): inclusion latency, summarised over the served demand units in
a bucket. Latency runs from the unit's *first* submission to on-chain
inclusion, so the waiting hidden inside rejected and retried attempts counts
against the design.
-}
type LatencyStats = DistStats Duration

{- | Inclusion latency measured in actual ranking blocks, from the demand
unit's first submission. Only 'Block.RankingBlockProduced' events advance the
count; EB announcements and certified EB summaries do not.
-}
type BlockLatencyStats = DistStats Int

latencyStats :: [DemandUnit] -> LatencyStats
latencyStats units =
  fmap Duration . summarize $
    [ durationToInt (diffSlots slot unit.unitFirstSubmitted)
    | unit <- units
    , Just (UnitIncluded slot _ _ _) <- [unit.unitOutcome]
    ]

blockLatencyStats :: [DemandUnit] -> BlockLatencyStats
blockLatencyStats units =
  summarize
    [ max 0 (block - unit.unitFirstSubmittedBlock)
    | unit <- units
    , Just (UnitIncluded _ block _ _) <- [unit.unitOutcome]
    ]

durationToInt :: Duration -> Int
durationToInt (Duration n) = n
