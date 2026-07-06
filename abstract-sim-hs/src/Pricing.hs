module Pricing (
  Prices (..),
  PriceUpdate (..),
  ControllerInput (..),
  initialPrices,
  quotedFee,
  quotedFeeFor,
  realisedFee,
  admissionRequiredFee,
  coversProducerHeadroom,
  feeStillValid,
  updatePrices,
  priceStepsAhead,
  worstCaseNextPrices,
  signalWindow,
  retentionWindow,
)
where

import Block (BlockSummary (..), BlockUsage (..), InclusionPoint (..))
import Data.Foldable qualified as Foldable
import Data.Maybe (catMaybes, fromMaybe, mapMaybe)
import Data.Sequence (Seq)
import Design (ControllerConfig (..), ControllerSignal (..), Design (..), Eip1559Controller (..), FeeSemantics (..), PriorityPremiumScope (..))
import Resource (Bytes (..), ExUnits (..), Resources (..))
import Transaction (Script (..), Tx (..), TxBody (..))
import Types (Lane (..), Lovelace (..), PerLane (..), SlotNo, atLane, diffSlots, lanes)

newtype Prices = Prices {laneCoeffs :: PerLane Double}
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
    (Prices (fmap (maybe 1.0 (.controllerInitialCoefficient)) controllers.laneControllers))
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
quoted lane is the posted lane, except under 'PremiumRbOnly' where EB
inclusion is quoted at the standard lane ('Design.PriorityPremiumScope').
-}
realisedFee :: PriorityPremiumScope -> FeeSemantics -> Prices -> InclusionPoint -> Tx -> Lovelace
realisedFee scope semantics prices inclusionPoint tx =
  realisedFeeAtLane chargedLane semantics prices tx
 where
  chargedLane =
    case (scope, inclusionPoint) of
      (PremiumRbOnly, IncludedInEb _) -> Standard
      _ -> tx.txLane

realisedFeeAtLane :: Lane -> FeeSemantics -> Prices -> Tx -> Lovelace
realisedFeeAtLane chargedLane semantics prices tx =
  case semantics of
    FixedFee -> postedFee
    HonourSubmissionQuoteFor _ -> postedFee
    Eip1559 ->
      min postedFee (quotedFeeFor prices chargedLane tx.txBody._txSize tx.txBody._txScript)
 where
  postedFee = tx.txBody._txFee

laneCoeff :: Prices -> Lane -> Double
laneCoeff prices lane = atLane lane prices.laneCoeffs

{- | The fee a tx must post to enter the mempool: the quote after
@headroomUpdates@ worst-case controller steps.

This is the admission-side complement of the producer headroom
('coversProducerHeadroom'), and with a horizon of 1 it is deliberately
semi-redundant with it: nothing enters the mempool that a producer would
refuse to put in an EB at today's prices, so unserviceable txs are rejected
at the door (visibly, resubmittably) instead of sitting against the mempool
cap forever. The producer check is still required because prices may keep
rising while an admitted tx waits its turn — admission keeps the mempool
serviceable, the producer check keeps certified EBs valid. 'FixedFee' never
re-prices, so it needs no headroom.
-}
admissionRequiredFee :: ControllerConfig -> Int -> FeeSemantics -> Prices -> Tx -> Lovelace
admissionRequiredFee controllers headroomUpdates semantics prices tx =
  case semantics of
    FixedFee -> quotedFee prices tx
    _ -> quotedFee (priceStepsAhead controllers headroomUpdates prices) tx

{- | Producer headroom: does this tx stay valid through the single price
update that can fire before the certification check? A prudent producer
fills EBs only with txs satisfying this bound, so its EBs cannot fail fee
validation. Admission with a horizon >= 1 guarantees it at entry
('admissionRequiredFee'); the producer re-checks against current prices
because they may have risen while the tx waited. For
'HonourSubmissionQuoteFor' the honour window may expire before the
certification check, so only the price bound guarantees safety; 'FixedFee'
never re-prices, so everything is safe.
-}
coversProducerHeadroom :: ControllerConfig -> FeeSemantics -> Prices -> Tx -> Bool
coversProducerHeadroom controllers semantics prices tx =
  case semantics of
    FixedFee -> True
    _ -> tx.txBody._txFee >= quotedFee (priceStepsAhead controllers 1 prices) tx

{- | Staleness per the design's fee semantics: under 'Eip1559' the posted max
fee must still cover the current quote; 'HonourSubmissionQuoteFor' defers
that check until the honour window after submission has elapsed; 'FixedFee'
never goes stale.
-}
feeStillValid :: FeeSemantics -> SlotNo -> Prices -> Tx -> Bool
feeStillValid semantics slot prices tx =
  case semantics of
    FixedFee -> True
    Eip1559 -> coversCurrentQuote
    HonourSubmissionQuoteFor honourFor ->
      diffSlots slot tx.txSubmitted <= honourFor || coversCurrentQuote
 where
  coversCurrentQuote =
    tx.txBody._txFee >= quotedFee prices tx

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

