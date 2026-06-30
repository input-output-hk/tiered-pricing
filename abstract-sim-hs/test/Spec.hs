module Main (main) where

import Actor (Actor (..), ActorId (..), ActorType (..), LaneLatencyEstimate (..), SubmissionEnv (..), resubmitTransaction)
import Block (BlockSummary (..), BlockUsage (..), EbId (..), InclusionPoint (..))
import Config (SimConfig (..))
import Control.Monad (unless)
import Curve (curvesDefault)
import Data.Aeson (eitherDecode)
import Data.ByteString.Lazy qualified as BL
import Data.List.NonEmpty (NonEmpty (..))
import Data.Sequence qualified as Seq
import Design (ControllerConfig (..), ControllerSignal (..), Design (..), Eip1559Controller (..), FeeSemantics (..), LaneStructure (..), PriorityPremiumScope (..), defaultDesign)
import Load (BurstEffect (..), arrivalRateAt, severeCongestionLoad, tryBurstEffectAt)
import LoadProfile (LoadProfile (..), loadLoadProfile)
import Metrics.Accumulator (MetricsAcc (..), emptyMetricsAcc)
import Metrics.Price (PriceOscillation (..), PriceStability (..), priceOscillationFrom, priceStabilityFrom)
import Parser (parseDesign, parseSimConfig)
import Pricing (ControllerInput (..), PriceUpdate (..), Prices (..), admissionRequiredFee, coversProducerHeadroom, initialPrices, quotedFee, realisedFee, retentionWindow, updatePrices)
import Resource (Bytes (..), ExUnits (..), Resources (..))
import Retry (noRetries)
import Sweep (SweepOverrides (..), SweepSpec (..), SweepVariant (..), applyOverrides, loadSweepSpec, parseSweepArgs)
import System.Exit (exitFailure)
import Transaction (Demand (..), Lane (..), Provenance (..), Script (..), Tx (..), TxBody (..), hash)
import Types (Duration (..), Lovelace (..), PerLane (..), SlotNo (..), Urgency (..), atLane)

main :: IO ()
main = do
  assertDesignFixture
  assertSimConfigFixture
  assertSweepFixture
  assertSweepLoadProfileOverride
  assertLiveConfigsParse
  assertLoadProfiles
  assertHeadroomInvariant
  assertPriorityControllerReadsCurrentProduction
  assertPriorityReservationWindowUsesRbEquivalentCapacity
  assertPriorityReservationWindowRetention
  assertCapacityWeightedWindowCountsCertifiedEbs
  assertCapacityWeightedWindowUsesExUnits
  assertPremiumScopeChargesByInclusionPoint
  assertPriceStabilityExcludesTransient
  assertNeverSettlingPriceNeverConverges
  assertEmptyPriceTraceHasNoStability
  assertEmptyPriceTraceHasNoOscillation
  assertMonotonePriceHasNoOscillation
  assertBurstRecoveryIsNotCompletedOscillation
  assertRepeatedReversalCountsAsOscillation
  assertOscillationIgnoresDeadbandMoves
  assertOscillationAggregatesAcrossLanes
  assertSingleLaneActorHasReservationPrice

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
    , sweepLoadProfilePath = Nothing
    , sweepVariants =
        [ SweepVariant "a" "test/fixtures/sim-config.json"
        , SweepVariant "b" "test/fixtures/sim-config.json"
        ]
    }

assertSweepLoadProfileOverride :: IO ()
assertSweepLoadProfileOverride = do
  let profilePath = "config/loads/eb-capacity-stress.json"
      overrides = SweepOverrides Nothing Nothing Nothing (Just profilePath)
  assertEqual
    "load profile command-line argument"
    (Right ("config/sweeps/mechanisms.json", overrides))
    (parseSweepArgs ["config/sweeps/mechanisms.json", "--load-profile", profilePath])
  assertEqual
    "load profile applied to sweep spec"
    (Just profilePath)
    (applyOverrides overrides expectedFixtureSweep).sweepLoadProfilePath

