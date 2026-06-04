module Run where

import Config (SimConfig (..), simConfigDefault)
import Control.Monad (foldM)
import Control.Monad.Reader (runReaderT)
import Control.Monad.State.Strict (runState)
import Data.Aeson (encode, object, (.=))
import Data.ByteString.Lazy qualified as BL
import Data.Foldable (Foldable (toList))
import Design (Design, LaneStructure (Two))
import Event (SimEvent)
import Metrics (MetricsAcc, MetricsConfig (..), emptyMetricsAcc, finalizeMetrics, recordMetricsEvents)
import Result (Result (..))
import Sim (SimM, initSimSt, step, unSimM)
import System.IO (Handle, IOMode (WriteMode), withFile)
import System.Random (mkStdGen)

run :: IO ()
run = run'

run' :: IO ()
run' = do
  runResult <- runWithSeedToFile "events.jsonl" 0 2000
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

runWithSeedToFile :: FilePath -> Seed -> Int -> IO (Run 'Two)
runWithSeedToFile eventsPath seed slots = do
  let st = initSimSt simConfigDefault (mkStdGen (fromInteger seed))
  (metricsAcc, _st') <-
    withFile eventsPath WriteMode \handle ->
      runTrace handle slots emptyMetricsAcc 0 st
  pure
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

  runTrace handle slotsRemaining metricsAcc nextEventNo simSt
    | slotsRemaining <= 0 = pure (metricsAcc, simSt)
    | otherwise = do
        let (events, simSt') =
              runState
                (runReaderT (unSimM step) simConfigDefault)
                simSt
            metricsAcc' = recordMetricsEvents metricsAcc events
        nextEventNo' <- writeTraceEvents handle nextEventNo (toList events)
        runTrace handle (slotsRemaining - 1) metricsAcc' nextEventNo' simSt'

writeTraceEvents :: Handle -> Int -> [SimEvent] -> IO Int
writeTraceEvents handle firstEventNo events =
  foldM writeTraceEvent firstEventNo events
 where
  writeTraceEvent eventNo event = do
    BL.hPut
      handle
      ( encode
          ( object
              [ "eventNo" .= eventNo
              , "event" .= event
              ]
          )
      )
    BL.hPut handle "\n"
    pure (eventNo + 1)

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