{- | What a controller update reads. Windowed signals consume the retained
history; windowless per-block signals consume the block production that
fired this update, so their controller event can never be trimmed away by
a retention window sized for another controller ('retentionWindow').
-}
data ControllerInput = ControllerInput
  { recentBlocks :: Seq BlockSummary
  , currentProduction :: Seq BlockSummary
  }

updatePrices :: ControllerConfig -> ControllerInput -> Prices -> (Prices, [PriceUpdate])
updatePrices controllers input prices =
  (finalPrices, updates)
 where
  currentPrices =
    applyPriceFloors controllers prices
  laneResults :: PerLane (Maybe PriceUpdate)
  laneResults =
    (\lane coeff -> fmap (updateLanePrice lane input coeff))
      <$> lanes
      <*> currentPrices.laneCoeffs
      <*> controllers.laneControllers
  pricesBeforeFloor =
    Prices (fromMaybe <$> currentPrices.laneCoeffs <*> fmap (fmap (.priceUpdateNewCoeff)) laneResults)
  finalPrices =
    applyPriceFloors controllers pricesBeforeFloor
  updates =
    withFinalFloor <$> catMaybes (Foldable.toList laneResults)
  withFinalFloor update =
    update{priceUpdateNewCoeff = laneCoeff finalPrices update.priceUpdateLane}

-- | Upper bound on prices after @steps@ controller updates.
priceStepsAhead :: ControllerConfig -> Int -> Prices -> Prices
priceStepsAhead controllers steps prices =
  iterate (worstCaseNextPrices controllers) prices !! max 0 steps

{- | Upper bound on prices after the next controller update: one EIP-1559 step
raises a lane's coefficient by at most @1 + 1\/maxChangeDenominator@, and the
floors preserve that bound given already-floored inputs (the absolute floor is
constant; the multiplier floor tracks the standard lane, which is itself
bounded by its own step). Lanes without a controller never move.
-}
worstCaseNextPrices :: ControllerConfig -> Prices -> Prices
worstCaseNextPrices controllers prices =
  Prices (scale <$> controllers.laneControllers <*> prices.laneCoeffs)
 where
  scale Nothing coeff = coeff
  scale (Just controller) coeff =
    coeff * (1 + 1 / fromIntegral (max 1 controller.controllerMaxChangeDenominator))

applyPriceFloors :: ControllerConfig -> Prices -> Prices
applyPriceFloors controllers =
  applyMultiplierFloor controllers . applyAbsoluteFloor controllers

applyAbsoluteFloor :: ControllerConfig -> Prices -> Prices
applyAbsoluteFloor controllers (Prices coeffs) =
  Prices (fmap (max floorCoeff) coeffs)
 where
  floorCoeff = max 0 controllers.absoluteCoeffFloor

applyMultiplierFloor :: ControllerConfig -> Prices -> Prices
applyMultiplierFloor controllers prices =
  case (controllers.laneControllers.perPriority, controllers.multiplierFloor) of
    (Just _, Just floorMultiplier) ->
      Prices
        prices.laneCoeffs
          { perPriority =
              max
                prices.laneCoeffs.perPriority
                (prices.laneCoeffs.perStandard * floorMultiplier)
          }
    _ -> prices

updateLanePrice :: Lane -> ControllerInput -> Double -> Eip1559Controller -> PriceUpdate
updateLanePrice lane input oldCoeff controller =
  PriceUpdate
    { priceUpdateLane = lane
    , priceUpdateOldCoeff = oldCoeff
    , priceUpdateNewCoeff = applyEip1559Update controller oldCoeff utilisationValue
    , priceUpdateUtilisation = utilisationValue
    }
 where
  utilisationValue = controllerUtilisation lane input controller

controllerUtilisation :: Lane -> ControllerInput -> Eip1559Controller -> Double
controllerUtilisation lane input controller =
  case controller.controllerSignal of
    signal@CapacityWeightedWindow{} ->
      capacityWeightedWindowUtilisation lane (signalWindow signal) input.recentBlocks
    PriorityReservationUtil ->
      priorityReservationUtilisation input.currentProduction
    PriorityReservationWindow windowSize ->
      priorityReservationWindowUtilisation windowSize input.recentBlocks

