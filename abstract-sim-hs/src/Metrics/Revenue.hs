module Metrics.Revenue (
  Revenue (..),
  revenueFrom,
) where

import Metrics.Accumulator
import Transaction (Tx (..), TxBody (..))
import Types (Lovelace (..))

{- | Metric (5): revenue — fees collected and refunds returned. Net revenue is
@feesCollected - refundsPaid@, left for the consumer to compute.
-}
data Revenue = Revenue
  { feesCollected :: Lovelace
  -- ^ total fees paid by included txs
  , refundsPaid :: Lovelace
  -- ^ total refunded for overpayment vs the realised dynamic price
  }
  deriving (Eq, Show)

revenueFrom :: MetricsAcc -> Revenue
revenueFrom acc =
  Revenue
    { feesCollected = sumLovelace (fmap txFee (includedTxsWhere acc (const True)))
    , refundsPaid = Lovelace 0
    }

txFee :: Tx -> Lovelace
txFee tx =
  tx.txBody._txFee
