module Main (main) where

import Control.Monad (unless)
import Data.Aeson (eitherDecode)
import Data.ByteString.Lazy qualified as BL
import Design (defaultDesign)
import Parser
  ( ParseActorPopulation (..)
  , ParseActorType (..)
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
  assertDesignFixture
  assertSimConfigFixture
  assertLiveConfigsParse

{- The JSON under test/fixtures/ is test-owned data, frozen independently of
config/, which is free to change with whatever experiment is being run. Only
the fixtures carry content assertions.
-}
assertDesignFixture :: IO ()
assertDesignFixture = do
  bytes <- BL.readFile "test/fixtures/design.json"
  case eitherDecode bytes of
    Left err -> do
      putStrLn ("failed to parse design fixture: " <> err)
      exitFailure
    Right actual ->
      assertEqual "design fixture syntax" expectedFixtureDesign actual
  assertRightEqual "design fixture conversion" defaultDesign (fromParseDesign expectedFixtureDesign)
  actualDesign <- parseDesign "test/fixtures/design.json"
  assertEqual "design fixture file parser" defaultDesign actualDesign

assertSimConfigFixture :: IO ()
assertSimConfigFixture = do
  expectedConfig <- expectRight "expected sim config fixture conversion" (fromParseSimConfig expectedFixtureSimConfig)
  bytes <- BL.readFile "test/fixtures/sim-config.json"
  case eitherDecode bytes of
    Left err -> do
      putStrLn ("failed to parse sim config fixture: " <> err)
      exitFailure
    Right actual -> do
      assertEqual "sim config fixture syntax" expectedFixtureSimConfig actual
      assertRightEqual "sim config fixture conversion" expectedConfig (fromParseSimConfig actual)
  actualConfig <- parseSimConfig "test/fixtures/sim-config.json"
  assertEqual "sim config fixture file parser" expectedConfig actualConfig

-- The live configs are not content-asserted — only that they still parse.
assertLiveConfigsParse :: IO ()
assertLiveConfigsParse = do
  _ <- parseDesign "config/default-design.json"
  _ <- parseSimConfig "config/default-sim-config.json"
  pure ()

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

expectedFixtureDesign :: ParseDesign
expectedFixtureDesign =
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

expectedFixtureSimConfig :: ParseSimConfig
expectedFixtureSimConfig =
  ParseSimConfig
    { parseSimConfigDesign = expectedFixtureDesign
    , parseSimConfigCurves = DefaultCurvesP
    , parseSimConfigF = 0.05
    , parseSimConfigD = 13
    , parseSimConfigLoad = SevereCongestionLoadP
    , parseSimConfigActors =
        [ ParseActorPopulation
            { parseActorCount = 2
            , parseActorType = HonestActorP
            , parseActorFeeBuffer = 2.0
            , parseActorMinValueFeeMultiple = 1.0
            , parseActorValueMultiplier = 1.0
            , parseActorUrgencyMultiplier = 1.0
            }
        ]
    , parseSimConfigRbTxBytesCap = 90_112
    , parseSimConfigRbExUnitsCap = 96_991_334
    , parseSimConfigEbTxBytesCap = 12_000_000
    , parseSimConfigEbStructureBytesCap = 512_000
    , parseSimConfigEbExUnitsCap = 9_499_133_448
    , parseSimConfigMempoolBytesCap = 24_000_000
    , parseSimConfigAdmissionHeadroomUpdates = 1
    , parseSimConfigLaneLatencyEstimate =
        ParseLaneLatencyEstimate
          { parseExpectedStandardLatency = 50
          , parseExpectedPriorityLatency = 25
          }
    , parseSimConfigPriceConvergenceBandPct = 0.05
    , parseSimConfigLoadChangePct = 0.10
    , parseSimConfigRetryPolicy = Nothing
    }