{- | How much retained history a controller signal reads. Per-block signals
read none of it — they consume 'currentProduction' — so retention cannot
starve them however the other controller is configured.
-}
signalWindow :: ControllerSignal -> Int
signalWindow = \case
  CapacityWeightedWindow windowSize -> max 1 windowSize
  PriorityReservationUtil -> 0
  PriorityReservationWindow windowSize -> 3 * max 1 windowSize

{- | How far back the price controllers can read the recent-block history.
Derived from 'signalWindow' so the engine's retention and the signals'
lookback cannot drift apart: the engine keeps exactly this many summaries.
-}
retentionWindow :: ControllerConfig -> Int
retentionWindow controllers =
  maximum
    ( 1
        : [ signalWindow controller.controllerSignal
          | Just controller <- Foldable.toList controllers.laneControllers
          ]
    )

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
  max
    (utilisationRatio (fmap laneUsedBytes summaries) (fmap summaryCapacityBytes summaries))
    (utilisationRatio (fmap laneUsedExUnits summaries) (fmap summaryCapacityExUnits summaries))
 where
  summaries = takeLast windowSize recentBlocks
  laneUsedBytes summary =
    (atLane lane (summaryLaneUsage summary)).resBytes.unBytes
  laneUsedExUnits summary =
    (atLane lane (summaryLaneUsage summary)).resExUnits.unExUnits
  summaryCapacityBytes summary =
    (summaryCapacity summary).resBytes.unBytes
  summaryCapacityExUnits summary =
    (summaryCapacity summary).resExUnits.unExUnits
  summaryLaneUsage = \case
    RbPraos _ usage -> usage.usageLanes
    RbCertifying _ -> pure mempty
    EbAnnounced _ _ -> pure mempty
    EbCertified _ usage -> usage.usageLanes
  summaryCapacity = \case
    RbPraos _ usage -> usage.usageCapacity
    RbCertifying _ -> mempty
    EbAnnounced _ _ -> mempty
    EbCertified _ usage -> usage.usageCapacity

{- | The design doc's @priorityUtil(b)@, per priced block: the controller
event in this update's block production — a Praos RB's fill of the
reservation, or the certified EB's partition fill. Announcements carry no
sample ("endorsement-only RBs do not fire a controller event — their
certified EB does, separately, when applied"), and every block production
contains exactly one event, so the mean is that single sample.
-}
priorityReservationUtilisation :: Seq BlockSummary -> Double
priorityReservationUtilisation production =
  mean (fmap priorityFill (mapMaybe prioritySignalSample (Foldable.toList production)))

priorityReservationWindowUtilisation :: Int -> Seq BlockSummary -> Double
priorityReservationWindowUtilisation windowSize recentBlocks =
  aggregatePriorityFill samples
 where
  samples =
    takeLast (max 1 windowSize) (mapMaybe prioritySignalSample (Foldable.toList recentBlocks))

prioritySignalSample :: BlockSummary -> Maybe BlockUsage
prioritySignalSample = \case
  RbPraos _ usage -> Just usage
  RbCertifying _ -> Nothing
  EbAnnounced _ _ -> Nothing
  EbCertified _ usage -> Just usage

{- | How full the priority lane ran against its signal capacity, on the
binding dimension.
-}
priorityFill :: BlockUsage -> Double
priorityFill usage =
  clamp 0 1 (max bytesFill exUnitsFill)
 where
  priorityUsed = usage.usageLanes.perPriority
  bytesFill =
    resourceRatio priorityUsed.resBytes.unBytes usage.usageSignalCapacity.resBytes.unBytes
  exUnitsFill =
    resourceRatio priorityUsed.resExUnits.unExUnits usage.usageSignalCapacity.resExUnits.unExUnits

aggregatePriorityFill :: [BlockUsage] -> Double
aggregatePriorityFill usages =
  clamp 0 1 (max bytesFill exUnitsFill)
 where
  bytesFill =
    resourceRatio
      (sum (fmap cappedPriorityBytes usages))
      (sum (fmap (\usage -> usage.usageSignalCapacity.resBytes.unBytes) usages))
  exUnitsFill =
    resourceRatio
      (sum (fmap cappedPriorityExUnits usages))
      (sum (fmap (\usage -> usage.usageSignalCapacity.resExUnits.unExUnits) usages))
  cappedPriorityBytes usage =
    min
      usage.usageLanes.perPriority.resBytes.unBytes
      usage.usageSignalCapacity.resBytes.unBytes
  cappedPriorityExUnits usage =
    min
      usage.usageLanes.perPriority.resExUnits.unExUnits
      usage.usageSignalCapacity.resExUnits.unExUnits

resourceRatio :: Int -> Int -> Double
resourceRatio _ capacity | capacity <= 0 = 0
resourceRatio used capacity =
  fromIntegral used / fromIntegral capacity

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