{- The live configs are not generally content-asserted — only that they still
parse, that the mechanism set remains complete, and that selecting a workload
at run time has not mutated their embedded historical load.
-}
assertLiveConfigsParse :: IO ()
assertLiveConfigsParse = do
  _ <- parseDesign "config/default-design.json"
  defaultConfig <- parseSimConfig "config/default-sim-config.json"
  assertEqual "default config retains its original load" severeCongestionLoad defaultConfig.simConfigLoad
  sweepSpec <- loadSweepSpec "config/sweeps/example.json"
  mapM_ (parseSimConfig . (.variantConfig)) sweepSpec.sweepVariants
  mechanisms <- loadSweepSpec "config/sweeps/mechanisms.json"
  assertEqual
    "mechanism sweep covers controls, phase-2 candidates, and windowed-priority companions"
    [ "flat-fee"
    , "single-lane-eip1559"
    , "priority-only-reserved"
    , "priority-only-open"
    , "both-dynamic-reserved"
    , "both-dynamic-open"
    , "priority-only-reserved-window3"
    , "priority-only-open-window3"
    , "both-dynamic-reserved-window3"
    , "both-dynamic-open-window3"
    , "priority-only-reserved-windowed"
    , "priority-only-open-windowed"
    , "both-dynamic-reserved-windowed"
    , "both-dynamic-open-windowed"
    , "priority-only-reserved-window10"
    , "priority-only-open-window10"
    , "both-dynamic-reserved-window10"
    , "both-dynamic-open-window10"
    , "priority-only-reserved-window20"
    , "priority-only-open-window20"
    , "both-dynamic-reserved-window20"
    , "both-dynamic-open-window20"
    ]
    (fmap (.variantName) mechanisms.sweepVariants)
  mechanismConfigs <- traverse (parseSimConfig . (.variantConfig)) mechanisms.sweepVariants
  assertTrue
    "mechanism configs retain their original load"
    (all ((== severeCongestionLoad) . (.simConfigLoad)) mechanismConfigs)

assertLoadProfiles :: IO ()
assertLoadProfiles = do
  severe <- loadLoadProfile "config/loads/severe-congestion.json"
  assertEqual "severe-congestion profile name" "severe-congestion" severe.loadProfileName
  assertEqual
    "standalone severe-congestion profile matches the embedded preset"
    severeCongestionLoad
    severe.loadProfileProcess
  profile <- loadLoadProfile "config/loads/eb-capacity-stress.json"
  assertEqual "EB capacity stress profile name" "eb-capacity-stress" profile.loadProfileName
  let process = profile.loadProfileProcess
  assertEqual
    "EB capacity stress phase rates"
    [40, 40, 320, 320, 20, 20, 400, 400, 20, 20, 320, 320, 20, 20, 400, 400, 40, 40]
    (arrivalRateAt process . SlotNo <$> boundarySamples)
  assertEqual
    "EB capacity stress rate after the 2,000-slot experiment"
    0
    (arrivalRateAt process (SlotNo 2_000))
  assertEqual
    "EB capacity stress phases do not alter value or urgency mix"
    (Just (BurstEffect 1 1))
    (tryBurstEffectAt process (SlotNo 650))
 where
  boundarySamples =
    [ 0
    , 199
    , 200
    , 449
    , 450
    , 649
    , 650
    , 899
    , 900
    , 1_099
    , 1_100
    , 1_349
    , 1_350
    , 1_549
    , 1_550
    , 1_799
    , 1_800
    , 1_999
    ]

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

