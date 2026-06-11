module Block (
  EbId (..),
  EndorserBlock (..),
  InclusionPoint (..),
  PendingEb (..),
  RankingBlock (..),
  BlockSummary (..),
  RankingBlockSummary (..),
  EndorserBlockSummary (..),
  mkRankingBlockSummary,
  mkEndorserBlockSummary,
  selectedTxBodies,
  laneBytes,
  txBytes,
  selectByBlockCapacity,
  selectByBlockCapacityFrom,
  selectFifoWithStandardCap,
  selectPriorityByBlockCapacity,
  txBlockResources,
)
where

import Data.Aeson (ToJSON (..), object, (.=))
import Data.Foldable qualified as Foldable
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Maybe (mapMaybe)
import Data.Sequence (Seq (..), (|>))
import Data.Set (Set)
import Data.Set qualified as Set
import Transaction (Lane (..), Script (_scriptExUnits), Tx (..), TxBody (..), TxId)
import Types (SlotNo)

data EbId = EbId Int deriving (Eq, Ord, Show)

instance ToJSON EbId where
  toJSON (EbId n) = toJSON n

data EndorserBlock = EndorserBlock
  { _ebTxs :: Set TxId
  , _ebId :: EbId
  }

instance ToJSON EndorserBlock where
  toJSON eb =
    object
      [ "id" .= eb._ebId
      , "txIds" .= Set.toAscList eb._ebTxs
      ]

data InclusionPoint
  = IncludedInRb
  | IncludedInEb EbId
  deriving stock (Eq, Show)

instance ToJSON InclusionPoint where
  toJSON = \case
    IncludedInRb ->
      object ["tag" .= ("IncludedInRb" :: String)]
    IncludedInEb ebId ->
      object
        [ "tag" .= ("IncludedInEb" :: String)
        , "ebId" .= ebId
        ]

data PendingEb = PendingEb {pendingEbId :: EbId, pendingEbAnnounced :: SlotNo}

instance ToJSON PendingEb where
  toJSON pending =
    object
      [ "id" .= pending.pendingEbId
      , "announced" .= pending.pendingEbAnnounced
      ]

data RankingBlock = CertifyingBlock EbId | PraosBlock [TxId]
  deriving stock (Eq, Show)

instance ToJSON RankingBlock where
  toJSON = \case
    CertifyingBlock ebId ->
      object
        [ "tag" .= ("CertifyingBlock" :: String)
        , "ebId" .= ebId
        ]
    PraosBlock txIds ->
      object
        [ "tag" .= ("PraosBlock" :: String)
        , "txIds" .= txIds
        ]

data BlockSummary
  = RankingBlockProduced RankingBlockSummary
  | EndorserBlockAnnounced EndorserBlockSummary
  | EndorserBlockCertified EndorserBlockSummary
  deriving stock (Eq, Show)

instance ToJSON BlockSummary where
  toJSON = \case
    RankingBlockProduced summary ->
      object
        [ "tag" .= ("RankingBlockProduced" :: String)
        , "summary" .= summary
        ]
    EndorserBlockAnnounced summary ->
      object
        [ "tag" .= ("EndorserBlockAnnounced" :: String)
        , "summary" .= summary
        ]
    EndorserBlockCertified summary ->
      object
        [ "tag" .= ("EndorserBlockCertified" :: String)
        , "summary" .= summary
        ]

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

instance ToJSON RankingBlockSummary where
  toJSON summary =
    object
      [ "block" .= summary.rankingBlock
      , "capacityBytes" .= summary.rankingBlockCapacityBytes
      , "capacityExUnits" .= summary.rankingBlockCapacityExUnits
      , "usedBytes" .= summary.rankingBlockUsedBytes
      , "usedExUnits" .= summary.rankingBlockUsedExUnits
      , "priorityBytes" .= summary.rankingBlockPriorityBytes
      , "priorityExUnits" .= summary.rankingBlockPriorityExUnits
      , "standardBytes" .= summary.rankingBlockStandardBytes
      , "standardExUnits" .= summary.rankingBlockStandardExUnits
      , "priorityCapacityBytes" .= summary.rankingBlockPriorityCapacityBytes
      , "priorityCapacityExUnits" .= summary.rankingBlockPriorityCapacityExUnits
      ]

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

