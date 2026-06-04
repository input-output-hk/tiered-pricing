module Config (
  SimConfig (..),
  simConfigDefault,
)
where

import Actor (LaneLatencyEstimate (..))
import Curve (Curves, curvesDefault)
import Design (Design, LaneStructure (Two), defaultDesign)
import Load (ArrivalProcess, severeCongestionLoad)
import Types (Duration (..))

data SimConfig s = SimConfig
  { simConfigDesign :: Design s
  , simConfigCurves :: Curves
  , simConfigF :: Double
  , simConfigD :: Int
  , simConfigLoad :: ArrivalProcess
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

{- | Current scalar ex-units caps use the same memory-equivalent convention as
the sampled curve: @mem + steps * (price_step / price_mem)@.

When '_scriptExUnits' is split into real Cardano dimensions, replace these
scalar caps with:

* RB: memory 72,000,000; steps 20,000,000,000.
* EB: memory 7,000,000,000; steps 2,000,000,000,000.
-}
simConfigDefault :: SimConfig 'Two
simConfigDefault =
  SimConfig
    { simConfigDesign = defaultDesign
    , simConfigCurves = curvesDefault
    , simConfigF = 0.05
    , simConfigD = 13
    , simConfigLoad = severeCongestionLoad
    , simConfigRbTxBytesCap = 90_112
    , simConfigRbExUnitsCap = 96_991_334
    , simConfigEbTxBytesCap = 12_000_000
    , simConfigEbStructureBytesCap = 512_000
    , simConfigEbExUnitsCap = 9_499_133_448
    , simConfigMempoolBytesCap = 24_000_000
    , simConfigLaneLatencyEstimate =
        LaneLatencyEstimate
          { expectedStandardLatency = Duration 50
          , expectedPriorityLatency = Duration 25
          }
    , simConfigPriceConvergenceBandPct = 0.05
    , simConfigLoadChangePct = 0.10
    }
