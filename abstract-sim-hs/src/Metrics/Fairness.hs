module Metrics.Fairness (
  Fairness (..),
  fairnessFrom,
) where

import Data.Map.Strict qualified as Map
import Data.Maybe (mapMaybe)
import Data.Set qualified as Set
import Metrics.Accumulator

-- | Metric (7): fairness and starvation.
data Fairness = Fairness
  { starvedTxs :: Int
  -- ^ admissible txs never included before expiry
  , fairnessIndex :: Double
  -- ^ Jain's fairness index over per-actor inclusion, in [0,1]
  }
  deriving (Eq, Show)

fairnessFrom :: MetricsAcc -> Fairness
fairnessFrom acc =
  Fairness
    { starvedTxs = Set.size endOfRunUnincluded
    , fairnessIndex = jainIndex includedCounts
    }
 where
  endOfRunUnincluded =
    acc.accAdmitted
      `Set.difference` Map.keysSet acc.accIncludedAt
      `Set.difference` acc.accEvicted
  submittedActors =
    Set.fromList (Map.elems acc.accSubmittedByActor)
  includedCounts =
    fmap actorIncludedCount (Set.toList submittedActors)
  actorInclusions =
    Map.fromListWith
      (+)
      (fmap actorInclusion (mapMaybe includedActorId (Map.keys acc.accIncludedAt)))

  actorIncludedCount actorId =
    Map.findWithDefault 0 actorId actorInclusions

  includedActorId txId =
    Map.lookup txId acc.accSubmittedByActor

  actorInclusion actorId =
    (actorId, 1 :: Int)
