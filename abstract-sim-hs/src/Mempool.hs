module Mempool
  ( Mempool (..)
  , emptyMempool
  , admitToMempool
  , setMempoolTxs
  , removeFromMempool
  )
where

import Data.Foldable (toList)
import Data.Sequence (Seq, (|>))
import Data.Sequence qualified as Seq
import Data.Set (Set)
import Data.Set qualified as Set
import Transaction (Tx (..), TxBody (..), TxId)

{- | The mempool owns its transactions: holders of a 'Mempool' never need a
side table to look bodies up, so the impossible miss has no representation.
-}
data Mempool = Mempool
  { mempoolTxs :: Seq Tx
  , mempoolBytes :: Int
  }

emptyMempool :: Mempool
emptyMempool =
  Mempool
    { mempoolTxs = mempty
    , mempoolBytes = 0
    }

admitToMempool :: Mempool -> Tx -> Mempool
admitToMempool mempool tx =
  Mempool
    { mempoolTxs = mempool.mempoolTxs |> tx
    , mempoolBytes = mempool.mempoolBytes + tx.txBody._txSize
    }

-- | Rebuild the mempool from a selection's remainder.
setMempoolTxs :: Seq Tx -> Mempool
setMempoolTxs txs =
  Mempool
    { mempoolTxs = txs
    , mempoolBytes = sum [tx.txBody._txSize | tx <- toList txs]
    }

removeFromMempool :: Set TxId -> Mempool -> Mempool
removeFromMempool txIds mempool =
  setMempoolTxs (Seq.filter (\tx -> tx.txId `Set.notMember` txIds) mempool.mempoolTxs)
