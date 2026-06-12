module Run (
  Run (..),
  Seed,
  defaultSimConfigPath,
  run',
  runWithSeedToFile,
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
import Sim (initSimSt, step, unSimM)
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
runWithSeedToFile config eventsPath seed slots = do
  let st = initSimSt config (mkStdGen (fromInteger seed))
  (metricsAcc, _st') <-
    withFile eventsPath WriteMode \handle ->
      runTrace handle slots emptyMetricsAcc 0 st
  pure
    Run
      { _runResult = finalizeMetrics metricsConfig slots metricsAcc
      , _runDesign = config.simConfigDesign
      , _runSeed = seed
      }
 where
  metricsConfig = metricsConfigFrom config

  runTrace handle slotsRemaining metricsAcc nextEventNo simSt
    | slotsRemaining <= 0 = pure (metricsAcc, simSt)
    | otherwise = do
        let (events, simSt') =
              runState
                (runReaderT (unSimM step) config)
                simSt
            metricsAcc' = recordMetricsEvents metricsAcc events
        nextEventNo' <- writeTraceEvents handle nextEventNo (toList events)
        runTrace handle (slotsRemaining - 1) metricsAcc' nextEventNo' simSt'

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
