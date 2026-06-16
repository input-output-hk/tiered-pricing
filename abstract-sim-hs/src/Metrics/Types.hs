{- | Shared metric aliases, aggregate result, and metric configuration.

Individual metric-family result types and calculations live in their own
modules under "Metrics.*".
-}
module Metrics.Types (
  Metrics (..),
  MetricsConfig (..),
  metricsConfigDefault,

  -- * Sliced breakdowns
  ByUrgency,
  ByLane,
  ByUrgencyLane,
  InclusionStats (..),
  ValueOutcome (..),
  LatencyStats,
  BlockLatencyStats,

  -- * Aggregate measures
  PriceShock (..),
  PriceUpdate (..),
  Revenue (..),
  Throughput (..),
  RankingBlockCounts (..),
  PriceStability (..),
  DemandLoad (..),

  -- * Invariants
  InvariantBreach (..),
  InvariantKind (..),
) where

import Data.Map.Strict (Map)
import Metrics.Demand (DemandLoad (..))
import Metrics.Inclusion (InclusionStats (..))
import Metrics.Invariants (InvariantBreach (..), InvariantKind (..))
import Metrics.Latency (BlockLatencyStats, LatencyStats)
import Metrics.Price (PriceShock (..), PriceStability (..))
import Metrics.Revenue (Revenue (..))
import Metrics.Throughput (RankingBlockCounts (..), Throughput (..))
import Metrics.Value (ValueOutcome (..))
import Pricing (PriceUpdate (..))
import Transaction (Lane)
import Types (SlotNo, Urgency)

-- | A metric sliced by urgency class.
type ByUrgency a = Map Urgency a

-- | A metric sliced by submitted lane.
type ByLane a = Map Lane a

-- | A metric sliced by urgency class and submitted lane.
type ByUrgencyLane a = Map (Urgency, Lane) a

-- | Aggregate metrics for one simulation run.
data Metrics = Metrics
  { inclusion :: ByUrgency InclusionStats
  -- ^ (1) transaction inclusion, by urgency
  , value :: ByUrgency ValueOutcome
  -- ^ (2) retained\/lost value, by urgency
  , laneValue :: ByLane ValueOutcome
  -- ^ Diagnostic retained\/lost value, by submitted/serving lane
  , latency :: ByUrgency LatencyStats
  -- ^ (3) inclusion latency, by urgency
  , actualBlockLatency :: ByUrgency BlockLatencyStats
  -- ^ (3) inclusion latency in actual ranking blocks, by urgency
  , laneInclusion :: ByLane InclusionStats
  -- ^ Diagnostic transaction inclusion, by submitted lane
  , laneLatency :: ByLane LatencyStats
  -- ^ Diagnostic inclusion latency, by submitted lane
  , laneActualBlockLatency :: ByLane BlockLatencyStats
  -- ^ Diagnostic inclusion latency in actual ranking blocks, by submitted lane
  , urgencyLaneInclusion :: ByUrgencyLane InclusionStats
  -- ^ Diagnostic transaction inclusion, by urgency and submitted lane
  , urgencyLaneValue :: ByUrgencyLane ValueOutcome
  -- ^ Diagnostic retained\/lost value, by urgency and submitted/serving lane
  , urgencyLaneLatency :: ByUrgencyLane LatencyStats
  -- ^ Diagnostic inclusion latency, by urgency and submitted lane
  , urgencyLaneActualBlockLatency :: ByUrgencyLane BlockLatencyStats
  -- ^ Diagnostic inclusion latency in actual ranking blocks, by urgency and submitted lane
  , priceShock :: PriceShock
  -- ^ (4) price shock
  , priceChanges :: [(SlotNo, PriceUpdate)]
  -- ^ Dynamic price update trace, in event order
  , revenue :: Revenue
  -- ^ (5) revenue\/fees + refunds
  , throughput :: Throughput
  -- ^ (6) aggregate throughput \/ EB utilization
  , rankingBlocks :: RankingBlockCounts
  -- ^ Diagnostic counts of tx-containing and EB-certifying RBs
  , priceStability :: PriceStability
  -- ^ (7) price convergence\/oscillation
  , invariantBreaches :: [InvariantBreach]
  -- ^ (8) invariant breaches
  , demandLoad :: DemandLoad
  -- ^ Diagnostic per-attempt load: retry amplification and fee-bump cost
  }
  deriving (Eq, Show)

data MetricsConfig = MetricsConfig
  { metricsPriceConvergenceBandPct :: Double
  }

metricsConfigDefault :: MetricsConfig
metricsConfigDefault =
  MetricsConfig
    { metricsPriceConvergenceBandPct = 0.05
    }
