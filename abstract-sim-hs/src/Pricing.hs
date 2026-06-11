module Pricing (
  Prices (..),
  PriceUpdate (..),
  initialPrices,
  quotedFee,
  quotedFeeFor,
  realisedFee,
  updatePrices,
  worstCaseNextPrices,
)
where

import Block (BlockSummary (..), EndorserBlockSummary (..), RankingBlock (..), RankingBlockSummary (..))
import Data.Foldable qualified as Foldable
import Data.Maybe (mapMaybe)
import Data.Sequence (Seq)
import Design (ControllerConfig (..), ControllerSignal (..), Design (..), Eip1559Controller (..), FeeSemantics (..))
import Transaction (Lane (..), Script (..), Tx (..), TxBody (..))
import Types (Lovelace (..))

data Prices = Prices
  { standardCoeff :: Double
  , priorityCoeff :: Double
  }
  deriving stock (Eq, Show)

data PriceUpdate = PriceUpdate
  { priceUpdateLane :: Lane
  , priceUpdateOldCoeff :: Double
  , priceUpdateNewCoeff :: Double
  , priceUpdateUtilisation :: Double
  }
  deriving stock (Eq, Show)

initialPrices :: Design -> Prices
initialPrices design =
  applyPriceFloors
    controllers
    Prices
      { standardCoeff = maybe 1.0 controllerInitialCoefficient controllers.standardController
      , priorityCoeff = maybe 1.0 controllerInitialCoefficient controllers.priorityController
      }
 where
  controllers = design.designControllers

quotedFee :: Prices -> Tx -> Lovelace
quotedFee prices tx =
  quotedFeeFor prices tx.txLane tx.txBody._txSize tx.txBody._txScript

{- | @tierCoeff * minfee pp utxo tx@: the lane coefficient multiplies the
entire Cardano min fee — constant, byte, ex-unit, and reference-script terms —
not just the size term.
-}
quotedFeeFor :: Prices -> Lane -> Int -> Script -> Lovelace
quotedFeeFor prices lane txBytes script =
  Lovelace (ceiling (laneCoeff prices lane * minFee txBytes script))

{- | Conway mainnet min fee. '_scriptSize' is reference-script bytes: priced
per byte, but the script lives in the UTxO set, so it contributes to no tx or
block byte capacity. '_scriptExUnits' is the memory-equivalent scalar (see the
note in "Config"), so it is priced at the memory price alone.
-}
minFee :: Int -> Script -> Double
minFee txBytes script =
  fromInteger minFeeB
    + fromInteger minFeeA * fromIntegral txBytes
    + exUnitsMemPrice * fromIntegral script._scriptExUnits
    + fromInteger refScriptCostPerByte * fromIntegral script._scriptSize

{- | The fee actually charged when a tx reaches the chain. Under 'Eip1559' the
node keeps the quote at inclusion and refunds the rest of the posted max fee;
'FixedFee' and 'HonourSubmissionQuoteFor' charge the posted fee in full. The
"priority pays the standard price in EBs" variant would switch the quoted lane
on the inclusion point here.
-}
realisedFee :: FeeSemantics -> Prices -> Tx -> Lovelace
realisedFee semantics prices tx =
  case semantics of
    FixedFee -> postedFee
    HonourSubmissionQuoteFor _ -> postedFee
    Eip1559 -> min postedFee (quotedFee prices tx)
 where
  postedFee = tx.txBody._txFee

laneCoeff :: Prices -> Lane -> Double
laneCoeff prices Standard = prices.standardCoeff
laneCoeff prices Priority = prices.priorityCoeff

minFeeA :: Integer
minFeeA = 44

minFeeB :: Integer
minFeeB = 155_381

-- | Mainnet @executionUnitPrices.priceMemory@.
exUnitsMemPrice :: Double
exUnitsMemPrice = 0.0577

{- | Mainnet @minFeeRefScriptCostPerByte@ base rate; the 1.2×-per-25KiB tier
escalation is dropped as an abstraction.
-}
refScriptCostPerByte :: Integer
refScriptCostPerByte = 15

