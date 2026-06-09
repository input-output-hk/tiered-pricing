module Metrics.Invariants (
  InvariantBreach (..),
  InvariantKind (..),
  invariantBreachesFrom,
) where

import Metrics.Accumulator (MetricsAcc)
import Types (Duration)

{- | The kind of invariant that was violated (metric (9)). Grounded in the
invariants stated in the design doc.
-}
data InvariantKind
  = -- | a stale tx made it into a certified EB or RB body
    StaleTxIncluded
  | -- | an included tx paid less than @tierCoeff * minfee@
    FeeBelowFloor
  | -- | the mempool exceeded its cap of 2× EB size
    MempoolCapExceeded
  | -- | an actor balance or refund pool went negative
    NegativeBalance
  deriving (Eq, Show)

-- | Metric (8): a single invariant breach observed during the run.
data InvariantBreach = InvariantBreach
  { breachKind :: InvariantKind
  , breachSlot :: Duration
  -- ^ slot at which the breach was detected
  , breachDetail :: String
  -- ^ human-readable context
  }
  deriving (Eq, Show)

invariantBreachesFrom :: MetricsAcc -> [InvariantBreach]
invariantBreachesFrom _ =
  []
