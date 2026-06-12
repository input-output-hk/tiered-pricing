module Event
  ( SimEvent (..)
  )
where

import Actor (ActorId)
import Block (BlockSummary, InclusionPoint)
import Data.Aeson (ToJSON (..), object, (.=))
import Data.List.NonEmpty (NonEmpty)
import Data.List.NonEmpty qualified as NE
import Pricing (PriceUpdate (..))
import Transaction (EvictionReason, RejectReason, Tx, TxId)
import Types (Lovelace, SlotNo)

data SimEvent
  = TxSubmitted SlotNo ActorId Tx
  | TxAdmitted SlotNo TxId
  | TxRejected SlotNo TxId (NonEmpty RejectReason)
  | -- | The final 'Lovelace' is the realised fee: what the node actually
    -- charged at inclusion under the design's 'Design.FeeSemantics'.
    TxIncluded SlotNo TxId InclusionPoint Lovelace
  | TxEvicted SlotNo TxId EvictionReason
  | -- | A retried demand unit (identified by its origin tx number) declined
    -- to resubmit: congestion ate its surplus, or it ran out of attempts.
    -- Its remaining value is definitively lost at this slot.
    TxAbandoned SlotNo Int
  | BlockProduced SlotNo BlockSummary
  | PriceUpdated SlotNo PriceUpdate

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
    TxIncluded slot txId inclusionPoint realised ->
      object
        [ "tag" .= ("TxIncluded" :: String)
        , "slot" .= slot
        , "txId" .= txId
        , "inclusionPoint" .= inclusionPoint
        , "realisedFee" .= realised
        ]
    TxAbandoned slot originNumber ->
      object
        [ "tag" .= ("TxAbandoned" :: String)
        , "slot" .= slot
        , "originNumber" .= originNumber
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
    PriceUpdated slot update ->
      object
        [ "tag" .= ("PriceUpdated" :: String)
        , "slot" .= slot
        , "lane" .= update.priceUpdateLane
        , "oldCoeff" .= update.priceUpdateOldCoeff
        , "newCoeff" .= update.priceUpdateNewCoeff
        , "utilisation" .= update.priceUpdateUtilisation
        ]
