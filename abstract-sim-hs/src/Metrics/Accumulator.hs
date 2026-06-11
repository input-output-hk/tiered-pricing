module Metrics.Accumulator (
  MetricsAcc (..),
  AccPriceChange (..),
  DemandUnit (..),
  UnitOutcome (..),
  emptyMetricsAcc,
  recordMetricsEvents,
  observedSlots,
  observedUrgencies,
  allLanes,
  observedUrgencyLanes,
  includedTxsWhere,
  unitsWhere,
  unitLane,
  unitServed,
  matchesUnitUrgencyLane,
  sumLovelace,
  maximumOrZero,
  mean,
  ratio,
) where

import Actor (ActorId)
import Block (BlockSummary (..))
import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Data.Set qualified as Set
import Event (SimEvent (..))
import Transaction (Lane (..), Tx (..), TxBody (..), TxId)
import Types (Lovelace (..), SlotNo (..), Urgency)

data MetricsAcc = MetricsAcc
  { accSubmitted :: Map TxId Tx
  , accSubmittedAt :: Map TxId SlotNo
  , accSubmittedByActor :: Map TxId ActorId
  , accAdmitted :: Set.Set TxId
  , accIncludedAt :: Map TxId SlotNo
  , accRealisedFee :: Map TxId Lovelace
  , accSubmittedAtBlock :: Map TxId Int
  , accIncludedAtBlock :: Map TxId Int
  , accRankingBlockCount :: Int
  , accEvicted :: Set.Set TxId
  , accUnits :: Map Int DemandUnit
  -- ^ demand units keyed by origin tx number; the headline metrics read
  -- these, the @Map TxId@ structures above are per-attempt
  , accBlocks :: [BlockSummary]
  , accPriceJumps :: [Double]
  , accPriceChanges :: [AccPriceChange]
  }

data AccPriceChange = AccPriceChange
  { accPriceChangeSlot :: SlotNo
  , accPriceChangeLane :: Lane
  , accPriceChangeOldCoeff :: Double
  , accPriceChangeNewCoeff :: Double
  , accPriceChangeUtilisation :: Double
  }
  deriving (Eq, Show)

{- | A demand unit's terminal state. No outcome means the unit was still in
flight (mempool-resident or awaiting a retry) when the run ended.
-}
data UnitOutcome
  = -- | slot, ranking-block count, serving lane, realised fee
    UnitIncluded SlotNo Int Lane Lovelace
  | UnitAbandoned SlotNo
  deriving (Eq, Show)

{- | One demand unit, keyed by origin: the user's underlying intent, however
many submission attempts it took. The headline metrics count these once;
per-attempt load lives in "Metrics.Demand".
-}
data DemandUnit = DemandUnit
  { unitValue :: Lovelace
  , unitUrgency :: Urgency
  , unitFirstSubmitted :: SlotNo
  -- ^ first submission — the latency and value-decay anchor
  , unitFirstSubmittedBlock :: Int
  , unitAttempts :: Int
  , unitLastLane :: Lane
  -- ^ lane of the latest attempt seen
  , unitFirstPostedFee :: Lovelace
  , unitServingPostedFee :: Maybe Lovelace
  -- ^ posted fee of the attempt that reached the chain
  , unitOutcome :: Maybe UnitOutcome
  }
  deriving (Eq, Show)

emptyMetricsAcc :: MetricsAcc
emptyMetricsAcc =
  MetricsAcc
    { accSubmitted = mempty
    , accSubmittedAt = mempty
    , accSubmittedByActor = mempty
    , accAdmitted = mempty
    , accIncludedAt = mempty
    , accRealisedFee = mempty
    , accSubmittedAtBlock = mempty
    , accIncludedAtBlock = mempty
    , accRankingBlockCount = 0
    , accEvicted = mempty
    , accUnits = mempty
    , accBlocks = mempty
    , accPriceJumps = mempty
    , accPriceChanges = mempty
    }

