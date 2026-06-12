module Metrics.Price (
  PriceShock (..),
  PriceStability (..),
  priceShockFrom,
  priceChangesFrom,
  priceStabilityFrom,
) where

import Data.List (tails)
import Data.List.NonEmpty (NonEmpty (..))
import Data.List.NonEmpty qualified as NE
import Data.Map.Strict qualified as Map
import Data.Maybe (listToMaybe)
import Metrics.Accumulator
import Metrics.Stats (maximumOrZero)
import Pricing (PriceUpdate (..))
import Types (Duration (..), Lane, SlotNo (..), diffSlots)

-- | Metric (4): price shock — how violently the dynamic price moved.
data PriceShock = PriceShock
  { maxPriceJump :: Double
  -- ^ largest single-step relative price increase over the run
  , shockCount :: Int
  -- ^ number of steps whose jump exceeded the shock threshold
  }
  deriving (Eq, Show)

{- | Metric (7): price convergence and oscillation, judged per lane against
the lane's final coefficient — its steady state, if it has one.
-}
data PriceStability = PriceStability
  { convergenceTime :: Maybe Duration
  -- ^ slots from run start until every lane's price entered the band around
  -- its final coefficient and stayed there; 'Nothing' if some lane was still
  -- out of band at its last update, or if no lane changed price at all
  , oscillationAmplitude :: Double
  -- ^ peak-to-peak price movement after settling — the residual in-band
  -- ripple; a lane that never settled reports its full-run swing instead
  }
  deriving (Eq, Show)

priceShockFrom :: MetricsAcc -> PriceShock
priceShockFrom acc =
  PriceShock
    { maxPriceJump = maximumOrZero jumps
    , shockCount = length (filter (> priceShockThreshold) jumps)
    }
 where
  jumps =
    [ relativeJump update.priceUpdateOldCoeff update.priceUpdateNewCoeff
    | (_, update) <- acc.accPriceChanges
    ]

relativeJump :: Double -> Double -> Double
relativeJump oldCoeff newCoeff
  | oldCoeff <= 0 = 0
  | otherwise = abs (newCoeff - oldCoeff) / oldCoeff

-- | The dynamic price update trace, in event order.
priceChangesFrom :: MetricsAcc -> [(SlotNo, PriceUpdate)]
priceChangesFrom acc =
  reverse acc.accPriceChanges

priceStabilityFrom :: Double -> MetricsAcc -> PriceStability
priceStabilityFrom bandPct acc =
  PriceStability
    { convergenceTime =
        case traverse (.laneSettledAt) stabilities of
          Just settledSlots@(_ : _) -> Just (diffSlots (maximum settledSlots) (SlotNo 0))
          _ -> Nothing
    , oscillationAmplitude = maximumOrZero (fmap (.laneAmplitude) stabilities)
    }
 where
  stabilities =
    fmap (laneStability bandPct) (Map.elems (changesByLane acc))

changesByLane :: MetricsAcc -> Map.Map Lane (NonEmpty (SlotNo, PriceUpdate))
changesByLane acc =
  NE.sortWith fst
    <$> Map.fromListWith
      (<>)
      [ (update.priceUpdateLane, (slot, update) :| [])
      | (slot, update) <- acc.accPriceChanges
      ]

data LaneStability = LaneStability
  { laneSettledAt :: Maybe SlotNo
  , laneAmplitude :: Double
  }

{- | Settling against a non-empty, slot-ordered lane trace. The coefficient
path runs from the first update's starting coefficient through each new
coefficient; the lane settles at the earliest point from which every later
coefficient stays within the band around the final one. The final coefficient
is trivially in band, so settling requires an earlier point to qualify;
otherwise the lane was still moving when last updated and reports its full
swing as amplitude.
-}
laneStability :: Double -> NonEmpty (SlotNo, PriceUpdate) -> LaneStability
laneStability bandPct changes =
  LaneStability
    { laneSettledAt = fst <$> settledTail
    , laneAmplitude = peakToPeak (fmap snd (maybe coeffPath snd settledTail))
    }
 where
  -- (slot, coefficient) at each candidate settling point, oldest first: the
  -- pre-trace coefficient at slot 0, then each update's new coefficient.
  coeffPath =
    (SlotNo 0, (snd (NE.head changes)).priceUpdateOldCoeff)
      : [(slot, update.priceUpdateNewCoeff) | (slot, update) <- NE.toList changes]
  finalCoeff = snd (last coeffPath)

  -- A settling point needs at least one later coefficient to judge: the lone
  -- final coefficient is trivially in band against itself, so candidates of
  -- length < 2 (it, and the empty tail) cannot count as settling.
  settledTail =
    listToMaybe
      [ (slot, suffix)
      | suffix@((slot, _) : _ : _) <- tails coeffPath
      , all (withinBand bandPct finalCoeff . snd) suffix
      ]

peakToPeak :: [Double] -> Double
peakToPeak coeffs =
  maximum coeffs - minimum coeffs

withinBand :: Double -> Double -> Double -> Bool
withinBand bandPct reference price =
  abs (price - reference) <= abs reference * max 0 bandPct

priceShockThreshold :: Double
priceShockThreshold = 0.10
