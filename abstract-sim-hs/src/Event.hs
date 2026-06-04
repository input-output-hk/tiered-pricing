module Event
  ( SimEvent (..)
  )
where

import Actor (ActorId)
import Block (BlockSummary, InclusionPoint)
import Data.Aeson (ToJSON (..), object, (.=))
import Data.List.NonEmpty (NonEmpty)
import Data.List.NonEmpty qualified as NE
import Transaction (EvictionReason, Lane, RejectReason, Tx, TxId)
import Types (SlotNo)

data SimEvent
  = TxSubmitted SlotNo ActorId Tx
  | TxAdmitted SlotNo TxId
  | TxRejected SlotNo TxId (NonEmpty RejectReason)
  | TxIncluded SlotNo TxId InclusionPoint
  | TxEvicted SlotNo TxId EvictionReason
  | BlockProduced SlotNo BlockSummary
  | PriceUpdated SlotNo Lane Double Double Double

instance ToJSON SimEvent where
  toJSON = \case
    TxSubmitted slot actorId tx ->
      object
        [ "tag" .= ("TxSubmitted" :: String)
        , "slot" .= slot
        , "actorId" .= actorId
        , "tx" .= tx
        ]
    TxAdmitted slot txId ->
      object
        [ "tag" .= ("TxAdmitted" :: String)
        , "slot" .= slot
        , "txId" .= txId
        ]
    TxRejected slot txId reasons ->
      object
        [ "tag" .= ("TxRejected" :: String)
        , "slot" .= slot
        , "txId" .= txId
        , "reasons" .= NE.toList reasons
        ]
    TxIncluded slot txId inclusionPoint ->
      object
        [ "tag" .= ("TxIncluded" :: String)
        , "slot" .= slot
        , "txId" .= txId
        , "inclusionPoint" .= inclusionPoint
        ]
    TxEvicted slot txId reason ->
      object
        [ "tag" .= ("TxEvicted" :: String)
        , "slot" .= slot
        , "txId" .= txId
        , "reason" .= reason
        ]
    BlockProduced slot summary ->
      object
        [ "tag" .= ("BlockProduced" :: String)
        , "slot" .= slot
        , "summary" .= summary
        ]
    PriceUpdated slot lane oldCoeff newCoeff utilisation ->
      object
        [ "tag" .= ("PriceUpdated" :: String)
        , "slot" .= slot
        , "lane" .= lane
        , "oldCoeff" .= oldCoeff
        , "newCoeff" .= newCoeff
        , "utilisation" .= utilisation
        ]
