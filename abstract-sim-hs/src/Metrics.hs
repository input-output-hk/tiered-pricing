{- | Per-run metrics for one abstract simulation run.

Each 'Metrics' value is the post-fold report for a single run (one candidate
design × seed): the run threads its own state and accumulator, and reduces
to these finished statistics at the end. The surrounding @Stream Metrics@ is
a list of such reports, one per sweep point.

Field set mirrors the @Metrics@ section of @abstract-experiment-design.md@.
-}
module Metrics (
  module Metrics.Accumulator,
  module Metrics.Fold,
  module Metrics.Stats,
  module Metrics.Types,
) where

import Metrics.Accumulator
import Metrics.Fold
import Metrics.Stats
import Metrics.Types
