module Metrics.Inclusion (
  InclusionStats (..),
  inclusionByUrgency,
  inclusionByLane,
  inclusionByUrgencyLane,
) where

import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Metrics.Accumulator
import Transaction (Lane)
import Types (Urgency)

{- | Metric (1): demand-unit inclusion. A unit counts once however many
submission attempts it took, so @included \/ submitted@ is the service rate
of underlying demand, not of attempts ("Metrics.Demand" counts those). Raw
counts; the rate is left to the consumer.
-}
data InclusionStats = InclusionStats
  { submitted :: Int
  -- ^ demand units first submitted in this bucket
  , included :: Int
  -- ^ units that reached the chain (an RB body or a certified EB), on any
  -- attempt
  }
  deriving (Eq, Show)

inclusionByUrgency :: MetricsAcc -> Map Urgency InclusionStats
inclusionByUrgency acc =
  Map.fromList (fmap inclusionForUrgency (observedUrgencies acc))
 where
  inclusionForUrgency urgency =
    (urgency, inclusionStatsWhere acc ((== urgency) . (.unitUrgency)))

inclusionByLane :: MetricsAcc -> Map Lane InclusionStats
inclusionByLane acc =
  Map.fromList (fmap inclusionForLane allLanes)
 where
  inclusionForLane lane =
    (lane, inclusionStatsWhere acc ((== lane) . unitLane))

inclusionByUrgencyLane :: MetricsAcc -> Map (Urgency, Lane) InclusionStats
inclusionByUrgencyLane acc =
  Map.fromList (fmap inclusionForUrgencyLane (observedUrgencyLanes acc))
 where
  inclusionForUrgencyLane key@(urgency, lane) =
    (key, inclusionStatsWhere acc (matchesUnitUrgencyLane urgency lane))

inclusionStatsWhere :: MetricsAcc -> (DemandUnit -> Bool) -> InclusionStats
inclusionStatsWhere acc predicate =
  InclusionStats
    { submitted = length units
    , included = length (filter unitServed units)
    }
 where
  units = unitsWhere acc predicate
