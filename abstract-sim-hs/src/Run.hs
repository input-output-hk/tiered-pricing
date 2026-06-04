module Run where

import Config (SimConfig (..), simConfigDefault)
import Control.Monad (foldM)
import Control.Monad.Reader (runReaderT)
import Control.Monad.State.Strict (runState)
import Design (Design, LaneStructure (Two))
import Metrics (MetricsAcc, MetricsConfig (..), emptyMetricsAcc, finalizeMetrics, recordMetricsEvents)
import Result (Result (..))
import Sim (SimM, initSimSt, step, unSimM)
import System.Random (mkStdGen)

run :: IO ()
run = run'

run' :: IO ()
run' = do
  let runResult = runWithSeed 0 2000
  print runResult._runResult

runWithSeed :: Seed -> Int -> Run 'Two
runWithSeed seed slots =
  Run
    { _runResult = Result [finalizeMetrics metricsConfig slots metricsAcc]
    , _runDesign = simConfigDefault.simConfigDesign
    , _runSeed = seed
    }
 where
  metricsConfig =
    MetricsConfig
      { metricsLoad = simConfigDefault.simConfigLoad
      , metricsPriceConvergenceBandPct = simConfigDefault.simConfigPriceConvergenceBandPct
      , metricsLoadChangePct = simConfigDefault.simConfigLoadChangePct
      }
  st = initSimSt simConfigDefault (mkStdGen (fromInteger seed))
  (metricsAcc, _st') =
    runState
      (runReaderT (unSimM (runMetrics slots)) simConfigDefault)
      st

runMetrics :: Int -> SimM 'Two MetricsAcc
runMetrics slots =
  foldM stepMetricsAcc emptyMetricsAcc [1 .. slots]
 where
  stepMetricsAcc acc _ = do
    events <- step
    pure (recordMetricsEvents acc events)

data Run (s :: LaneStructure) = Run -- To be derived from config
  { _runResult :: Result
  , _runDesign :: Design s
  , _runSeed :: Seed
  }

type Seed = Integer
