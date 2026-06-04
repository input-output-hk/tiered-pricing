module Event
  ( SimEvent (..)
  )
where

import Actor (ActorId)
import Block (BlockSummary, InclusionPoint)
import Data.List.NonEmpty (NonEmpty)
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
