{- | Reified slicing dimensions for demand-unit metrics.

A metric family provides a kernel @[DemandUnit] -> stats@; this module owns
how runs are bucketed. The urgency × lane matrix is the literal product
'(>*<)' of the two base dimensions rather than a third hand-maintained
convention.
-}
module Metrics.Slice (
  Dimension (..),
  urgencyDim,
  laneDim,
  (>*<),
  sliceBy,
) where

import Data.Map.Strict (Map)
import Data.Map.Strict qualified as Map
import Metrics.Accumulator (DemandUnit (..), MetricsAcc, allLanes, observedUrgencies, unitLane, unitsWhere)
import Transaction (Lane)
import Types (Urgency)

{- | One way of bucketing demand units: which bucket keys exist for a run,
and which units belong to each.
-}
data Dimension k = Dimension
  { dimKeys :: MetricsAcc -> [k]
  -- ^ the bucket universe — observed in the run (urgencies) or exhaustive
  -- (lanes)
  , dimMatch :: k -> DemandUnit -> Bool
  }

urgencyDim :: Dimension Urgency
urgencyDim =
  Dimension
    { dimKeys = observedUrgencies
    , dimMatch = \urgency unit -> unit.unitUrgency == urgency
    }

{- | Lane attribution: the lane that actually served the unit when included,
otherwise the last lane it attempted ('unitLane').
-}
laneDim :: Dimension Lane
laneDim =
  Dimension
    { dimKeys = const allLanes
    , dimMatch = \lane unit -> unitLane unit == lane
    }

-- | The product dimension: every key pair, conjunction of the matches.
(>*<) :: Dimension a -> Dimension b -> Dimension (a, b)
dimA >*< dimB =
  Dimension
    { dimKeys = \acc -> (,) <$> dimA.dimKeys acc <*> dimB.dimKeys acc
    , dimMatch = \(a, b) unit -> dimA.dimMatch a unit && dimB.dimMatch b unit
    }

-- | Fan a metric kernel out over a dimension's buckets.
sliceBy :: Ord k => Dimension k -> ([DemandUnit] -> r) -> MetricsAcc -> Map k r
sliceBy dim kernel acc =
  Map.fromList
    [ (key, kernel (unitsWhere acc (dim.dimMatch key)))
    | key <- dim.dimKeys acc
    ]
