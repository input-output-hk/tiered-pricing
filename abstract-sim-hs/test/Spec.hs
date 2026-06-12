module Main (main) where

import Control.Monad (unless)
import Data.Aeson (eitherDecode)
import Data.ByteString.Lazy qualified as BL
import Design (Design (..), FeeSemantics (..), defaultDesign)
import Pricing (admissionRequiredFee, coversProducerHeadroom, initialPrices)
import Transaction (Lane (..), Script (..), Tx (..), TxBody (..), hash)
import Types (Duration (..), Lovelace (..), SlotNo (..), Urgency (..))
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
import Sweep (SweepSpec (..), SweepVariant (..), loadSweepSpec)
import System.Exit (exitFailure)

main :: IO ()
main = do
  assertDesignFixture
  assertSimConfigFixture
  assertSweepFixture
  assertLiveConfigsParse
  assertHeadroomInvariant

{- | The design's central safety argument, as code: the admission fee is
monotone in the headroom horizon, and a tx admitted with a horizon of one
worst-case controller step satisfies the EB producer-headroom bound at the
same prices ('Pricing.coversProducerHeadroom').
-}
assertHeadroomInvariant :: IO ()
assertHeadroomInvariant = do
  let controllers = defaultDesign.designControllers
      prices = initialPrices defaultDesign
      semanticsUnderTest =
        [ FixedFee
        , Eip1559
        , HonourSubmissionQuoteFor (Duration 20)
        ]
  sequence_
    [ assertTrue
        ("admission fee monotone in headroom: " <> show semantics <> ", lane " <> show lane <> ", n=" <> show n)
        ( admissionRequiredFee controllers n semantics prices (testTx lane)
            <= admissionRequiredFee controllers (n + 1) semantics prices (testTx lane)
        )
    | semantics <- semanticsUnderTest
    , lane <- [Standard, Priority]
    , n <- [0 .. 5]
    ]
  sequence_
    [ assertTrue
        ("admission at horizon 1 implies producer headroom: " <> show semantics <> ", lane " <> show lane)
        ( let admitted = withFee (admissionRequiredFee controllers 1 semantics prices (testTx lane)) (testTx lane)
           in coversProducerHeadroom controllers semantics prices admitted
        )
    | semantics <- semanticsUnderTest
    , lane <- [Standard, Priority]
    ]

withFee :: Lovelace -> Tx -> Tx
withFee fee tx =
  tx{txBody = tx.txBody{_txFee = fee}}

testTx :: Lane -> Tx
testTx lane =
  Tx
    { txId = hash body
    , txBody = body
    , txSubmitted = SlotNo 0
    , txValue = Lovelace 1_000_000
    , txUrgency = Exponential 0.04
    , txLane = lane
    , txOriginNumber = 1
    , txAttempt = 1
    , txOriginSubmitted = SlotNo 0
    }
 where
  body =
    TxBody
      { _txSize = 500
      , _txScript = Script{_scriptSize = 0, _scriptExUnits = 0}
      , _txDependsOn = mempty
      , _txFee = Lovelace 0
      , _txNumber = 1
      }

assertTrue :: String -> Bool -> IO ()
assertTrue label cond =
  unless cond do
    putStrLn ("assertion failed: " <> label)
    exitFailure

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

assertSweepFixture :: IO ()
assertSweepFixture = do
  spec <- loadSweepSpec "test/fixtures/sweep.json"
  assertEqual "sweep fixture" expectedFixtureSweep spec

expectedFixtureSweep :: SweepSpec
expectedFixtureSweep =
  SweepSpec
    { sweepDescription = Just "fixture"
    , sweepSeeds = 3
    , sweepSlots = 500
    , sweepOutDir = "/tmp/fixture-sweep"
    , sweepVariants =
        [ SweepVariant "a" "test/fixtures/sim-config.json"
        , SweepVariant "b" "test/fixtures/sim-config.json"
        ]
    }

{- The live configs are not content-asserted — only that they still parse,
including every variant config the example sweep manifest references.
-}
assertLiveConfigsParse :: IO ()
assertLiveConfigsParse = do
  _ <- parseDesign "config/default-design.json"
  _ <- parseSimConfig "config/default-sim-config.json"
  sweepSpec <- loadSweepSpec "config/sweeps/example.json"
  mapM_ (parseSimConfig . (.variantConfig)) sweepSpec.sweepVariants

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