updatePrices :: Design -> Seq BlockSummary -> Prices -> (Prices, [PriceUpdate])
updatePrices design recentBlocks prices =
  (finalPrices, updates)
 where
  controllers = design.designControllers
  currentPrices =
    applyPriceFloors controllers prices
  standardResult =
    updateLanePrice design Standard recentBlocks currentPrices.standardCoeff
      <$> controllers.standardController
  priorityResult =
    updateLanePrice design Priority recentBlocks currentPrices.priorityCoeff
      <$> controllers.priorityController
  pricesBeforeFloor =
    Prices
      { standardCoeff = maybe currentPrices.standardCoeff priceUpdateNewCoeff standardResult
      , priorityCoeff = maybe currentPrices.priorityCoeff priceUpdateNewCoeff priorityResult
      }
  finalPrices =
    applyPriceFloors controllers pricesBeforeFloor
  updates =
    fmap withFinalFloor (maybe [] pure standardResult <> maybe [] pure priorityResult)
  withFinalFloor update =
    update{priceUpdateNewCoeff = laneCoeff finalPrices update.priceUpdateLane}

{- | Upper bound on prices after the next controller update: one EIP-1559 step
raises a lane's coefficient by at most @1 + 1\/maxChangeDenominator@, and the
floors preserve that bound given already-floored inputs (the absolute floor is
constant; the multiplier floor tracks the standard lane, which is itself
bounded by its own step). Lanes without a controller never move.
-}
worstCaseNextPrices :: ControllerConfig -> Prices -> Prices
worstCaseNextPrices controllers prices =
  Prices
    { standardCoeff = scale controllers.standardController prices.standardCoeff
    , priorityCoeff = scale controllers.priorityController prices.priorityCoeff
    }
 where
  scale Nothing coeff = coeff
  scale (Just controller) coeff =
    coeff * (1 + 1 / fromIntegral (max 1 controller.controllerMaxChangeDenominator))

applyPriceFloors :: ControllerConfig -> Prices -> Prices
applyPriceFloors controllers =
  applyMultiplierFloor controllers . applyAbsoluteFloor controllers

applyAbsoluteFloor :: ControllerConfig -> Prices -> Prices
applyAbsoluteFloor controllers prices =
  prices
    { standardCoeff = max floorCoeff prices.standardCoeff
    , priorityCoeff = max floorCoeff prices.priorityCoeff
    }
 where
  floorCoeff = max 0 controllers.absoluteCoeffFloor

applyMultiplierFloor :: ControllerConfig -> Prices -> Prices
applyMultiplierFloor controllers prices =
  case (controllers.priorityController, controllers.multiplierFloor) of
    (Just _, Just floorMultiplier) ->
      prices
        { priorityCoeff =
            max
              prices.priorityCoeff
              (prices.standardCoeff * floorMultiplier)
        }
    _ -> prices

updateLanePrice :: Design -> Lane -> Seq BlockSummary -> Double -> Eip1559Controller -> PriceUpdate
updateLanePrice design lane recentBlocks oldCoeff controller =
  PriceUpdate
    { priceUpdateLane = lane
    , priceUpdateOldCoeff = oldCoeff
    , priceUpdateNewCoeff = applyEip1559Update controller oldCoeff utilisationValue
    , priceUpdateUtilisation = utilisationValue
    }
 where
  utilisationValue = controllerUtilisation design lane recentBlocks controller

controllerUtilisation :: Design -> Lane -> Seq BlockSummary -> Eip1559Controller -> Double
controllerUtilisation design lane recentBlocks controller =
  case controller.controllerSignal of
    CapacityWeightedWindow windowSize ->
      capacityWeightedWindowUtilisation lane windowSize recentBlocks
    PriorityReservationUtil ->
      priorityReservationUtilisation design recentBlocks

applyEip1559Update :: Eip1559Controller -> Double -> Double -> Double
applyEip1559Update controller oldCoeff utilisationValue =
  oldCoeff * max 0 (1 + adjustment)
 where
  target = max 0.000_001 controller.controllerTargetUtilisation
  denominator = fromIntegral (max 1 controller.controllerMaxChangeDenominator)
  boundedUtilisation = clamp 0 1 utilisationValue
  adjustment = ((boundedUtilisation - target) / target) / denominator

