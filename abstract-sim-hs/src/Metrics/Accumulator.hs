module Metrics.Accumulator (
  MetricsAcc (..),
  DemandUnit (..),
  UnitOutcome (..),
  emptyMetricsAcc,
  recordMetricsEvents,
  observedUrgencies,
  allLanes,
  observedUrgencyLanes,
  includedTxs,
  unitsWhere,
  unitLane,
  unitServed,
  matchesUnitUrgencyLane,
  sumLovelace,
) where

import Block (BlockSummary (..))
import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Data.Set qualified as Set
import Event (SimEvent (..))
import Pricing (PriceUpdate)
import Transaction (Lane (..), Tx (..), TxBody (..), TxId)
import Types (Lovelace (..), SlotNo (..), Urgency)

data MetricsAcc = MetricsAcc
  { accSubmitted :: Map TxId Tx
  , accIncludedAt :: Map TxId SlotNo
  , accRealisedFee :: Map TxId Lovelace
  , accRankingBlockCount :: Int
  , accUnits :: Map Int DemandUnit
  -- ^ demand units keyed by origin tx number; the headline metrics read
  -- these, the @Map TxId@ structures above are per-attempt
  , accBlocks :: [BlockSummary]
  , accPriceChanges :: [(SlotNo, PriceUpdate)]
  -- ^ price-controller updates in reverse event order
  }

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
    , accIncludedAt = mempty
    , accRealisedFee = mempty
    , accRankingBlockCount = 0
    , accUnits = mempty
    , accBlocks = mempty
    , accPriceChanges = mempty
    }

recordMetricsEvents :: Foldable f => MetricsAcc -> f SimEvent -> MetricsAcc
recordMetricsEvents =
  foldl' stepMetrics

stepMetrics :: MetricsAcc -> SimEvent -> MetricsAcc
stepMetrics acc = \case
  TxSubmitted _ _ tx ->
    acc
      { accSubmitted = Map.insert tx.txId tx acc.accSubmitted
      , accUnits =
          Map.insertWith mergeUnit tx.txOriginNumber (freshUnit acc tx) acc.accUnits
      }
  -- Admissions, rejections, and evictions currently feed no metric: a demand
  -- unit's fate is read off its submissions, inclusions, and abandonments.
  TxAdmitted{} ->
    acc
  TxRejected{} ->
    acc
  TxEvicted{} ->
    acc
  TxIncluded slot txId _ realised ->
    acc
      { accIncludedAt = Map.insert txId slot acc.accIncludedAt
      , accRealisedFee = Map.insert txId realised acc.accRealisedFee
      , accUnits =
          case Map.lookup txId acc.accSubmitted of
            Nothing -> acc.accUnits
            Just tx ->
              Map.adjust
                (serveUnit slot acc.accRankingBlockCount tx realised)
                tx.txOriginNumber
                acc.accUnits
      }
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
  PriceUpdated slot update ->
    acc{accPriceChanges = (slot, update) : acc.accPriceChanges}

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

includedTxs :: MetricsAcc -> [Tx]
includedTxs acc =
  [tx | (txId, tx) <- Map.toList acc.accSubmitted, Map.member txId acc.accIncludedAt]

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

isRankingBlock :: BlockSummary -> Bool
isRankingBlock RankingBlockProduced{} = True
isRankingBlock _ = False

