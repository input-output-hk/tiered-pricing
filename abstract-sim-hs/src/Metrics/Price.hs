module Metrics.Price (
  PriceShock (..),
  PriceStability (..),
  priceShockFrom,
  priceChangesFrom,
  priceStabilityFrom,
) where

import Data.List (find, sortOn)
import Data.Map.Strict qualified as Map
import Data.Maybe (isNothing, listToMaybe)
import Load (ArrivalProcess, arrivalRateAt)
import Metrics.Accumulator
import Metrics.Stats (maximumOrZero)
import Pricing (PriceUpdate (..))
import Types (Duration (..), SlotNo (..), diffSlots)

-- | Metric (4): price shock — how violently the dynamic price moved.
data PriceShock = PriceShock
  { maxPriceJump :: Double
  -- ^ largest single-step relative price increase over the run
  , shockCount :: Int
  -- ^ number of steps whose jump exceeded the shock threshold
  }
  deriving (Eq, Show)

-- | Metric (7): price convergence and oscillation.
data PriceStability = PriceStability
  { convergenceTime :: Maybe Duration
  -- ^ slots until price entered and stayed within the band; 'Nothing' if it never converged
  , oscillationAmplitude :: Double
  -- ^ steady-state peak-to-peak price oscillation
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

priceStabilityFrom :: ArrivalProcess -> Double -> Double -> Int -> MetricsAcc -> PriceStability
priceStabilityFrom load bandPct loadChangePct slots acc =
  PriceStability
    { convergenceTime = convergenceTimeFrom load bandPct loadChangePct slots acc
    , oscillationAmplitude = maximumOrZero (fmap amplitude (Map.elems coeffsByLane))
    }
 where
  coeffsByLane =
    Map.fromListWith
      (<>)
      (concatMap laneCoeffs acc.accPriceChanges)

  laneCoeffs (_, update) =
    [ (update.priceUpdateLane, [update.priceUpdateOldCoeff])
    , (update.priceUpdateLane, [update.priceUpdateNewCoeff])
    ]

  amplitude coeffs =
    maximum coeffs - minimum coeffs

convergenceTimeFrom :: ArrivalProcess -> Double -> Double -> Int -> MetricsAcc -> Maybe Duration
convergenceTimeFrom load bandPct loadChangePct slots acc
  | slots <= 0 = Nothing
  | null convergenceResults = Nothing
  | any isNothing convergenceResults = Nothing
  | otherwise = maximum <$> sequence convergenceResults
 where
  regimes =
    loadRegimes load loadChangePct slots
  laneChanges lane =
    sortOn fst (filter ((== lane) . (.priceUpdateLane) . snd) acc.accPriceChanges)
  convergenceResults =
    concatMap convergenceForLane allLanes

  convergenceForLane lane =
    let changes = laneChanges lane
     in if null changes
          then []
          else fmap (convergenceInRegime bandPct changes) regimes

data LoadRegime = LoadRegime
  { regimeStart :: SlotNo
  , regimeEnd :: SlotNo
  }

loadRegimes :: ArrivalProcess -> Double -> Int -> [LoadRegime]
loadRegimes load changePct slots
  | slots <= 0 = []
  | otherwise = reverse (finish currentStart slots acc)
 where
  slotNumbers = [1 .. slots - 1]
  (_, currentStart, acc) =
    foldl advance (arrivalRateAt load (SlotNo 0), 0, []) slotNumbers

  advance (prevRate, start, regimes) slot =
    let currentRate = arrivalRateAt load (SlotNo slot)
     in if materialLoadChange changePct prevRate currentRate
          then (currentRate, slot, LoadRegime (SlotNo start) (SlotNo slot) : regimes)
          else (currentRate, start, regimes)

  finish start end regimes =
    LoadRegime (SlotNo start) (SlotNo end) : regimes

materialLoadChange :: Double -> Double -> Double -> Bool
materialLoadChange changePct oldRate newRate
  | oldRate == newRate = False
  | oldRate <= 0 = newRate > 0
  | otherwise =
      abs (newRate - oldRate) / oldRate > max 0 changePct

convergenceInRegime :: Double -> [(SlotNo, PriceUpdate)] -> LoadRegime -> Maybe Duration
convergenceInRegime bandPct changes regime
  | regimeEnd regime <= regimeStart regime = Nothing
  | otherwise = do
      reference <- priceAtOrBefore changes (previousSlot (regimeEnd regime))
      convergedAt <- find (convergesFrom reference) candidateSlots
      pure (diffSlots convergedAt (regimeStart regime))
 where
  changesInRegime =
    filter (changeInRegime regime) changes
  candidateSlots =
    regimeStart regime : fmap fst changesInRegime

  convergesFrom reference candidate =
    case priceAtOrBefore changes candidate of
      Nothing -> False
      Just candidatePrice ->
        let futurePrices =
              fmap ((.priceUpdateNewCoeff) . snd) $
                filter ((> candidate) . fst) changesInRegime
         in all (withinBand bandPct reference) (candidatePrice : futurePrices)

changeInRegime :: LoadRegime -> (SlotNo, PriceUpdate) -> Bool
changeInRegime regime (slot, _) =
  slot >= regimeStart regime
    && slot < regimeEnd regime

priceAtOrBefore :: [(SlotNo, PriceUpdate)] -> SlotNo -> Maybe Double
priceAtOrBefore changes slot =
  case filter ((<= slot) . fst) changes of
    [] -> (.priceUpdateOldCoeff) . snd <$> listToMaybe changes
    priorChanges -> Just ((.priceUpdateNewCoeff) (snd (last priorChanges)))

previousSlot :: SlotNo -> SlotNo
previousSlot (SlotNo slot) =
  SlotNo (max 0 (slot - 1))

withinBand :: Double -> Double -> Double -> Bool
withinBand bandPct reference price =
  abs (price - reference) <= abs reference * max 0 bandPct

priceShockThreshold :: Double
priceShockThreshold = 0.10