recordMetricsEvents :: Foldable f => MetricsAcc -> f SimEvent -> MetricsAcc
recordMetricsEvents =
  foldl' stepMetrics

stepMetrics :: MetricsAcc -> SimEvent -> MetricsAcc
stepMetrics acc = \case
  TxSubmitted slot actorId tx ->
    acc
      { accSubmitted = Map.insert tx.txId tx acc.accSubmitted
      , accSubmittedAt = Map.insert tx.txId slot acc.accSubmittedAt
      , accSubmittedByActor = Map.insert tx.txId actorId acc.accSubmittedByActor
      , accSubmittedAtBlock = Map.insert tx.txId acc.accRankingBlockCount acc.accSubmittedAtBlock
      , accUnits =
          Map.insertWith mergeUnit tx.txOriginNumber (freshUnit acc tx) acc.accUnits
      }
  TxAdmitted _ txId ->
    acc{accAdmitted = Set.insert txId acc.accAdmitted}
  TxRejected{} ->
    acc
  TxIncluded slot txId _ realised ->
    acc
      { accIncludedAt = Map.insert txId slot acc.accIncludedAt
      , accRealisedFee = Map.insert txId realised acc.accRealisedFee
      , accIncludedAtBlock = Map.insert txId acc.accRankingBlockCount acc.accIncludedAtBlock
      , accUnits =
          case Map.lookup txId acc.accSubmitted of
            Nothing -> acc.accUnits
            Just tx ->
              Map.adjust
                (serveUnit slot acc.accRankingBlockCount tx realised)
                tx.txOriginNumber
                acc.accUnits
      }
  TxEvicted _ txId _ ->
    acc{accEvicted = Set.insert txId acc.accEvicted}
  TxAbandoned slot origin ->
    acc{accUnits = Map.adjust (abandonUnit slot) origin acc.accUnits}
  BlockProduced _ summary ->
    acc
      { accBlocks = summary : acc.accBlocks
      , accRankingBlockCount =
          if isRankingBlock summary
            then acc.accRankingBlockCount + 1
            else acc.accRankingBlockCount
      }
  PriceUpdated slot lane oldCoeff newCoeff utilisation ->
    acc
      { accPriceJumps = relativeJump oldCoeff newCoeff : acc.accPriceJumps
      , accPriceChanges =
          AccPriceChange
            { accPriceChangeSlot = slot
            , accPriceChangeLane = lane
            , accPriceChangeOldCoeff = oldCoeff
            , accPriceChangeNewCoeff = newCoeff
            , accPriceChangeUtilisation = utilisation
            }
            : acc.accPriceChanges
      }

freshUnit :: MetricsAcc -> Tx -> DemandUnit
freshUnit acc tx =
  DemandUnit
    { unitValue = tx.txValue
    , unitUrgency = tx.txUrgency
    , unitFirstSubmitted = tx.txOriginSubmitted
    , unitFirstSubmittedBlock = acc.accRankingBlockCount
    , unitAttempts = tx.txAttempt
    , unitLastLane = tx.txLane
    , unitFirstPostedFee = tx.txBody._txFee
    , unitServingPostedFee = Nothing
    , unitOutcome = Nothing
    }

{- | Events are chronological, so the first sighting (the first attempt) fixed
the unit's origin facts; later attempts only advance the attempt count and
the lane.
-}
mergeUnit :: DemandUnit -> DemandUnit -> DemandUnit
mergeUnit new old =
  old
    { unitAttempts = max old.unitAttempts new.unitAttempts
    , unitLastLane =
        if new.unitAttempts >= old.unitAttempts
          then new.unitLastLane
          else old.unitLastLane
    }

