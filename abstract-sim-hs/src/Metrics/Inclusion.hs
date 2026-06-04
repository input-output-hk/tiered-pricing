module Metrics.Inclusion (
  InclusionStats (..),
  inclusionByUrgency,
  inclusionByLane,
  inclusionByUrgencyLane,
) where

import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Metrics.Accumulator
import Transaction (Lane, Tx (..))
import Types (Urgency)

{- | Metric (1): transaction inclusion. Raw counts; the inclusion rate is left
to the consumer as @included \/ submitted@ to avoid storing a derived value.
-}
data InclusionStats = InclusionStats
  { submitted :: Int
  -- ^ txs submitted in this urgency bucket
  , included :: Int
  -- ^ txs that reached the chain (an RB body or a certified EB)
  }
  deriving (Eq, Show)

inclusionByUrgency :: MetricsAcc -> Map Urgency InclusionStats
inclusionByUrgency acc =
  Map.fromList (fmap inclusionForUrgency (observedUrgencies acc))
 where
  inclusionForUrgency urgency =
    (urgency, inclusionStatsWhere acc ((== urgency) . txUrgency))

inclusionByLane :: MetricsAcc -> Map Lane InclusionStats
inclusionByLane acc =
  Map.fromList (fmap inclusionForLane allLanes)
 where
  inclusionForLane lane =
    (lane, inclusionStatsWhere acc ((== lane) . txLane))

inclusionByUrgencyLane :: MetricsAcc -> Map (Urgency, Lane) InclusionStats
inclusionByUrgencyLane acc =
  Map.fromList (fmap inclusionForUrgencyLane (observedUrgencyLanes acc))
 where
  inclusionForUrgencyLane key@(urgency, lane) =
    (key, inclusionStatsWhere acc (matchesUrgencyLane urgency lane))

inclusionStatsWhere :: MetricsAcc -> (Tx -> Bool) -> InclusionStats
inclusionStatsWhere acc predicate =
  InclusionStats
    { submitted = length (submittedTxsWhere acc predicate)
    , included = length (includedTxsWhere acc predicate)
    }
