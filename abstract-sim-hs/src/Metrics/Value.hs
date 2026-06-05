module Metrics.Value (
  ValueOutcome (..),
  valueByUrgency,
) where

import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Data.Maybe (mapMaybe)
import Metrics.Accumulator
import Transaction (Tx (..))
import Transaction qualified
import Types (BlockDelay (..), Lovelace, Urgency)

-- | Metric (2): retained vs lost transaction value.
data ValueOutcome = ValueOutcome
  { retainedValue :: Lovelace
  -- ^ value of txs successfully included
  , lostValue :: Lovelace
  -- ^ value of txs that expired or were evicted unincluded
  }
  deriving (Eq, Show)

valueByUrgency :: MetricsAcc -> Map Urgency ValueOutcome
valueByUrgency acc =
  Map.fromList (fmap valueForUrgency (observedUrgencies acc))
 where
  valueForUrgency urgency =
    (urgency, valueOutcomeWhere acc ((== urgency) . txUrgency))

valueOutcomeWhere :: MetricsAcc -> (Tx -> Bool) -> ValueOutcome
valueOutcomeWhere acc predicate =
  ValueOutcome
    { retainedValue = sumLovelace (fmap fst includedValueOutcomes)
    , lostValue =
        sumLovelace
          (fmap snd includedValueOutcomes <> fmap txValue evictedTxs)
    }
 where
  includedValueOutcomes =
    mapMaybe includedValueOutcome (filter (predicate . snd) (Map.toList acc.accSubmitted))
  evictedTxs =
    evictedTxsWhere acc predicate

  includedValueOutcome (txId, tx) = do
    latency <- includedBlockLatency acc txId
    let blockDelay = BlockDelay (fromIntegral latency)
    pure
      ( Transaction.retainedValue blockDelay tx
      , Transaction.lostValue blockDelay tx
      )
