module Mempool
  ( Mempool (..)
  , emptyMempool
  , admitToMempool
  , setMempoolTxIds
  , removeFromMempool
  )
where

import Data.Foldable (Foldable (toList))
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Sequence (Seq, (|>))
import Data.Sequence qualified as Seq
import Data.Set (Set)
import Data.Set qualified as Set
import Transaction (Tx (..), TxBody (..), TxId)

data Mempool = Mempool
  { mempoolTxIds :: Seq TxId
  , mempoolBytes :: Int
  }

emptyMempool :: Mempool
emptyMempool =
  Mempool
    { mempoolTxIds = mempty
    , mempoolBytes = 0
    }

admitToMempool :: Mempool -> Tx -> Mempool
admitToMempool mempool tx =
  mempool
    { mempoolTxIds = mempool.mempoolTxIds |> tx.txId
    , mempoolBytes = mempool.mempoolBytes + tx.txBody._txSize
    }

setMempoolTxIds :: Map TxId Tx -> Seq TxId -> Mempool
setMempoolTxIds txs txIds =
  Mempool
    { mempoolTxIds = txIds
    , mempoolBytes = txIdsBytes txs txIds
    }

removeFromMempool :: Map TxId Tx -> Set TxId -> Mempool -> Mempool
removeFromMempool txs txIds mempool =
  setMempoolTxIds txs (Seq.filter (`Set.notMember` txIds) mempool.mempoolTxIds)

txIdsBytes :: Map TxId Tx -> Seq TxId -> Int
txIdsBytes txs =
  sum . fmap txIdBytes . toList
 where
  txIdBytes txId =
    maybe 0 txSize (Map.lookup txId txs)

txSize :: Tx -> Int
txSize tx =
  tx.txBody._txSize
