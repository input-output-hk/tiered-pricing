module Metrics.Price (
  PriceShock (..),
  PriceChange (..),
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
import Transaction (Lane)
import Types (Duration (..), SlotNo (..), diffSlots)

-- | Metric (4): price shock — how violently the dynamic price moved.
data PriceShock = PriceShock
  { maxPriceJump :: Double
  -- ^ largest single-step relative price increase over the run
  , shockCount :: Int
  -- ^ number of steps whose jump exceeded the shock threshold
  }
  deriving (Eq, Show)

-- | One dynamic price controller update, preserved from the event stream.
data PriceChange = PriceChange
  { priceChangeSlot :: SlotNo
  , priceChangeLane :: Lane
  , priceChangeOldCoeff :: Double
  , priceChangeNewCoeff :: Double
  , priceChangeUtilisation :: Double
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
    { maxPriceJump = maximumOrZero acc.accPriceJumps
    , shockCount = length (filter (> priceShockThreshold) acc.accPriceJumps)
    }

priceChangesFrom :: MetricsAcc -> [PriceChange]
priceChangesFrom acc =
  fmap toPriceChange (reverse acc.accPriceChanges)

toPriceChange :: AccPriceChange -> PriceChange
toPriceChange change =
  PriceChange
    { priceChangeSlot = change.accPriceChangeSlot
    , priceChangeLane = change.accPriceChangeLane
    , priceChangeOldCoeff = change.accPriceChangeOldCoeff
    , priceChangeNewCoeff = change.accPriceChangeNewCoeff
    , priceChangeUtilisation = change.accPriceChangeUtilisation
    }

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
      (concatMap priceChangeLaneCoeffs acc.accPriceChanges)

  priceChangeLaneCoeffs change =
    [ (change.accPriceChangeLane, [change.accPriceChangeOldCoeff])
    , (change.accPriceChangeLane, [change.accPriceChangeNewCoeff])
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
    sortOn accPriceChangeSlot (filter ((== lane) . accPriceChangeLane) acc.accPriceChanges)
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

convergenceInRegime :: Double -> [AccPriceChange] -> LoadRegime -> Maybe Duration
convergenceInRegime bandPct changes regime
  | regimeEnd regime <= regimeStart regime = Nothing
  | otherwise = do
      reference <- priceAtOrBefore changes (previousSlot (regimeEnd regime))
      convergedAt <- find (convergesFrom reference) candidateSlots
      pure (diffSlots convergedAt (regimeStart regime))
 where
  changesInRegime =
    filter (priceChangeInRegime regime) changes
  candidateSlots =
    regimeStart regime : fmap accPriceChangeSlot changesInRegime

  convergesFrom reference candidate =
    case priceAtOrBefore changes candidate of
      Nothing -> False
      Just candidatePrice ->
        let futurePrices =
              fmap accPriceChangeNewCoeff $
                filter ((> candidate) . accPriceChangeSlot) changesInRegime
         in all (withinBand bandPct reference) (candidatePrice : futurePrices)

priceChangeInRegime :: LoadRegime -> AccPriceChange -> Bool
priceChangeInRegime regime change =
  accPriceChangeSlot change >= regimeStart regime
    && accPriceChangeSlot change < regimeEnd regime

priceAtOrBefore :: [AccPriceChange] -> SlotNo -> Maybe Double
priceAtOrBefore changes slot =
  case filter ((<= slot) . accPriceChangeSlot) changes of
    [] -> accPriceChangeOldCoeff <$> listToMaybe changes
    priorChanges -> Just (accPriceChangeNewCoeff (last priorChanges))

previousSlot :: SlotNo -> SlotNo
previousSlot (SlotNo slot) =
  SlotNo (max 0 (slot - 1))

withinBand :: Double -> Double -> Double -> Bool
withinBand bandPct reference price =
  abs (price - reference) <= abs reference * max 0 bandPct

priceShockThreshold :: Double
priceShockThreshold = 0.10
