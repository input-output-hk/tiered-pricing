module Config (
  SimConfig (..),
)
where

import Actor (Actor, LaneLatencyEstimate (..))
import Curve (Curves)
import Design (Design)
import Load (ArrivalProcess)
import Retry (RetryPolicy)

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
  , simConfigAdmissionHeadroomUpdates :: Int
  -- ^ Node admission policy: how many worst-case controller steps the posted
  -- fee must survive to enter the mempool. 0 admits anything covering
  -- today's quote; 1 admits only what an EB producer could take right now;
  -- larger values approximate "unlikely to be out-priced before inclusion"
  -- (~ceil(f x expected lane latency)). See 'Sim.admissionRequiredFee'.
  , simConfigLaneLatencyEstimate :: LaneLatencyEstimate
  , simConfigPriceConvergenceBandPct :: Double
  , simConfigLoadChangePct :: Double
  , simConfigRetryPolicy :: RetryPolicy
  -- ^ how rejected and evicted demand resubmits; defaults to 'Retry.noRetries'
  -- when absent from the config file
  }
  deriving stock (Eq, Show)

{- | Current scalar ex-units caps use the same memory-equivalent convention as
the sampled curve: @mem + steps * (price_step / price_mem)@.

When '_scriptExUnits' is split into real Cardano dimensions, replace these
scalar caps with:

* RB: memory 72,000,000; steps 20,000,000,000.
* EB: memory 7,000,000,000; steps 2,000,000,000,000.
-}
