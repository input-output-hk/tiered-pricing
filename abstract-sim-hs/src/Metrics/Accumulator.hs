module Metrics.Accumulator (
  MetricsAcc (..),
  AccPriceChange (..),
  emptyMetricsAcc,
  recordMetricsEvents,
  observedSlots,
  observedUrgencies,
  allLanes,
  observedUrgencyLanes,
  submittedTxsWhere,
  includedTxsWhere,
  evictedTxsWhere,
  latenciesWhere,
  includedLatency,
  blockLatenciesWhere,
  includedBlockLatency,
  matchesUrgencyLane,
  sumLovelace,
  maximumOrZero,
  mean,
  ratio,
  jainIndex,
) where

import Actor (ActorId)
import Block (BlockSummary (..))
import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Data.Maybe (mapMaybe)
import Data.Set qualified as Set
import Event (SimEvent (..))
import Transaction (Lane (..), Tx (..), TxId)
import Types (Duration, Lovelace (..), SlotNo (..), Urgency, diffSlots)

data MetricsAcc = MetricsAcc
  { accSubmitted :: Map TxId Tx
  , accSubmittedAt :: Map TxId SlotNo
  , accSubmittedByActor :: Map TxId ActorId
  , accAdmitted :: Set.Set TxId
  , accIncludedAt :: Map TxId SlotNo
  , accSubmittedAtBlock :: Map TxId Int
  , accIncludedAtBlock :: Map TxId Int
  , accRankingBlockCount :: Int
  , accEvicted :: Set.Set TxId
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

emptyMetricsAcc :: MetricsAcc
emptyMetricsAcc =
  MetricsAcc
    { accSubmitted = mempty
    , accSubmittedAt = mempty
    , accSubmittedByActor = mempty
    , accAdmitted = mempty
    , accIncludedAt = mempty
    , accSubmittedAtBlock = mempty
    , accIncludedAtBlock = mempty
    , accRankingBlockCount = 0
    , accEvicted = mempty
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
      }
  TxAdmitted _ txId ->
    acc{accAdmitted = Set.insert txId acc.accAdmitted}
  TxRejected{} ->
    acc
  TxIncluded slot txId _ ->
    acc
      { accIncludedAt = Map.insert txId slot acc.accIncludedAt
      , accIncludedAtBlock = Map.insert txId acc.accRankingBlockCount acc.accIncludedAtBlock
      }
  TxEvicted _ txId _ ->
    acc{accEvicted = Set.insert txId acc.accEvicted}
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

submittedTxsWhere :: MetricsAcc -> (Tx -> Bool) -> [Tx]
submittedTxsWhere acc predicate =
  filter predicate (Map.elems acc.accSubmitted)

includedTxsWhere :: MetricsAcc -> (Tx -> Bool) -> [Tx]
includedTxsWhere acc predicate =
  fmap snd (filter isIncludedMatch (Map.toList acc.accSubmitted))
 where
  isIncludedMatch (txId, tx) =
    predicate tx && Map.member txId acc.accIncludedAt

evictedTxsWhere :: MetricsAcc -> (Tx -> Bool) -> [Tx]
evictedTxsWhere acc predicate =
  fmap snd (filter isEvictedMatch (Map.toList acc.accSubmitted))
 where
  isEvictedMatch (txId, tx) =
    predicate tx && Set.member txId acc.accEvicted

latenciesWhere :: MetricsAcc -> (Tx -> Bool) -> [Duration]
latenciesWhere acc predicate =
  mapMaybe (includedLatency acc . fst) (filter (predicate . snd) (Map.toList acc.accSubmitted))

includedLatency :: MetricsAcc -> TxId -> Maybe Duration
includedLatency acc txId = do
  submittedAt <- Map.lookup txId acc.accSubmittedAt
  includedAt <- Map.lookup txId acc.accIncludedAt
  pure (diffSlots includedAt submittedAt)

blockLatenciesWhere :: MetricsAcc -> (Tx -> Bool) -> [Int]
blockLatenciesWhere acc predicate =
  mapMaybe (includedBlockLatency acc . fst) (filter (predicate . snd) (Map.toList acc.accSubmitted))

includedBlockLatency :: MetricsAcc -> TxId -> Maybe Int
includedBlockLatency acc txId = do
  submittedAt <- Map.lookup txId acc.accSubmittedAtBlock
  includedAt <- Map.lookup txId acc.accIncludedAtBlock
  pure (max 0 (includedAt - submittedAt))

matchesUrgencyLane :: Urgency -> Lane -> Tx -> Bool
matchesUrgencyLane urgency lane tx =
  txUrgency tx == urgency && txLane tx == lane

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
  TxIncluded slot _ _ -> slot
  TxEvicted slot _ _ -> slot
  BlockProduced slot _ -> slot
  PriceUpdated slot _ _ _ _ -> slot

observedUrgencies :: MetricsAcc -> [Urgency]
observedUrgencies acc =
  Set.toList (Set.fromList (fmap txUrgency (Map.elems acc.accSubmitted)))

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

jainIndex :: [Int] -> Double
jainIndex [] = 1
jainIndex xs
  | sumCounts <= 0 = 1
  | otherwise = (sumCounts * sumCounts) / (fromIntegral (length xs) * sumSquares)
 where
  counts = fmap fromIntegral xs
  sumCounts = sum counts
  sumSquares = sum (fmap (^ (2 :: Int)) counts)
