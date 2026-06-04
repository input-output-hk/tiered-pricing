module Block
  ( EbId (..)
  , EndorserBlock (..)
  , InclusionPoint (..)
  , PendingEb (..)
  , RankingBlock (..)
  , BlockSummary (..)
  , RankingBlockSummary (..)
  , EndorserBlockSummary (..)
  , mkRankingBlockSummary
  , mkEndorserBlockSummary
  , selectedTxBodies
  , laneBytes
  , txBytes
  , selectByBlockCapacity
  , selectPriorityByBlockCapacity
  , txBlockResources
  )
where

import Data.Foldable qualified as Foldable
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Maybe (mapMaybe)
import Data.Sequence (Seq (..), (|>))
import Data.Set (Set)
import Transaction (Lane (..), Script (_scriptExUnits), Tx (..), TxBody (..), TxId)
import Types (SlotNo)

data EbId = EbId Int deriving (Eq, Ord, Show)

data EndorserBlock = EndorserBlock
  { _ebTxs :: Set TxId
  , _ebId :: EbId
  }

data InclusionPoint
  = IncludedInRb
  | IncludedInEb EbId
  deriving stock (Eq, Show)

data PendingEb = PendingEb {pendingEbId :: EbId, pendingEbAnnounced :: SlotNo}

data RankingBlock = CertifyingBlock EbId | PraosBlock [TxId]
  deriving stock (Eq, Show)

data BlockSummary
  = RankingBlockProduced RankingBlockSummary
  | EndorserBlockAnnounced EndorserBlockSummary
  | EndorserBlockCertified EndorserBlockSummary
  deriving stock (Eq, Show)

data RankingBlockSummary = RankingBlockSummary
  { rankingBlock :: RankingBlock
  , rankingBlockCapacityBytes :: Int
  , rankingBlockCapacityExUnits :: Int
  , rankingBlockUsedBytes :: Int
  , rankingBlockUsedExUnits :: Int
  , rankingBlockPriorityBytes :: Int
  , rankingBlockPriorityExUnits :: Int
  , rankingBlockStandardBytes :: Int
  , rankingBlockStandardExUnits :: Int
  , rankingBlockPriorityCapacityBytes :: Int
  , rankingBlockPriorityCapacityExUnits :: Int
  }
  deriving stock (Eq, Show)

data EndorserBlockSummary = EndorserBlockSummary
  { endorserBlockId :: EbId
  , endorserBlockCapacityBytes :: Int
  , endorserBlockCapacityExUnits :: Int
  , endorserBlockUsedBytes :: Int
  , endorserBlockUsedExUnits :: Int
  , endorserBlockPriorityBytes :: Int
  , endorserBlockPriorityExUnits :: Int
  , endorserBlockStandardBytes :: Int
  , endorserBlockStandardExUnits :: Int
  , endorserBlockPrioritySignalCapacityBytes :: Int
  , endorserBlockPrioritySignalCapacityExUnits :: Int
  }
  deriving stock (Eq, Show)

data SelectionStep acc
  = Select acc
  | Skip
  | Stop

mkRankingBlockSummary :: RankingBlock -> Int -> Int -> Int -> Int -> [Tx] -> RankingBlockSummary
mkRankingBlockSummary block capacityBytes capacityExUnits priorityCapacityBytes priorityCapacityExUnits txs =
  RankingBlockSummary
    { rankingBlock = block
    , rankingBlockCapacityBytes = capacityBytes
    , rankingBlockCapacityExUnits = capacityExUnits
    , rankingBlockUsedBytes = totalBytes txs
    , rankingBlockUsedExUnits = totalExUnits txs
    , rankingBlockPriorityBytes = laneBytes Priority txs
    , rankingBlockPriorityExUnits = laneExUnits Priority txs
    , rankingBlockStandardBytes = laneBytes Standard txs
    , rankingBlockStandardExUnits = laneExUnits Standard txs
    , rankingBlockPriorityCapacityBytes = priorityCapacityBytes
    , rankingBlockPriorityCapacityExUnits = priorityCapacityExUnits
    }