instance ToJSON EndorserBlockSummary where
  toJSON summary =
    object
      [ "id" .= summary.endorserBlockId
      , "capacityBytes" .= summary.endorserBlockCapacityBytes
      , "capacityExUnits" .= summary.endorserBlockCapacityExUnits
      , "usedBytes" .= summary.endorserBlockUsedBytes
      , "usedExUnits" .= summary.endorserBlockUsedExUnits
      , "priorityBytes" .= summary.endorserBlockPriorityBytes
      , "priorityExUnits" .= summary.endorserBlockPriorityExUnits
      , "standardBytes" .= summary.endorserBlockStandardBytes
      , "standardExUnits" .= summary.endorserBlockStandardExUnits
      , "prioritySignalCapacityBytes" .= summary.endorserBlockPrioritySignalCapacityBytes
      , "prioritySignalCapacityExUnits" .= summary.endorserBlockPrioritySignalCapacityExUnits
      ]

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
selectByBlockCapacity =
  selectByBlockCapacityFrom (0, 0)

{- | Like 'selectByBlockCapacity', but starting from already-used resources —
for a second selection pass over the same block. The returned usage is the
cumulative total across passes.
-}
selectByBlockCapacityFrom ::
  (Int, Int) ->
  Int ->
  Int ->
  Map TxId Tx ->
  Seq TxId ->
  (Seq TxId, Seq TxId, (Int, Int))
selectByBlockCapacityFrom usedSoFar =
  selectByBlockCapacityWith (const True) usedSoFar

selectPriorityByBlockCapacity ::
  Int ->
  Int ->
  Map TxId Tx ->
  Seq TxId ->
  (Seq TxId, Seq TxId, (Int, Int))
selectPriorityByBlockCapacity =
  selectByBlockCapacityWith ((== Priority) . txLane) (0, 0)

selectFifoWithStandardCap ::
  Double ->
  Int ->
  Int ->
  Map TxId Tx ->
  Seq TxId ->
  (Seq TxId, Seq TxId, (Int, Int))
selectFifoWithStandardCap standardShare byteCap exUnitCap txs txIds =
  (selected, skipped, overallUsage)
 where
  (selected, skipped, (overallUsage, _standardUsage)) =
    selectAccumL advance ((0, 0), (0, 0)) txIds

  blockCaps = (byteCap, exUnitCap)
  standardCaps = (shareOf byteCap, shareOf exUnitCap)
  shareOf cap =
    floor (max 0 (min 1 standardShare) * fromIntegral cap) :: Int

  advance (used, standardUsed) txId =
    case Map.lookup txId txs of
      Nothing -> Skip
      Just tx
        | not (within (used `plus` cost) blockCaps) -> Stop
        | tx.txLane /= Standard -> Select (used `plus` cost, standardUsed)
        | within (standardUsed `plus` cost) standardCaps ->
            Select (used `plus` cost, standardUsed `plus` cost)
        | otherwise -> Skip
       where
        cost = txBlockResources tx

  plus (a, b) (c, d) = (a + c, b + d)
  within (a, b) (capA, capB) = a <= capA && b <= capB

selectByBlockCapacityWith ::
  (Tx -> Bool) ->
  (Int, Int) ->
  Int ->
  Int ->
  Map TxId Tx ->
  Seq TxId ->
  (Seq TxId, Seq TxId, (Int, Int))
selectByBlockCapacityWith acceptTx usedSoFar byteCap exUnitCap txs =
  selectAccumL advanceUsage usedSoFar
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