capacityWeightedWindowUtilisation :: Lane -> Int -> Seq BlockSummary -> Double
capacityWeightedWindowUtilisation lane windowSize recentBlocks =
  utilisationRatio (fmap laneUsed summaries) (fmap summaryCapacity summaries)
 where
  summaries = takeLast (max 1 windowSize) recentBlocks
  laneUsed (RankingBlockProduced summary) =
    rankingLaneBytes lane summary
  laneUsed (EndorserBlockAnnounced summary) =
    endorserLaneBytes lane summary
  laneUsed (EndorserBlockCertified _) =
    0

priorityReservationUtilisation :: Design -> Seq BlockSummary -> Double
priorityReservationUtilisation _ recentBlocks =
  mean (takeLast 1 (mapMaybe prioritySignalSample (Foldable.toList recentBlocks)))
 where
  prioritySignalSample (RankingBlockProduced summary) =
    rankingBlockPrioritySignal summary
  prioritySignalSample (EndorserBlockAnnounced _) =
    Nothing
  prioritySignalSample (EndorserBlockCertified summary) =
    endorserBlockPrioritySignal summary

rankingBlockPrioritySignal :: RankingBlockSummary -> Maybe Double
rankingBlockPrioritySignal summary =
  case summary.rankingBlock of
    PraosBlock{} ->
      Just
        ( priorityResourceFill
            summary.rankingBlockPriorityBytes
            summary.rankingBlockPriorityExUnits
            summary.rankingBlockPriorityCapacityBytes
            summary.rankingBlockPriorityCapacityExUnits
        )
    CertifyingBlock{} ->
      Nothing

endorserBlockPrioritySignal :: EndorserBlockSummary -> Maybe Double
endorserBlockPrioritySignal summary =
  Just
    ( priorityResourceFill
        summary.endorserBlockPriorityBytes
        summary.endorserBlockPriorityExUnits
        summary.endorserBlockPrioritySignalCapacityBytes
        summary.endorserBlockPrioritySignalCapacityExUnits
    )

priorityResourceFill :: Int -> Int -> Int -> Int -> Double
priorityResourceFill usedBytes usedExUnits capacityBytes capacityExUnits =
  clamp 0 1 (max bytesFill exUnitsFill)
 where
  bytesFill = resourceRatio usedBytes capacityBytes
  exUnitsFill = resourceRatio usedExUnits capacityExUnits

resourceRatio :: Int -> Int -> Double
resourceRatio _ capacity | capacity <= 0 = 0
resourceRatio used capacity =
  fromIntegral used / fromIntegral capacity

rankingLaneBytes :: Lane -> RankingBlockSummary -> Int
rankingLaneBytes Standard = rankingBlockStandardBytes
rankingLaneBytes Priority = rankingBlockPriorityBytes

endorserLaneBytes :: Lane -> EndorserBlockSummary -> Int
endorserLaneBytes Standard = endorserBlockStandardBytes
endorserLaneBytes Priority = endorserBlockPriorityBytes

summaryCapacity :: BlockSummary -> Int
summaryCapacity (RankingBlockProduced summary) =
  rankingBlockCapacityBytes summary
summaryCapacity (EndorserBlockAnnounced summary) =
  endorserBlockCapacityBytes summary
summaryCapacity (EndorserBlockCertified _) =
  0

utilisationRatio :: [Int] -> [Int] -> Double
utilisationRatio used capacity
  | totalCapacity <= 0 = 0
  | otherwise = fromIntegral totalUsed / fromIntegral totalCapacity
 where
  totalUsed = sum used
  totalCapacity = sum capacity

takeLast :: (Foldable f) => Int -> f a -> [a]
takeLast n xs =
  drop (length ys - n) ys
 where
  ys = Foldable.toList xs

mean :: [Double] -> Double
mean [] = 0
mean xs = sum xs / fromIntegral (length xs)

clamp :: (Ord a) => a -> a -> a -> a
clamp lo hi =
  min hi . max lo