{- | The windowless priority signal must read this update's own block
production, not the retained history. Under a static-standard (priority-only)
design the retention window holds a single summary, so it can end at a
sample-less EB announcement; the controller must still see the certified
EB's partition fill from the current production rather than a phantom
mean [] = 0.
-}
assertPriorityControllerReadsCurrentProduction :: IO ()
assertPriorityControllerReadsCurrentProduction = do
  let reservation = Resources{resBytes = Bytes 90_112, resExUnits = ExUnits 96_991_334}
      ebCapacity = Resources{resBytes = Bytes 12_000_000, resExUnits = ExUnits 9_499_133_448}
      fullPriorityPartition =
        BlockUsage
          { usageCapacity = ebCapacity
          , usageUsed = Resources{resBytes = Bytes 90_112, resExUnits = ExUnits 0}
          , usageLanes =
              PerLane
                { perStandard = mempty
                , perPriority = Resources{resBytes = Bytes 90_112, resExUnits = ExUnits 0}
                }
          , usageSignalCapacity = reservation
          }
      emptyUsage =
        BlockUsage
          { usageCapacity = ebCapacity
          , usageUsed = mempty
          , usageLanes = pure mempty
          , usageSignalCapacity = reservation
          }
      priorityOnly =
        ControllerConfig
          { laneControllers =
              PerLane
                { perStandard = Nothing
                , perPriority =
                    Just
                      Eip1559Controller
                        { controllerTargetUtilisation = 0.50
                        , controllerMaxChangeDenominator = 8
                        , controllerInitialCoefficient = 16.0
                        , controllerSignal = PriorityReservationUtil
                        }
                }
          , multiplierFloor = Nothing
          , absoluteCoeffFloor = 1.0
          }
      input =
        ControllerInput
          { recentBlocks = Seq.fromList [EbAnnounced (EbId 8) emptyUsage]
          , currentProduction =
              Seq.fromList
                [ RbCertifying (EbId 7)
                , EbCertified (EbId 7) fullPriorityPartition
                , EbAnnounced (EbId 8) emptyUsage
                ]
          }
      (newPrices, updates) = updatePrices priorityOnly input (Prices (PerLane 1.0 16.0))
  case updates of
    [update] -> do
      assertEqual "priority update lane" Priority update.priceUpdateLane
      assertEqual "priority update utilisation" 1.0 update.priceUpdateUtilisation
      assertEqual "priority update old coeff" 16.0 update.priceUpdateOldCoeff
      assertEqual "priority update new coeff" 18.0 update.priceUpdateNewCoeff
      assertEqual "priority coeff after update" 18.0 (atLane Priority newPrices.laneCoeffs)
    _ -> do
      putStrLn ("expected exactly one priority update, got " <> show (length updates))
      exitFailure

assertPriorityReservationWindowUsesRbEquivalentCapacity :: IO ()
assertPriorityReservationWindowUsesRbEquivalentCapacity = do
  let reservation = Resources{resBytes = Bytes 100, resExUnits = ExUnits 1_000}
      ebCapacity = Resources{resBytes = Bytes 1_000, resExUnits = ExUnits 10_000}
      rbCapacity = reservation
      usage capacity priorityBytes priorityExUnits =
        BlockUsage
          { usageCapacity = capacity
          , usageUsed = Resources{resBytes = Bytes priorityBytes, resExUnits = ExUnits priorityExUnits}
          , usageLanes =
              PerLane
                { perStandard = mempty
                , perPriority = Resources{resBytes = Bytes priorityBytes, resExUnits = ExUnits priorityExUnits}
                }
          , usageSignalCapacity = reservation
          }
      priorityWindow =
        ControllerConfig
          { laneControllers =
              PerLane
                { perStandard = Nothing
                , perPriority =
                    Just
                      Eip1559Controller
                        { controllerTargetUtilisation = 0.50
                        , controllerMaxChangeDenominator = 8
                        , controllerInitialCoefficient = 2.0
                        , controllerSignal = PriorityReservationWindow 2
                        }
                }
          , multiplierFloor = Nothing
          , absoluteCoeffFloor = 1.0
          }
      input =
        ControllerInput
          { recentBlocks =
              Seq.fromList
                [ RbPraos [] (usage rbCapacity 100 1_000)
                , EbAnnounced (EbId 1) (usage ebCapacity 100 1_000)
                , RbPraos [] (usage rbCapacity 25 0)
                , EbCertified (EbId 2) (usage ebCapacity 200 4_000)
                ]
          , currentProduction = mempty
          }
      (newPrices, updates) = updatePrices priorityWindow input (Prices (PerLane 1.0 8.0))
  case updates of
    [update] -> do
      assertEqual "priority-window update lane" Priority update.priceUpdateLane
      assertEqual "priority-window aggregate utilisation" 0.625 update.priceUpdateUtilisation
      assertEqual "priority-window new coeff" 8.25 update.priceUpdateNewCoeff
      assertEqual "priority coeff after window update" 8.25 (atLane Priority newPrices.laneCoeffs)
    _ -> do
      putStrLn ("expected exactly one priority update, got " <> show (length updates))
      exitFailure