serveUnit :: SlotNo -> Int -> Tx -> Lovelace -> DemandUnit -> DemandUnit
serveUnit slot blockCount tx realised unit =
  case unit.unitOutcome of
    Just _ -> unit
    Nothing ->
      unit
        { unitOutcome = Just (UnitIncluded slot blockCount tx.txLane realised)
        , unitServingPostedFee = Just tx.txBody._txFee
        }

abandonUnit :: SlotNo -> DemandUnit -> DemandUnit
abandonUnit slot unit =
  case unit.unitOutcome of
    Just _ -> unit
    Nothing -> unit{unitOutcome = Just (UnitAbandoned slot)}

includedTxsWhere :: MetricsAcc -> (Tx -> Bool) -> [Tx]
includedTxsWhere acc predicate =
  fmap snd (filter isIncludedMatch (Map.toList acc.accSubmitted))
 where
  isIncludedMatch (txId, tx) =
    predicate tx && Map.member txId acc.accIncludedAt

unitsWhere :: MetricsAcc -> (DemandUnit -> Bool) -> [DemandUnit]
unitsWhere acc predicate =
  filter predicate (Map.elems acc.accUnits)

{- | Lane attribution for a demand unit: the lane that actually served it when
included, otherwise the last lane it attempted.
-}
unitLane :: DemandUnit -> Lane
unitLane unit =
  case unit.unitOutcome of
    Just (UnitIncluded _ _ lane _) -> lane
    _ -> unit.unitLastLane

unitServed :: DemandUnit -> Bool
unitServed unit =
  case unit.unitOutcome of
    Just UnitIncluded{} -> True
    _ -> False

matchesUnitUrgencyLane :: Urgency -> Lane -> DemandUnit -> Bool
matchesUnitUrgencyLane urgency lane unit =
  unit.unitUrgency == urgency && unitLane unit == lane

observedSlots :: [SimEvent] -> Int
observedSlots events =
  case fmap eventSlot events of
    [] -> 0
    slots -> 1 + maximum (fmap slotToInt slots)

eventSlot :: SimEvent -> SlotNo
eventSlot = \case
  TxSubmitted slot _ _ -> slot
  TxAdmitted slot _ -> slot
  TxRejected slot _ _ -> slot
  TxIncluded slot _ _ _ -> slot
  TxAbandoned slot _ -> slot
  TxEvicted slot _ _ -> slot
  BlockProduced slot _ -> slot
  PriceUpdated slot _ _ _ _ -> slot

observedUrgencies :: MetricsAcc -> [Urgency]
observedUrgencies acc =
  Set.toList (Set.fromList (fmap (.unitUrgency) (Map.elems acc.accUnits)))

allLanes :: [Lane]
allLanes = [Standard, Priority]

observedUrgencyLanes :: MetricsAcc -> [(Urgency, Lane)]
observedUrgencyLanes acc =
  (,) <$> observedUrgencies acc <*> allLanes

sumLovelace :: [Lovelace] -> Lovelace
sumLovelace =
  foldl' addLovelace (Lovelace 0)

addLovelace :: Lovelace -> Lovelace -> Lovelace
addLovelace (Lovelace a) (Lovelace b) =
  Lovelace (a + b)

slotToInt :: SlotNo -> Int
slotToInt (SlotNo n) = n

relativeJump :: Double -> Double -> Double
relativeJump oldCoeff newCoeff
  | oldCoeff <= 0 = 0
  | otherwise = abs (newCoeff - oldCoeff) / oldCoeff

isRankingBlock :: BlockSummary -> Bool
isRankingBlock RankingBlockProduced{} = True
isRankingBlock _ = False

maximumOrZero :: [Double] -> Double
maximumOrZero [] = 0
maximumOrZero xs = maximum xs

mean :: [Double] -> Double
mean [] = 0
mean xs = sum xs / fromIntegral (length xs)

ratio :: Int -> Int -> Double
ratio _ denominator | denominator <= 0 = 0
ratio numerator denominator =
  fromIntegral numerator / fromIntegral denominator
