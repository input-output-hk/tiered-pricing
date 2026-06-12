module Metrics.Inclusion (
  InclusionStats (..),
  inclusionStats,
) where

import Metrics.Accumulator (DemandUnit, unitServed)

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

inclusionStats :: [DemandUnit] -> InclusionStats
inclusionStats units =
  InclusionStats
    { submitted = length units
    , included = length (filter unitServed units)
    }
