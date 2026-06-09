module Config (
  SimConfig (..),
)
where

import Actor (Actor, LaneLatencyEstimate (..))
import Curve (Curves)
import Design (Design)
import Load (ArrivalProcess)

data SimConfig = SimConfig
  { simConfigDesign :: Design
  , simConfigCurves :: Curves
  , simConfigF :: Double
  , simConfigD :: Int
  , simConfigLoad :: ArrivalProcess
  , simConfigActors :: [Actor]
  , simConfigRbTxBytesCap :: Int
  , simConfigRbExUnitsCap :: Int
  , simConfigEbTxBytesCap :: Int
  , simConfigEbStructureBytesCap :: Int
  , simConfigEbExUnitsCap :: Int
  , simConfigMempoolBytesCap :: Int
  , simConfigLaneLatencyEstimate :: LaneLatencyEstimate
  , simConfigPriceConvergenceBandPct :: Double
  , simConfigLoadChangePct :: Double
  }
  deriving stock (Eq, Show)

{- | Current scalar ex-units caps use the same memory-equivalent convention as
the sampled curve: @mem + steps * (price_step / price_mem)@.

When '_scriptExUnits' is split into real Cardano dimensions, replace these
scalar caps with:

* RB: memory 72,000,000; steps 20,000,000,000.
* EB: memory 7,000,000,000; steps 2,000,000,000,000.
-}