assertPriorityReservationWindowRetention :: IO ()
assertPriorityReservationWindowRetention = do
  let priorityWindow =
        ControllerConfig
          { laneControllers =
              PerLane
                { perStandard = Nothing
                , perPriority =
                    Just
                      Eip1559Controller
                        { controllerTargetUtilisation = 0.50
                        , controllerMaxChangeDenominator = 8
                        , controllerInitialCoefficient = 2.0
                        , controllerSignal = PriorityReservationWindow 20
                        }
                }
          , multiplierFloor = Nothing
          , absoluteCoeffFloor = 1.0
          }
  assertEqual "priority-reservation window retention" 60 (retentionWindow priorityWindow)

assertCapacityWeightedWindowCountsCertifiedEbs :: IO ()
assertCapacityWeightedWindowCountsCertifiedEbs = do
  let ebCapacity = Resources{resBytes = Bytes 100, resExUnits = ExUnits 1_000}
      announcedFullStandard =
        BlockUsage
          { usageCapacity = ebCapacity
          , usageUsed = Resources{resBytes = Bytes 100, resExUnits = ExUnits 0}
          , usageLanes =
              PerLane
                { perStandard = Resources{resBytes = Bytes 100, resExUnits = ExUnits 0}
                , perPriority = mempty
                }
          , usageSignalCapacity = mempty
          }
      certifiedHalfStandard =
        BlockUsage
          { usageCapacity = ebCapacity
          , usageUsed = Resources{resBytes = Bytes 50, resExUnits = ExUnits 0}
          , usageLanes =
              PerLane
                { perStandard = Resources{resBytes = Bytes 50, resExUnits = ExUnits 0}
                , perPriority = mempty
                }
          , usageSignalCapacity = mempty
          }
      standardOnly =
        ControllerConfig
          { laneControllers =
              PerLane
                { perStandard =
                    Just
                      Eip1559Controller
                        { controllerTargetUtilisation = 0.50
                        , controllerMaxChangeDenominator = 8
                        , controllerInitialCoefficient = 1.0
                        , controllerSignal = CapacityWeightedWindow 10
                        }
                , perPriority = Nothing
                }
          , multiplierFloor = Nothing
          , absoluteCoeffFloor = 1.0
          }
      input =
        ControllerInput
          { recentBlocks =
              Seq.fromList
                [ EbAnnounced (EbId 1) announcedFullStandard
                , EbCertified (EbId 1) certifiedHalfStandard
                ]
          , currentProduction = mempty
          }
      (newPrices, updates) = updatePrices standardOnly input (Prices (PerLane 8.0 1.0))
  case updates of
    [update] -> do
      assertEqual "capacity-window update lane" Standard update.priceUpdateLane
      assertEqual "capacity-window certified EB utilisation" 0.5 update.priceUpdateUtilisation
      assertEqual "capacity-window ignores EB announcement" 8.0 update.priceUpdateNewCoeff
      assertEqual "standard coeff after capacity-window update" 8.0 (atLane Standard newPrices.laneCoeffs)
    _ -> do
      putStrLn ("expected exactly one standard update, got " <> show (length updates))
      exitFailure

