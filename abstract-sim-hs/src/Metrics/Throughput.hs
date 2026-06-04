module Metrics.Throughput (
  Throughput (..),
  RankingBlockCounts (..),
  throughputFrom,
  rankingBlocksFrom,
) where

import Block (BlockSummary (..), EndorserBlockSummary (..), RankingBlock (..), RankingBlockSummary (..))
import Data.Map.Strict qualified as Map
import Data.Maybe (mapMaybe)
import Metrics.Accumulator

-- | Metric (6): aggregate throughput and EB utilization.
data Throughput = Throughput
  { txThroughput :: Double
  -- ^ included txs per slot, averaged over the run
  , ebUtilization :: Double
  -- ^ mean fraction of EB capacity filled, in [0,1]
  }
  deriving (Eq, Show)

-- | Diagnostic RB composition counts.
data RankingBlockCounts = RankingBlockCounts
  { txContainingRbs :: Int
  -- ^ Praos ranking blocks whose body contained at least one tx
  , ebCertifyingRbs :: Int
  -- ^ ranking blocks whose body was an EB certificate
  }
  deriving (Eq, Show)

throughputFrom :: Int -> MetricsAcc -> Throughput
throughputFrom slots acc =
  Throughput
    { txThroughput =
        if slots <= 0
          then 0
          else fromIntegral (Map.size acc.accIncludedAt) / fromIntegral slots
    , ebUtilization = mean (fmap ebUtilization ebSummaries)
    }
 where
  ebSummaries =
    mapMaybe endorserBlockSummary acc.accBlocks
  ebUtilization summary =
    ratio
      (endorserBlockUsedBytes summary)
      (endorserBlockCapacityBytes summary)

endorserBlockSummary :: BlockSummary -> Maybe EndorserBlockSummary
endorserBlockSummary (EndorserBlockAnnounced summary) = Just summary
endorserBlockSummary _ = Nothing

rankingBlocksFrom :: MetricsAcc -> RankingBlockCounts
rankingBlocksFrom acc =
  foldl' countRankingBlock emptyRankingBlockCounts acc.accBlocks

emptyRankingBlockCounts :: RankingBlockCounts
emptyRankingBlockCounts =
  RankingBlockCounts
    { txContainingRbs = 0
    , ebCertifyingRbs = 0
    }

countRankingBlock :: RankingBlockCounts -> BlockSummary -> RankingBlockCounts
countRankingBlock counts (RankingBlockProduced summary) =
  case summary.rankingBlock of
    PraosBlock txIds
      | null txIds -> counts
      | otherwise -> counts{txContainingRbs = counts.txContainingRbs + 1}
    CertifyingBlock{} ->
      counts{ebCertifyingRbs = counts.ebCertifyingRbs + 1}
countRankingBlock counts _ =
  counts
