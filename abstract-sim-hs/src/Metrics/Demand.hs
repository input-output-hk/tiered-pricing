{- | Per-attempt load diagnostics. The headline metrics count demand units;
this family counts what serving (or failing to serve) those units cost the
network in submission attempts and fee bumps.
-}
module Metrics.Demand (
  DemandLoad (..),
  demandLoadFrom,
) where

import Data.Map.Strict qualified as Map
import Data.Maybe (isNothing)
import Metrics.Accumulator
import Types (Lovelace (..))

data DemandLoad = DemandLoad
  { demandUnits :: Int
  -- ^ distinct demand units observed
  , unitsServed :: Int
  , unitsAbandoned :: Int
  , unitsUnresolved :: Int
  -- ^ units still in flight when the run ended
  , submissionAttempts :: Int
  -- ^ total submissions across all units; @submissionAttempts \/
  -- demandUnits@ is the load-amplification factor of retry behaviour
  , attemptsMax :: Int
  -- ^ most attempts any single unit made
  , postedFeeGrowthMean :: Double
  -- ^ served units only: posted fee of the serving attempt over the first
  -- attempt's posted fee — 1.0 means no fee bump was needed to get served
  }
  deriving (Eq, Show)

demandLoadFrom :: MetricsAcc -> DemandLoad
demandLoadFrom acc =
  DemandLoad
    { demandUnits = length units
    , unitsServed = length served
    , unitsAbandoned = length abandoned
    , unitsUnresolved = length (filter (isNothing . (.unitOutcome)) units)
    , submissionAttempts = sum (fmap (.unitAttempts) units)
    , attemptsMax = maximum (0 : fmap (.unitAttempts) units)
    , postedFeeGrowthMean = mean postedFeeGrowths
    }
 where
  units = Map.elems acc.accUnits
  served = filter unitServed units
  abandoned =
    [unit | unit <- units, Just (UnitAbandoned _) <- [unit.unitOutcome]]
  postedFeeGrowths =
    [ fromInteger serving / fromInteger first
    | unit <- served
    , Lovelace serving <- maybe [] pure unit.unitServingPostedFee
    , let Lovelace first = unit.unitFirstPostedFee
    , first > 0
    ]