assertCapacityWeightedWindowUsesExUnits :: IO ()
assertCapacityWeightedWindowUsesExUnits = do
  let ebCapacity = Resources{resBytes = Bytes 100, resExUnits = ExUnits 1_000}
      certifiedExUnitBound =
        BlockUsage
          { usageCapacity = ebCapacity
          , usageUsed = Resources{resBytes = Bytes 10, resExUnits = ExUnits 1_000}
          , usageLanes =
              PerLane
                { perStandard = Resources{resBytes = Bytes 10, resExUnits = ExUnits 1_000}
                , perPriority = mempty
                }
          , usageSignalCapacity = mempty
          }
      standardOnly =
        ControllerConfig
          { laneControllers =
              PerLane
                { perStandard =
                    Just
                      Eip1559Controller
                        { controllerTargetUtilisation = 0.50
                        , controllerMaxChangeDenominator = 8
                        , controllerInitialCoefficient = 1.0
                        , controllerSignal = CapacityWeightedWindow 10
                        }
                , perPriority = Nothing
                }
          , multiplierFloor = Nothing
          , absoluteCoeffFloor = 1.0
          }
      input =
        ControllerInput
          { recentBlocks = Seq.fromList [EbCertified (EbId 1) certifiedExUnitBound]
          , currentProduction = mempty
          }
      (newPrices, updates) = updatePrices standardOnly input (Prices (PerLane 8.0 1.0))
  case updates of
    [update] -> do
      assertEqual "capacity-window update lane" Standard update.priceUpdateLane
      assertEqual "capacity-window ex-unit utilisation" 1.0 update.priceUpdateUtilisation
      assertEqual "capacity-window ex-units can bind" 9.0 update.priceUpdateNewCoeff
      assertEqual "standard coeff after ex-unit-bound update" 9.0 (atLane Standard newPrices.laneCoeffs)
    _ -> do
      putStrLn ("expected exactly one standard update, got " <> show (length updates))
      exitFailure

{- | The Giorgos design ('PremiumRbOnly'): the priority premium buys RB
inclusion specifically, so a priority tx landing in an EB is refunded down
to the standard quote. 'PremiumEverywhere' charges the posted lane's quote
regardless of inclusion point. Only the refund target moves — mempool
validity stays posted-lane in both scopes, and non-refunding semantics
('FixedFee') are unaffected.
-}
assertPremiumScopeChargesByInclusionPoint :: IO ()
assertPremiumScopeChargesByInclusionPoint = do
  let prices = initialPrices defaultDesign
      posted = Lovelace 10_000_000
      priorityTx = withFee posted (testTx Priority)
      priorityQuote = quotedFee prices (testTx Priority)
      standardQuote = quotedFee prices (testTx Standard)
  assertTrue
    "fixture quotes are distinct (else this test asserts nothing)"
    (standardQuote < priorityQuote)
  assertEqual
    "everywhere: EB inclusion charges the posted lane's quote"
    priorityQuote
    (realisedFee PremiumEverywhere Eip1559 prices (IncludedInEb (EbId 0)) priorityTx)
  assertEqual
    "rb-only: EB inclusion refunds to the standard quote"
    standardQuote
    (realisedFee PremiumRbOnly Eip1559 prices (IncludedInEb (EbId 0)) priorityTx)
  assertEqual
    "rb-only: RB inclusion still charges the priority quote"
    priorityQuote
    (realisedFee PremiumRbOnly Eip1559 prices IncludedInRb priorityTx)
  assertEqual
    "rb-only: standard txs are unaffected"
    standardQuote
    (realisedFee PremiumRbOnly Eip1559 prices (IncludedInEb (EbId 0)) (withFee posted (testTx Standard)))
  assertEqual
    "rb-only: fixed-fee semantics still charge the posted fee in full"
    posted
    (realisedFee PremiumRbOnly FixedFee prices (IncludedInEb (EbId 0)) priorityTx)