mkEndorserBlockSummary :: EbId -> Int -> Int -> Int -> Int -> [Tx] -> EndorserBlockSummary
mkEndorserBlockSummary ebId capacityBytes capacityExUnits prioritySignalCapacityBytes prioritySignalCapacityExUnits txs =
  EndorserBlockSummary
    { endorserBlockId = ebId
    , endorserBlockCapacityBytes = capacityBytes
    , endorserBlockCapacityExUnits = capacityExUnits
    , endorserBlockUsedBytes = totalBytes txs
    , endorserBlockUsedExUnits = totalExUnits txs
    , endorserBlockPriorityBytes = laneBytes Priority txs
    , endorserBlockPriorityExUnits = laneExUnits Priority txs
    , endorserBlockStandardBytes = laneBytes Standard txs
    , endorserBlockStandardExUnits = laneExUnits Standard txs
    , endorserBlockPrioritySignalCapacityBytes = prioritySignalCapacityBytes
    , endorserBlockPrioritySignalCapacityExUnits = prioritySignalCapacityExUnits
    }

selectedTxBodies :: Map TxId Tx -> Seq TxId -> [Tx]
selectedTxBodies txs =
  mapMaybe (`Map.lookup` txs) . Foldable.toList

laneBytes :: Lane -> [Tx] -> Int
laneBytes lane =
  sum . fmap txBytes . filter ((== lane) . txLane)

laneExUnits :: Lane -> [Tx] -> Int
laneExUnits lane =
  sum . fmap txExUnits . filter ((== lane) . txLane)

totalBytes :: [Tx] -> Int
totalBytes =
  sum . fmap txBytes

totalExUnits :: [Tx] -> Int
totalExUnits =
  sum . fmap txExUnits

txBytes :: Tx -> Int
txBytes tx = tx.txBody._txSize

txExUnits :: Tx -> Int
txExUnits tx = tx.txBody._txScript._scriptExUnits

selectByBlockCapacity ::
  Int ->
  Int ->
  Map TxId Tx ->
  Seq TxId ->
  (Seq TxId, Seq TxId, (Int, Int))
selectByBlockCapacity byteCap exUnitCap txs =
  selectByBlockCapacityWith (const True) byteCap exUnitCap txs

selectPriorityByBlockCapacity ::
  Int ->
  Int ->
  Map TxId Tx ->
  Seq TxId ->
  (Seq TxId, Seq TxId, (Int, Int))
selectPriorityByBlockCapacity =
  selectByBlockCapacityWith ((== Priority) . txLane)

selectByBlockCapacityWith ::
  (Tx -> Bool) ->
  Int ->
  Int ->
  Map TxId Tx ->
  Seq TxId ->
  (Seq TxId, Seq TxId, (Int, Int))
selectByBlockCapacityWith acceptTx byteCap exUnitCap txs =
  selectAccumL advanceUsage (0, 0)
 where
  advanceUsage (usedBytes, usedExUnits) txId =
    case Map.lookup txId txs of
      Nothing -> Skip
      Just tx
        | not (acceptTx tx) -> Skip
        | otherwise ->
            let (bodyBytes, bodyExUnits) = txBlockResources tx
                usedBytes' = usedBytes + bodyBytes
                usedExUnits' = usedExUnits + bodyExUnits
             in if usedBytes' <= byteCap && usedExUnits' <= exUnitCap
                  then Select (usedBytes', usedExUnits')
                  else Stop

txBlockResources :: Tx -> (Int, Int)
txBlockResources tx =
  (txBytes tx, txExUnits tx)

selectAccumL ::
  (acc -> a -> SelectionStep acc) ->
  acc ->
  Seq a ->
  (Seq a, Seq a, acc)
selectAccumL advance acc0 =
  go acc0 mempty mempty
 where
  go acc selected skipped Empty =
    (selected, skipped, acc)
  go acc selected skipped (x :<| xs) =
    case advance acc x of
      Select acc' -> go acc' (selected |> x) skipped xs
      Skip -> go acc selected (skipped |> x) xs
      Stop -> (selected, skipped <> (x :<| xs), acc)
