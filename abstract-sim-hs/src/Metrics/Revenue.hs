module Metrics.Revenue (
  Revenue (..),
  revenueFrom,
) where

import Data.Map.Strict qualified as Map
import Metrics.Accumulator
import Transaction (Tx (..), TxBody (..), subtractLovelace)
import Types (Lovelace (..))

{- | Metric (5): revenue — fees collected and refunds returned. Net revenue is
@feesCollected - refundsPaid@, left for the consumer to compute.
-}
data Revenue = Revenue
  { feesCollected :: Lovelace
  -- ^ total posted fees of included txs
  , refundsPaid :: Lovelace
  -- ^ total refunded for overpayment vs the realised fee at inclusion
  -- (nonzero only under 'Design.Eip1559' fee semantics)
  }
  deriving (Eq, Show)

revenueFrom :: MetricsAcc -> Revenue
revenueFrom acc =
  Revenue
    { feesCollected = sumLovelace (fmap txFee included)
    , refundsPaid = sumLovelace (fmap refund included)
    }
 where
  included = includedTxsWhere acc (const True)
  refund tx =
    case Map.lookup tx.txId acc.accRealisedFee of
      Nothing -> Lovelace 0
      Just realised -> subtractLovelace (txFee tx) realised

txFee :: Tx -> Lovelace
txFee tx =
  tx.txBody._txFee
