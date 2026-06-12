module Main (main) where

import Actor (Actor (..), ActorId (..), ActorType (..), LaneLatencyEstimate (..))
import Config (SimConfig (..))
import Control.Monad (unless)
import Curve (curvesDefault)
import Data.Aeson (eitherDecode)
import Data.ByteString.Lazy qualified as BL
import Data.List.NonEmpty (NonEmpty (..))
import Design (Design (..), FeeSemantics (..), defaultDesign)
import Load (severeCongestionLoad)
import Parser (parseDesign, parseSimConfig)
import Pricing (admissionRequiredFee, coversProducerHeadroom, initialPrices)
import Retry (noRetries)
import Sweep (SweepSpec (..), SweepVariant (..), loadSweepSpec)
import System.Exit (exitFailure)
import Transaction (Demand (..), Lane (..), Provenance (..), Script (..), Tx (..), TxBody (..), hash)
import Types (Duration (..), Lovelace (..), SlotNo (..), Urgency (..))

main :: IO ()
main = do
  assertDesignFixture
  assertSimConfigFixture
  assertSweepFixture
  assertLiveConfigsParse
  assertHeadroomInvariant

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
      assertEqual "design fixture decode" defaultDesign actual
  actualDesign <- parseDesign "test/fixtures/design.json"
  assertEqual "design fixture file parser" defaultDesign actualDesign

assertSimConfigFixture :: IO ()
assertSimConfigFixture = do
  actualConfig <- parseSimConfig "test/fixtures/sim-config.json"
  assertEqual "sim config fixture file parser" expectedFixtureSimConfig actualConfig

expectedFixtureSimConfig :: SimConfig
expectedFixtureSimConfig =
  SimConfig
    { simConfigDesign = defaultDesign
    , simConfigCurves = curvesDefault
    , simConfigF = 0.05
    , simConfigD = 13
    , simConfigLoad = severeCongestionLoad
    , simConfigActors = fixtureActor 0 :| [fixtureActor 1]
    , simConfigRbTxBytesCap = 90_112
    , simConfigRbExUnitsCap = 96_991_334
    , simConfigEbTxBytesCap = 12_000_000
    , simConfigEbStructureBytesCap = 512_000
    , simConfigEbExUnitsCap = 9_499_133_448
    , simConfigMempoolBytesCap = 24_000_000
    , simConfigAdmissionHeadroomUpdates = 1
    , simConfigLaneLatencyEstimate =
        LaneLatencyEstimate
          { expectedStandardLatency = Duration 50
          , expectedPriorityLatency = Duration 25
          }
    , simConfigPriceConvergenceBandPct = 0.05
    , simConfigLoadChangePct = 0.10
    , simConfigRetryPolicy = noRetries
    }

fixtureActor :: Int -> Actor
fixtureActor i =
  Actor
    { _actorId = ActorId i
    , actorType = Honest
    , actorFeeBuffer = 2.0
    , actorMinValueFeeMultiple = 1.0
    , actorValueMultiplier = 1.0
    , actorUrgencyMultiplier = 1.0
    }

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
    , txDemand =
        Demand
          { demandValue = Lovelace 1_000_000
          , demandUrgency = Exponential 0.04
          , demandSize = 500
          , demandScript = script
          }
    , txLane = lane
    , txProvenance = FreshDemand
    }
 where
  script = Script{_scriptSize = 0, _scriptExUnits = 0}
  body =
    TxBody
      { _txSize = 500
      , _txScript = script
      , _txDependsOn = mempty
      , _txFee = Lovelace 0
      , _txNumber = 1
      }

assertEqual :: (Eq a, Show a) => String -> a -> a -> IO ()
assertEqual label expected actual =
  unless (actual == expected) do
    putStrLn ("unexpected " <> label)
    putStrLn ("expected: " <> show expected)
    putStrLn ("actual:   " <> show actual)
    exitFailure

assertTrue :: String -> Bool -> IO ()
assertTrue label cond =
  unless cond do
    putStrLn ("assertion failed: " <> label)
    exitFailure