{- Price stability is judged against the final (steady-state) coefficient:
the transient ramp must not count towards settled coefficient range, and a lane
whose price is out of band right up to its last update never converged. -}
assertPriceStabilityExcludesTransient :: IO ()
assertPriceStabilityExcludesTransient = do
  let stability =
        priceStabilityFrom 0.05 $
          priceChangesAcc
            [ (10, Standard, 1.0, 2.0)
            , (20, Standard, 2.0, 4.0)
            , (30, Standard, 4.0, 8.0)
            , (40, Standard, 8.0, 8.25)
            , (50, Standard, 8.25, 8.0)
            , (60, Standard, 8.0, 8.25)
            ]
  assertEqual
    "ramp-then-settle converges where the band is entered for good"
    (Just (Duration 30))
    stability.convergenceTime
  assertEqual
    "settled coefficient range covers the settled tail only"
    0.25
    stability.settledCoefficientRange

assertNeverSettlingPriceNeverConverges :: IO ()
assertNeverSettlingPriceNeverConverges = do
  let stability =
        priceStabilityFrom 0.05 $
          priceChangesAcc
            [ (10, Standard, 1.0, 2.0)
            , (20, Standard, 2.0, 4.0)
            , (30, Standard, 4.0, 8.0)
            , (40, Standard, 8.0, 8.25)
            , (10, Priority, 1.0, 9.0)
            , (20, Priority, 9.0, 1.0)
            , (30, Priority, 1.0, 9.0)
            , (40, Priority, 9.0, 1.0)
            , (50, Priority, 1.0, 9.0)
            ]
  assertEqual
    "one oscillating lane forces overall non-convergence"
    Nothing
    stability.convergenceTime
  assertEqual
    "a never-settling lane reports its full swing"
    8.0
    stability.settledCoefficientRange

assertEmptyPriceTraceHasNoStability :: IO ()
assertEmptyPriceTraceHasNoStability = do
  let stability = priceStabilityFrom 0.05 emptyMetricsAcc
  assertEqual "no price changes: no convergence" Nothing stability.convergenceTime
  assertEqual "no price changes: no settled coefficient range" 0.0 stability.settledCoefficientRange

assertEmptyPriceTraceHasNoOscillation :: IO ()
assertEmptyPriceTraceHasNoOscillation = do
  assertEqual
    "no price changes: no oscillation"
    (PriceOscillation 0 0 0 0)
    (priceOscillationFrom 0.05 emptyMetricsAcc)

assertMonotonePriceHasNoOscillation :: IO ()
assertMonotonePriceHasNoOscillation = do
  assertEqual
    "monotone ramp has no oscillation"
    (PriceOscillation 0 0 0 0)
    ( priceOscillationFrom 0.05 $
        priceChangesAcc
          [ (10, Standard, 1.0, 2.0)
          , (20, Standard, 2.0, 4.0)
          , (30, Standard, 4.0, 8.0)
          ]
    )

assertBurstRecoveryIsNotCompletedOscillation :: IO ()
assertBurstRecoveryIsNotCompletedOscillation = do
  let oscillation =
        priceOscillationFrom 0.05 $
          priceChangesAcc
            [ (10, Standard, 1.0, 2.0)
            , (20, Standard, 2.0, 1.0)
            ]
  assertEqual "burst/recovery reversal count" 1 oscillation.oscillationReversalCount
  assertEqual "burst/recovery has no completed cycle" 0 oscillation.oscillationCycleCount
  assertEqual "burst/recovery amplitude" 1.0 oscillation.maxOscillationAmplitude
  assertClose "burst/recovery excess travel" (2 * log 2) oscillation.oscillationExcessTravel

assertRepeatedReversalCountsAsOscillation :: IO ()
assertRepeatedReversalCountsAsOscillation = do
  let oscillation =
        priceOscillationFrom 0.05 $
          priceChangesAcc
            [ (10, Standard, 1.0, 2.0)
            , (20, Standard, 2.0, 1.0)
            , (30, Standard, 1.0, 2.0)
            ]
  assertEqual "up/down/up reversal count" 2 oscillation.oscillationReversalCount
  assertEqual "up/down/up completed cycle" 1 oscillation.oscillationCycleCount
  assertEqual "up/down/up amplitude" 1.0 oscillation.maxOscillationAmplitude
  assertClose "up/down/up excess travel" (2 * log 2) oscillation.oscillationExcessTravel

