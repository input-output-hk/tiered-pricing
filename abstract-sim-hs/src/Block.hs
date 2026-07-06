module Block (
  EbId (..),
  EndorserBlock (..),
  InclusionPoint (..),
  PendingEb (..),
  BlockUsage (..),
  BlockSummary (..),
  RbSelectionMode (..),
  mkBlockUsage,
  prioritySignalCapacity,
  selectTxsByPolicy,
  selectRbTxs,
  nextEbId,
  selectByBlockCapacity,
  selectByBlockCapacityFrom,
  selectFifoWithStandardCap,
  selectPriorityByBlockCapacity,
  txBlockResources,
  txsBytes,
)
where

import Data.Aeson (ToJSON (..), object, (.=))
import Data.Aeson.Key qualified as Key
import Data.Aeson.Types (Pair)
import Data.List qualified as List
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Sequence (Seq (..), (|>))
import Design (ReservationPolicy (..), SelectionPolicy (..))
import Resource (Bytes (..), ExUnits (..), Resources (..), fitsWithin, scaleResources)
import Transaction (Script (_scriptExUnits), Tx (..), TxBody (..), TxId)
import Types (Lane (..), PerLane (..), SlotNo, lanes)

data EbId = EbId Int deriving (Eq, Ord, Show)

instance ToJSON EbId where
  toJSON (EbId n) = toJSON n

data EndorserBlock = EndorserBlock
  { _ebTxs :: [Tx]
  -- ^ announced bodies, in selection order — the EB owns its payload, so
  -- certification re-validates the very txs it announced
  , _ebId :: EbId
  }

