module Run (
  Run (..),
  RandomnessMode (..),
  Seed,
  defaultSimConfigPath,
  run',
  runWithSeed,
  runWithSeedUsing,
  runWithSeedToFile,
  runWithSeedToFileUsing,
) where

import Config (SimConfig (..))
import Control.Monad (foldM)
import Control.Monad.Reader (runReaderT)
import Control.Monad.State.Strict (runState)
import Data.Aeson (encode, object, (.=))
import Data.ByteString.Lazy qualified as BL
import Data.Foldable (Foldable (toList))
import Design (Design)
import Event (SimEvent)
import Metrics (Metrics, MetricsConfig (..), emptyMetricsAcc, finalizeMetrics, recordMetricsEvents)
import Parser (parseSimConfig)
import Sim (initSimSt, initSimStWithIndependentRngStreams, step, unSimM)
import System.IO (Handle, IOMode (WriteMode), withFile)
import System.Random (mkStdGen)

run' :: IO ()
run' = do
  config <- parseSimConfig defaultSimConfigPath
  runResult <- runWithSeedToFile config "events.jsonl" 0 2000
  print runResult._runResult

defaultSimConfigPath :: FilePath
defaultSimConfigPath = "config/default-sim-config.json"

runWithSeedToFile :: SimConfig -> FilePath -> Seed -> Int -> IO Run
runWithSeedToFile = runWithSeedToFileUsing SharedRandomness

runWithSeedToFileUsing :: RandomnessMode -> SimConfig -> FilePath -> Seed -> Int -> IO Run
runWithSeedToFileUsing randomness config eventsPath seed slots =
  withFile eventsPath WriteMode \handle ->
    runWithSeedAndSink randomness config (writeTraceEvents handle) 0 seed slots

-- | Run a simulation without serialising its event stream. Metrics are still
-- folded from exactly the same per-slot events as a traced run; only the
-- event sink differs.
runWithSeed :: SimConfig -> Seed -> Int -> IO Run
runWithSeed = runWithSeedUsing SharedRandomness

runWithSeedUsing :: RandomnessMode -> SimConfig -> Seed -> Int -> IO Run
runWithSeedUsing randomness config =
  runWithSeedAndSink randomness config (\() _events -> pure ()) ()

runWithSeedAndSink :: RandomnessMode -> SimConfig -> (sinkState -> [SimEvent] -> IO sinkState) -> sinkState -> Seed -> Int -> IO Run
runWithSeedAndSink randomness config sink initialSinkState seed slots = do
  let rootRng = mkStdGen (fromInteger seed)
      st = case randomness of
        SharedRandomness -> initSimSt config rootRng
        IndependentRandomness -> initSimStWithIndependentRngStreams config rootRng
  (metricsAcc, _sinkState, _st') <-
    runSimulation slots emptyMetricsAcc initialSinkState st
  pure
    Run
      { _runResult = finalizeMetrics metricsConfig slots metricsAcc
      , _runDesign = config.simConfigDesign
      , _runSeed = seed
      }
 where
  metricsConfig = metricsConfigFrom config

  runSimulation slotsRemaining metricsAcc sinkState simSt
    | slotsRemaining <= 0 = pure (metricsAcc, sinkState, simSt)
    | otherwise = do
        let (events, simSt') =
              runState
                (runReaderT (unSimM step) config)
                simSt
            metricsAcc' = recordMetricsEvents metricsAcc events
            eventList = toList events
        sinkState' <- sink sinkState eventList
        runSimulation (slotsRemaining - 1) metricsAcc' sinkState' simSt'

metricsConfigFrom :: SimConfig -> MetricsConfig
metricsConfigFrom config =
  MetricsConfig
    { metricsPriceConvergenceBandPct = config.simConfigPriceConvergenceBandPct
    }

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

data Run = Run
  { _runResult :: Metrics
  , _runDesign :: Design
  , _runSeed :: Seed
  }

type Seed = Integer

data RandomnessMode
  = SharedRandomness
  | IndependentRandomness
  deriving stock (Eq, Show)
