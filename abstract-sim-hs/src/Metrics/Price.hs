module Metrics.Price (
  PriceShock (..),
  PriceOscillation (..),
  PriceStability (..),
  priceShockFrom,
  priceOscillationFrom,
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

{- | Price oscillation: significant repeated direction reversals in the dynamic
coefficient. This is separate from shock (single-step jump size) and settled
range (residual amplitude after convergence).
-}
data PriceOscillation = PriceOscillation
  { oscillationReversalCount :: Int
  -- ^ significant up/down or down/up direction changes, summed across lanes
  , oscillationCycleCount :: Int
  -- ^ completed oscillation cycles, summed per lane as @reversalCount div 2@
  , maxOscillationAmplitude :: Double
  -- ^ largest peak-to-trough coefficient range across any three consecutive
  -- segment endpoints
  , oscillationExcessTravel :: Double
  -- ^ significant log-price path length beyond the net endpoint movement
  }
  deriving (Eq, Show)

{- | Metric (7): price convergence and residual range, judged per lane against
the lane's final coefficient -- its steady state, if it has one.
-}
data PriceStability = PriceStability
  { convergenceTime :: Maybe Duration
  -- ^ slots from run start until every lane's price entered the band around
  -- its final coefficient and stayed there; 'Nothing' if some lane was still
  -- out of band at its last update, or if no lane changed price at all
  , settledCoefficientRange :: Double
  -- ^ peak-to-peak coefficient movement after settling -- the residual in-band
  -- range; a lane that never settled reports its full-run swing instead
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

priceOscillationFrom :: Double -> MetricsAcc -> PriceOscillation
priceOscillationFrom deadbandPct acc =
  foldl' combineOscillation emptyOscillation laneOscillations
 where
  laneOscillations =
    fmap (laneOscillation deadbandPct) (Map.elems (changesByLane acc))

combineOscillation :: PriceOscillation -> PriceOscillation -> PriceOscillation
combineOscillation a b =
  PriceOscillation
    { oscillationReversalCount =
        a.oscillationReversalCount + b.oscillationReversalCount
    , oscillationCycleCount =
        a.oscillationCycleCount + b.oscillationCycleCount
    , maxOscillationAmplitude =
        max a.maxOscillationAmplitude b.maxOscillationAmplitude
    , oscillationExcessTravel =
        a.oscillationExcessTravel + b.oscillationExcessTravel
    }

emptyOscillation :: PriceOscillation
emptyOscillation =
  PriceOscillation
    { oscillationReversalCount = 0
    , oscillationCycleCount = 0
    , maxOscillationAmplitude = 0
    , oscillationExcessTravel = 0
    }

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
    , settledCoefficientRange = maximumOrZero (fmap (.laneCoefficientRange) stabilities)
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
  , laneCoefficientRange :: Double
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
    , laneCoefficientRange = peakToPeak (fmap snd (maybe coeffPath snd settledTail))
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

data PriceDirection = PriceUp | PriceDown
  deriving (Eq, Show)

data PriceMove = PriceMove
  { moveDirection :: PriceDirection
  , moveOldCoeff :: Double
  , moveNewCoeff :: Double
  }
  deriving (Eq, Show)

laneOscillation :: Double -> NonEmpty (SlotNo, PriceUpdate) -> PriceOscillation
laneOscillation deadbandPct changes =
  case segmentEndpoints significantMoves of
    [] -> emptyOscillation
    [_] -> emptyOscillation
    endpoints ->
      PriceOscillation
        { oscillationReversalCount = reversalCount
        , oscillationCycleCount = reversalCount `div` 2
        , maxOscillationAmplitude =
            maximumOrZero (fmap tripleAmplitude (triples endpoints))
        , oscillationExcessTravel =
            max 0 (logTravel endpoints - netLogMovement endpoints)
        }
 where
  significantMoves =
    [ PriceMove direction oldCoeff newCoeff
    | (oldCoeff, newCoeff) <- adjacentPairs coeffPath
    , oldCoeff > 0
    , newCoeff > 0
    , relativeJump oldCoeff newCoeff > max 0 deadbandPct
    , Just direction <- [priceDirection oldCoeff newCoeff]
    ]
  reversalCount = max 0 (length (compressedDirections significantMoves) - 1)
  coeffPath =
    (snd (NE.head changes)).priceUpdateOldCoeff
      : [update.priceUpdateNewCoeff | (_, update) <- NE.toList changes]

priceDirection :: Double -> Double -> Maybe PriceDirection
priceDirection oldCoeff newCoeff
  | newCoeff > oldCoeff = Just PriceUp
  | newCoeff < oldCoeff = Just PriceDown
  | otherwise = Nothing

compressedDirections :: [PriceMove] -> [PriceDirection]
compressedDirections [] = []
compressedDirections (move : moves) =
  reverse (foldl' step [move.moveDirection] moves)
 where
  step directions@(direction : _) nextMove
    | nextMove.moveDirection == direction = directions
    | otherwise = nextMove.moveDirection : directions
  step [] nextMove = [nextMove.moveDirection]

segmentEndpoints :: [PriceMove] -> [Double]
segmentEndpoints [] = []
segmentEndpoints (move : moves) =
  go move.moveDirection move.moveNewCoeff [move.moveOldCoeff] moves
 where
  go _ endCoeff points [] =
    reverse (endCoeff : points)
  go direction _ points (nextMove : rest)
    | nextMove.moveDirection == direction =
        go direction nextMove.moveNewCoeff points rest
    | otherwise =
        go nextMove.moveDirection nextMove.moveNewCoeff (nextMove.moveOldCoeff : points) rest

adjacentPairs :: [a] -> [(a, a)]
adjacentPairs xs =
  zip xs (drop 1 xs)

triples :: [a] -> [(a, a, a)]
triples (a : b : c : rest) =
  (a, b, c) : triples (b : c : rest)
triples _ = []

tripleAmplitude :: (Double, Double, Double) -> Double
tripleAmplitude (a, b, c) =
  maximum [a, b, c] - minimum [a, b, c]

logTravel :: [Double] -> Double
logTravel endpoints =
  sum [logDistance a b | (a, b) <- adjacentPairs endpoints]

netLogMovement :: [Double] -> Double
netLogMovement [] = 0
netLogMovement (first : rest) =
  logDistance first (lastEndpoint first rest)

lastEndpoint :: a -> [a] -> a
lastEndpoint current [] = current
lastEndpoint _ (next : rest) = lastEndpoint next rest

logDistance :: Double -> Double -> Double
logDistance a b
  | a <= 0 || b <= 0 = 0
  | otherwise = abs (log b - log a)

priceShockThreshold :: Double
priceShockThreshold = 0.10
