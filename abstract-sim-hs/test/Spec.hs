module Main (main) where

import Control.Monad (unless)
import Data.Aeson (eitherDecode)
import Data.ByteString.Lazy qualified as BL
import Design (defaultDesign)
import Parser
  ( ParseActorPolicy (..)
  , ParseActorPopulation (..)
  , ParseActorProfile (..)
  , ParseArrivalProcess (..)
  , ParseControllerConfig (..)
  , ParseControllerSignal (..)
  , ParseCurves (..)
  , ParseDesign (..)
  , ParseEip1559Controller (..)
  , ParseFeeSemantics (..)
  , ParseLanePricing (..)
  , ParseLaneLatencyEstimate (..)
  , ParseLaneStructure (..)
  , ParseReservationPolicy (..)
  , ParseSelectionPolicy (..)
  , ParseSimConfig (..)
  , fromParseDesign
  , fromParseSimConfig
  , parseDesign
  , parseSimConfig
  )
import System.Exit (exitFailure)

main :: IO ()
main = do
  assertDefaultDesignConfig
  assertDefaultSimConfig

assertDefaultDesignConfig :: IO ()
assertDefaultDesignConfig = do
  bytes <- BL.readFile "config/default-design.json"
  case eitherDecode bytes of
    Left err -> do
      putStrLn ("failed to parse default design config: " <> err)
      exitFailure
    Right actual ->
      assertEqual "default design config" expectedDefaultDesign actual
  assertRightEqual "default domain design" defaultDesign (fromParseDesign expectedDefaultDesign)
  actualDesign <- parseDesign "config/default-design.json"
  assertEqual "default design file parser" defaultDesign actualDesign

assertDefaultSimConfig :: IO ()
assertDefaultSimConfig = do
  expectedConfig <- expectRight "expected default sim config conversion" (fromParseSimConfig expectedDefaultSimConfig)
  bytes <- BL.readFile "config/default-sim-config.json"
  case eitherDecode bytes of
    Left err -> do
      putStrLn ("failed to parse default sim config: " <> err)
      exitFailure
    Right actual -> do
      assertEqual "default sim config syntax" expectedDefaultSimConfig actual
      assertRightEqual "default sim config conversion" expectedConfig (fromParseSimConfig actual)
  actualConfig <- parseSimConfig "config/default-sim-config.json"
  assertEqual "default sim config file parser" expectedConfig actualConfig

assertEqual :: (Eq a, Show a) => String -> a -> a -> IO ()
assertEqual label expected actual =
  unless (actual == expected) do
    putStrLn ("unexpected " <> label)
    putStrLn ("expected: " <> show expected)
    putStrLn ("actual:   " <> show actual)
    exitFailure

assertRightEqual :: (Eq a, Show a, Show err) => String -> a -> Either err a -> IO ()
assertRightEqual label expected actual =
  case actual of
    Left err -> do
      putStrLn ("unexpected " <> label <> " error")
      putStrLn ("error: " <> show err)
      exitFailure
    Right value ->
      assertEqual label expected value

expectRight :: (Show err) => String -> Either err a -> IO a
expectRight label actual =
  case actual of
    Left err -> do
      putStrLn ("unexpected " <> label <> " error")
      putStrLn ("error: " <> show err)
      exitFailure
    Right value ->
      pure value

expectedDefaultDesign :: ParseDesign
expectedDefaultDesign =
  ParseDesign
    { parseDesignLaneStructure = TwoP
    , parseDesignPricing = BothDynamicP
    , parseDesignReservationPolicy = PriorityReservationRb 90_112
    , parseDesignSelection = FifoP
    , parseDesignFeeSemantics = Eip1559P
    , parseDesignControllers =
        ParseControllerConfig
          { parseStandardController =
              Just
                ParseEip1559Controller
                  { parseControllerTargetUtilisation = 0.50
                  , parseControllerMaxChangeDenominator = 8
                  , parseControllerInitialCoefficient = 1.0
                  , parseControllerSignal = CapacityWeightedWindowP 20
                  }
          , parsePriorityController =
              Just
                ParseEip1559Controller
                  { parseControllerTargetUtilisation = 0.50
                  , parseControllerMaxChangeDenominator = 8
                  , parseControllerInitialCoefficient = 16.0
                  , parseControllerSignal = PriorityReservationUtilP
                  }
          , parseMultiplierFloor = Nothing
          , parseAbsoluteCoeffFloor = 1.0
          }
    }

expectedDefaultSimConfig :: ParseSimConfig
expectedDefaultSimConfig =
  ParseSimConfig
    { parseSimConfigDesign = expectedDefaultDesign
    , parseSimConfigCurves = DefaultCurvesP
    , parseSimConfigF = 0.05
    , parseSimConfigD = 13
    , parseSimConfigLoad = SevereCongestionLoadP
    , parseSimConfigActors =
        [ ParseActorPopulation
            { parseActorCount = 2
            , parseActorProfile =
                HonestProfileP
                  ParseActorPolicy
                    { parseActorFeeBuffer = 1.10
                    , parseActorMinValueFeeMultiple = 1.0
                    }
            }
        ]
    , parseSimConfigRbTxBytesCap = 90_112
    , parseSimConfigRbExUnitsCap = 96_991_334
    , parseSimConfigEbTxBytesCap = 12_000_000
    , parseSimConfigEbStructureBytesCap = 512_000
    , parseSimConfigEbExUnitsCap = 9_499_133_448
    , parseSimConfigMempoolBytesCap = 24_000_000
    , parseSimConfigLaneLatencyEstimate =
        ParseLaneLatencyEstimate
          { parseExpectedStandardLatency = 50
          , parseExpectedPriorityLatency = 25
          }
    , parseSimConfigPriceConvergenceBandPct = 0.05
    , parseSimConfigLoadChangePct = 0.10
    }