assertOscillationIgnoresDeadbandMoves :: IO ()
assertOscillationIgnoresDeadbandMoves = do
  assertEqual
    "deadband moves are ignored"
    (PriceOscillation 0 0 0 0)
    ( priceOscillationFrom 0.05 $
        priceChangesAcc
          [ (10, Standard, 1.0, 1.04)
          , (20, Standard, 1.04, 1.0)
          , (30, Standard, 1.0, 2.0)
          ]
    )

assertOscillationAggregatesAcrossLanes :: IO ()
assertOscillationAggregatesAcrossLanes = do
  let oscillation =
        priceOscillationFrom 0.05 $
          priceChangesAcc
            [ (10, Standard, 1.0, 2.0)
            , (20, Standard, 2.0, 1.0)
            , (30, Standard, 1.0, 2.0)
            , (10, Priority, 4.0, 2.0)
            , (20, Priority, 2.0, 4.0)
            , (30, Priority, 4.0, 2.0)
            ]
  assertEqual "multi-lane reversal count" 4 oscillation.oscillationReversalCount
  assertEqual "multi-lane cycle count" 2 oscillation.oscillationCycleCount
  assertEqual "multi-lane max amplitude" 2.0 oscillation.maxOscillationAmplitude

-- | The accumulator stores price changes newest-first.
priceChangesAcc :: [(Int, Lane, Double, Double)] -> MetricsAcc
priceChangesAcc changes =
  emptyMetricsAcc
    { accPriceChanges =
        [ ( SlotNo slot
          , PriceUpdate
              { priceUpdateLane = lane
              , priceUpdateOldCoeff = oldCoeff
              , priceUpdateNewCoeff = newCoeff
              , priceUpdateUtilisation = 0.5
              }
          )
        | (slot, lane, oldCoeff, newCoeff) <- reverse changes
        ]
    }

{- | A single-lane actor has a value-based reservation price: when the fee
exceeds what its transaction is worth, it declines to submit, exactly as the
two-lane lane choice does ('Actor.chooseLane'). Without this, single-lane
demand is price-inelastic and the EIP-1559 controller has nothing to push
against under congestion, so the base fee compounds without bound.
-}
assertSingleLaneActorHasReservationPrice :: IO ()
assertSingleLaneActorHasReservationPrice = do
  assertEqual
    "single-lane actor declines when the fee exceeds the tx value"
    Nothing
    (submittedLane (singleLaneEnvAt (Prices (PerLane 100.0 100.0))))
  assertEqual
    "single-lane actor submits when the value covers the fee"
    (Just Standard)
    (submittedLane (singleLaneEnvAt (Prices (PerLane 1.0 1.0))))
 where
  submittedLane env =
    fmap (.txLane) (resubmitTransaction env FreshDemand 1 1.2 (fixtureActor 0) reservationDemand)
  singleLaneEnvAt prices =
    SubmissionEnv
      { envLaneStructure = One
      , envF = 0.05
      , envSlot = SlotNo 0
      , envPrices = prices
      , envLatency =
          LaneLatencyEstimate
            { expectedStandardLatency = Duration 50
            , expectedPriorityLatency = Duration 25
            }
      }
  reservationDemand =
    Demand
      { demandValue = Lovelace 1_000_000
      , demandUrgency = Exponential 0.04
      , demandSize = 500
      , demandScript = Script{_scriptSize = 0, _scriptExUnits = 0}
      }

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

assertClose :: String -> Double -> Double -> IO ()
assertClose label expected actual =
  unless (abs (actual - expected) < 1.0e-9) do
    putStrLn ("unexpected " <> label)
    putStrLn ("expected: " <> show expected)
    putStrLn ("actual:   " <> show actual)
    exitFailure