instance ToJSON EndorserBlock where
  toJSON eb =
    object
      [ "id" .= eb._ebId
      , "txIds" .= List.sort (fmap (.txId) eb._ebTxs)
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

-- | What a produced payload used, against what capacity.
data BlockUsage = BlockUsage
  { usageCapacity :: Resources
  , usageUsed :: Resources
  , usageLanes :: PerLane Resources
  -- ^ the used resources, split by lane
  , usageSignalCapacity :: Resources
  -- ^ the priority lane's effective capacity — what the priority
  -- controller's reservation-utilisation signal divides by
  }
  deriving stock (Eq, Show)

{- | One slot's produced block, as the price controllers and metrics see it.
A certifying RB is payload-free by construction: there is no usage to
fabricate and none to misread.
-}
data BlockSummary
  = RbPraos [TxId] BlockUsage
  | RbCertifying EbId
  | EbAnnounced EbId BlockUsage
  | EbCertified EbId BlockUsage
  deriving stock (Eq, Show)

-- | Whether a transaction-carrying RB applied the priority-only rule or used
-- all of its capacity for a mixed block because the queued transactions fit.
data RbSelectionMode = ReservedRb | MixedRb
  deriving stock (Eq, Show)

{- | The wire encoding predates this type's shape and is pinned by the viz
ingester: ranking blocks carry a nested @block@ tag (Praos\/Certifying), a
certifying RB serialises as an all-zero usage, and the signal-capacity keys
are @priorityCapacity*@ on RBs but @prioritySignalCapacity*@ on EBs.
-}
instance ToJSON BlockSummary where
  toJSON = \case
    RbPraos txIds usage ->
      object
        [ "tag" .= ("RankingBlockProduced" :: String)
        , "summary"
            .= object
              ( ("block" .= object ["tag" .= ("PraosBlock" :: String), "txIds" .= txIds])
                  : usagePairs "priorityCapacity" usage
              )
        ]
    RbCertifying ebId ->
      object
        [ "tag" .= ("RankingBlockProduced" :: String)
        , "summary"
            .= object
              ( ("block" .= object ["tag" .= ("CertifyingBlock" :: String), "ebId" .= ebId])
                  : usagePairs "priorityCapacity" emptyUsage
              )
        ]
    EbAnnounced ebId usage ->
      endorserJson "EndorserBlockAnnounced" ebId usage
    EbCertified ebId usage ->
      endorserJson "EndorserBlockCertified" ebId usage
   where
    endorserJson tag ebId usage =
      object
        [ "tag" .= (tag :: String)
        , "summary" .= object (("id" .= ebId) : usagePairs "prioritySignalCapacity" usage)
        ]
    emptyUsage =
      BlockUsage
        { usageCapacity = mempty
        , usageUsed = mempty
        , usageLanes = pure mempty
        , usageSignalCapacity = mempty
        }

usagePairs :: String -> BlockUsage -> [Pair]
usagePairs signalCapacityKey usage =
  [ "capacityBytes" .= usage.usageCapacity.resBytes.unBytes
  , "capacityExUnits" .= usage.usageCapacity.resExUnits.unExUnits
  , "usedBytes" .= usage.usageUsed.resBytes.unBytes
  , "usedExUnits" .= usage.usageUsed.resExUnits.unExUnits
  , "priorityBytes" .= usage.usageLanes.perPriority.resBytes.unBytes
  , "priorityExUnits" .= usage.usageLanes.perPriority.resExUnits.unExUnits
  , "standardBytes" .= usage.usageLanes.perStandard.resBytes.unBytes
  , "standardExUnits" .= usage.usageLanes.perStandard.resExUnits.unExUnits
  , Key.fromString (signalCapacityKey <> "Bytes") .= usage.usageSignalCapacity.resBytes.unBytes
  , Key.fromString (signalCapacityKey <> "ExUnits") .= usage.usageSignalCapacity.resExUnits.unExUnits
  ]

mkBlockUsage :: Resources -> Resources -> [Tx] -> BlockUsage
mkBlockUsage capacity signalCapacity txs =
  BlockUsage
    { usageCapacity = capacity
    , usageUsed = foldMap txBlockResources txs
    , usageLanes = laneUsage <$> lanes
    , usageSignalCapacity = signalCapacity
    }
 where
  laneUsage lane =
    foldMap txBlockResources (filter ((== lane) . (.txLane)) txs)

{- | The priority lane's effective RB capacity under a reservation policy:
the reservation caps its bytes, never its ex-units.
-}
prioritySignalCapacity :: ReservationPolicy -> Resources -> Resources
prioritySignalCapacity reservation rbCapacity =
  case reservation of
    PriorityReservationRb reservationBytes ->
      rbCapacity{resBytes = min rbCapacity.resBytes (Bytes reservationBytes)}
    PriorityReservationRbEbThreshold reservationBytes _ ->
      rbCapacity{resBytes = min rbCapacity.resBytes (Bytes reservationBytes)}
    NoReservation -> rbCapacity

{- | How a producer orders the mempool into a block, absent any reservation
rule. EBs always use this directly: the RB reservation does not constrain
EB content.
-}
selectTxsByPolicy ::
  SelectionPolicy ->
  Resources ->
  Seq Tx ->
  (Seq Tx, Seq Tx, Resources)
selectTxsByPolicy selection capacity txs =
  case selection of
    Fifo ->
      selectByBlockCapacity capacity txs
    PriorityFirst ->
      let (prioritySelected, afterPriority, priorityUsage) =
            selectPriorityByBlockCapacity capacity txs
          (standardSelected, remainingMempool, totalUsage) =
            selectByBlockCapacityFrom priorityUsage capacity afterPriority
       in (prioritySelected <> standardSelected, remainingMempool, totalUsage)
    FifoWithStandardCap standardShare ->
      selectFifoWithStandardCap standardShare capacity txs

{- | Ranking-block selection under the design's reservation policy. Both
reservation policies keep the RB priority-only; the EB-threshold policy
differs only in when an EB may be announced, which is decided at
announcement time (see @ebNeeded@ in "Sim"), not here.
-}
selectRbTxs ::
  SelectionPolicy ->
  ReservationPolicy ->
  Resources ->
  Seq Tx ->
  (Seq Tx, Seq Tx, Resources, RbSelectionMode)
selectRbTxs selection reservation rbCapacity txs =
  case reservation of
    PriorityReservationRb{} ->
      withMode ReservedRb $ selectPriorityByBlockCapacity (prioritySignalCapacity reservation rbCapacity) txs
    PriorityReservationRbEbThreshold{} ->
      withMode ReservedRb $ selectPriorityByBlockCapacity (prioritySignalCapacity reservation rbCapacity) txs
    NoReservation ->
      withMode MixedRb $ selectTxsByPolicy selection rbCapacity txs
 where
  withMode mode (selected, remaining, usage) = (selected, remaining, usage, mode)

-- | Total body bytes of a set of transactions — the byte footprint the
-- set would occupy as an EB payload.
txsBytes :: Seq Tx -> Int
txsBytes = sum . fmap ((._txSize) . (.txBody))

nextEbId :: Map EbId EndorserBlock -> EbId
nextEbId ebs =
  case Map.lookupMax ebs of
    Nothing -> EbId 0
    Just (EbId n, _) -> EbId (n + 1)

data SelectionStep acc
  = Select acc
  | Skip
  | Stop

selectByBlockCapacity ::
  Resources ->
  Seq Tx ->
  (Seq Tx, Seq Tx, Resources)
selectByBlockCapacity =
  selectByBlockCapacityFrom mempty

{- | Like 'selectByBlockCapacity', but starting from already-used resources —
for a second selection pass over the same block. The returned usage is the
cumulative total across passes.
-}
selectByBlockCapacityFrom ::
  Resources ->
  Resources ->
  Seq Tx ->
  (Seq Tx, Seq Tx, Resources)
selectByBlockCapacityFrom usedSoFar =
  selectByBlockCapacityWith (const True) usedSoFar

selectPriorityByBlockCapacity ::
  Resources ->
  Seq Tx ->
  (Seq Tx, Seq Tx, Resources)
selectPriorityByBlockCapacity =
  selectByBlockCapacityWith ((== Priority) . (.txLane)) mempty

selectFifoWithStandardCap ::
  Double ->
  Resources ->
  Seq Tx ->
  (Seq Tx, Seq Tx, Resources)
selectFifoWithStandardCap standardShare capacity txs =
  (selected, skipped, overallUsage)
 where
  (selected, skipped, (overallUsage, _standardUsage)) =
    selectAccumL advance (mempty, mempty) txs

  standardCapacity = scaleResources standardShare capacity

  advance (used, standardUsed) tx
    | not ((used <> cost) `fitsWithin` capacity) = Stop
    | tx.txLane /= Standard = Select (used <> cost, standardUsed)
    | (standardUsed <> cost) `fitsWithin` standardCapacity =
        Select (used <> cost, standardUsed <> cost)
    | otherwise = Skip
   where
    cost = txBlockResources tx

selectByBlockCapacityWith ::
  (Tx -> Bool) ->
  Resources ->
  Resources ->
  Seq Tx ->
  (Seq Tx, Seq Tx, Resources)
selectByBlockCapacityWith acceptTx usedSoFar capacity =
  selectAccumL advanceUsage usedSoFar
 where
  advanceUsage used tx
    | not (acceptTx tx) = Skip
    | otherwise =
        let used' = used <> txBlockResources tx
         in if used' `fitsWithin` capacity
              then Select used'
              else Stop

txBlockResources :: Tx -> Resources
txBlockResources tx =
  Resources
    { resBytes = Bytes tx.txBody._txSize
    , resExUnits = ExUnits tx.txBody._txScript._scriptExUnits
    }

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
