{- | The experiment sweep: candidate designs × seeds, per
@abstract-experiment-design.md@. A sweep is defined by a committed manifest
file, so the experiment — which designs, how many seeds, how many slots — is
a reviewable, versionable artifact rather than a shell invocation:

@
{ "description": "Candidate designs vs controls under severe congestion",
  "seeds": 30,
  "slots": 2000,
  "out": "sweep-results\/2026-06-12-controls",
  "variants": [
    { "name": "two-lane-reserved", "config": "config\/default-sim-config.json" },
    { "name": "two-lane-open", "config": "config\/variants\/no-reservation.json" } ] }
@

By default, every (variant, seed) pair is one full traced run. With
@--summary-only@, the same events are folded into the same metrics but are not
serialised, avoiding large JSONL files. The output directory is
self-contained for reproduction: the resolved spec is embedded in
@summary.json@, each variant's effective config is copied alongside it, and
an optional @--load-profile@ file is copied as
@selected-load-profile.json@. Preset and file overrides are written into
those effective variant configs. @f@ and @D@ are not sweep axes: they stay
fixed at the justified values in each config.
-}
module Sweep (
  LoadOverride (..),
  SweepSpec (..),
  SweepVariant (..),
  SweepOverrides (..),
  applyOverrides,
  loadSweepSpec,
  parseSweepArgs,
  runSweep,
) where

import Config (SimConfig)
import Control.Applicative ((<|>))
import Control.Exception (evaluate)
import Data.Aeson (FromJSON (..), Value (Object, String), eitherDecode, encode, object, withObject, (.:), (.:?), (.=))
import Data.Aeson.Key qualified as Key
import Data.Aeson.KeyMap qualified as KeyMap
import Data.ByteString.Lazy qualified as BL
import Data.List (maximumBy, nub)
import Data.Map.Strict qualified as Map
import Data.Maybe (fromMaybe)
import Data.Ord (comparing)
import Data.Text qualified as T
import LoadProfile (LoadProfile (..), loadLoadProfile)
import Metrics (
  DemandLoad (..),
  DistStats (..),
  InclusionStats (..),
  Metrics (..),
  PriceOscillation (..),
  PriceShock (..),
  PriceStability (..),
  Revenue (..),
  Throughput (..),
  ValueOutcome (..),
  ratio,
  sumLovelace,
  weightedMean,
 )
import Parser (parseSimConfig)
import Run (Run (..), Seed, runWithSeed, runWithSeedToFile)
import System.Directory (copyFile, createDirectoryIfMissing)
import System.FilePath ((</>))
import Text.Printf (printf)
import Types (Duration (..), Lane (..), Lovelace (..), Urgency (..))

data LoadOverride
  = LoadPreset String
  | LoadProfileFile FilePath
  deriving (Eq, Show)

data SweepSpec = SweepSpec
  { sweepDescription :: Maybe String
  , sweepSeeds :: Int
  , sweepSlots :: Int
  , sweepOutDir :: FilePath
  , sweepLoadOverride :: Maybe LoadOverride
  -- ^ Forces every variant onto either a named preset or a file-backed load
  -- profile. Manifest @"load"@ values select presets; command-line flags can
  -- select either form.
  , sweepSummaryOnly :: Bool
  -- ^ Fold all events into metrics without writing per-run JSONL traces.
  , sweepVariants :: [SweepVariant]
  }
  deriving (Eq, Show)

data SweepVariant = SweepVariant
  { variantName :: String
  , variantConfig :: FilePath
  }
  deriving (Eq, Show)

type RunScalars = [(String, Double)]

-- | Command-line overrides on top of the manifest, for quick iteration
-- without editing the committed experiment definition.
data SweepOverrides = SweepOverrides
  { overrideSeeds :: Maybe Int
  , overrideSlots :: Maybe Int
  , overrideOut :: Maybe FilePath
  , overrideLoad :: Maybe LoadOverride
  , overrideSummaryOnly :: Bool
  }
  deriving (Eq, Show)

parseSweepArgs :: [String] -> Either String (FilePath, SweepOverrides)
parseSweepArgs = go (Nothing, SweepOverrides Nothing Nothing Nothing Nothing False)
 where
  go (manifest, overrides) = \case
    [] ->
      case manifest of
        Nothing -> Left "sweep: a manifest file is required (see config/sweeps/)"
        Just path -> Right (path, overrides)
    "--seeds" : value : rest -> do
      seeds <- readPositive "--seeds" value
      go (manifest, overrides{overrideSeeds = Just seeds}) rest
    "--slots" : value : rest -> do
      slots <- readPositive "--slots" value
      go (manifest, overrides{overrideSlots = Just slots}) rest
    "--out" : dir : rest ->
      go (manifest, overrides{overrideOut = Just dir}) rest
    "--load" : value : rest ->
      go (manifest, overrides{overrideLoad = Just (LoadPreset value)}) rest
    "--load-profile" : path : rest ->
      go (manifest, overrides{overrideLoad = Just (LoadProfileFile path)}) rest
    "--summary-only" : rest ->
      go (manifest, overrides{overrideSummaryOnly = True}) rest
    arg : rest
      | take 2 arg == "--" -> Left ("sweep: unknown flag " <> arg)
      | Nothing <- manifest -> go (Just arg, overrides) rest
      | otherwise ->
          Left "sweep: takes exactly one manifest file; variant configs are listed inside it"

  readPositive flag value =
    case reads value of
      [(n, "")] | n >= 1 -> Right n
      _ -> Left ("sweep: " <> flag <> " needs a positive integer, got " <> show value)

applyOverrides :: SweepOverrides -> SweepSpec -> SweepSpec
applyOverrides overrides spec =
  spec
    { sweepSeeds = fromMaybe spec.sweepSeeds overrides.overrideSeeds
    , sweepSlots = fromMaybe spec.sweepSlots overrides.overrideSlots
    , sweepOutDir = fromMaybe spec.sweepOutDir overrides.overrideOut
    , sweepLoadOverride = overrides.overrideLoad <|> spec.sweepLoadOverride
    , sweepSummaryOnly = overrides.overrideSummaryOnly || spec.sweepSummaryOnly
    }

data ParseSweepSpec = ParseSweepSpec
  { parseSweepDescription :: Maybe String
  , parseSweepSeeds :: Maybe Int
  , parseSweepSlots :: Maybe Int
  , parseSweepOut :: Maybe FilePath
  , parseSweepLoad :: Maybe String
  , parseSweepSummaryOnly :: Maybe Bool
  , parseSweepVariants :: [ParseSweepVariant]
  }

instance FromJSON ParseSweepSpec where
  parseJSON =
    withObject "ParseSweepSpec" \obj ->
      ParseSweepSpec
        <$> obj .:? "description"
        <*> obj .:? "seeds"
        <*> obj .:? "slots"
        <*> obj .:? "out"
        <*> obj .:? "load"
        <*> obj .:? "summaryOnly"
        <*> obj .: "variants"

data ParseSweepVariant = ParseSweepVariant
  { parseVariantName :: String
  , parseVariantConfig :: FilePath
  }

instance FromJSON ParseSweepVariant where
  parseJSON =
    withObject "ParseSweepVariant" \obj ->
      ParseSweepVariant
        <$> obj .: "name"
        <*> obj .: "config"

loadSweepSpec :: FilePath -> IO SweepSpec
loadSweepSpec path = do
  bytes <- BL.readFile path
  case eitherDecode bytes of
    Left err ->
      fail ("cannot parse sweep manifest " <> path <> ": " <> err)
    Right parsed ->
      case fromParseSweepSpec parsed of
        Left err -> fail ("invalid sweep manifest " <> path <> ": " <> err)
        Right spec -> pure spec

fromParseSweepSpec :: ParseSweepSpec -> Either String SweepSpec
fromParseSweepSpec parsed = do
  variants <- traverse toVariant parsed.parseSweepVariants
  validate variants
  pure
    SweepSpec
      { sweepDescription = parsed.parseSweepDescription
      , sweepSeeds = fromMaybe 10 parsed.parseSweepSeeds
      , sweepSlots = fromMaybe 2_000 parsed.parseSweepSlots
      , sweepOutDir = fromMaybe "sweep-results" parsed.parseSweepOut
      , sweepLoadOverride = LoadPreset <$> parsed.parseSweepLoad
      , sweepSummaryOnly = fromMaybe False parsed.parseSweepSummaryOnly
      , sweepVariants = variants
      }
 where
  toVariant variant
    | null variant.parseVariantName = Left "variant names must be non-empty"
    | otherwise =
        Right
          SweepVariant
            { variantName = variant.parseVariantName
            , variantConfig = variant.parseVariantConfig
            }
  validate variants
    | null variants = Left "at least one variant is required"
    | names /= nub names = Left "variant names must be unique"
    | any (< 1) (fromMaybe 1 <$> [parsed.parseSweepSeeds, parsed.parseSweepSlots]) =
        Left "seeds and slots must be positive"
    | otherwise = Right ()
   where
    names = fmap (.variantName) variants

runSweep :: SweepSpec -> IO ()
runSweep spec = do
  resolvedLoad <- traverse resolveLoadOverride spec.sweepLoadOverride
  createDirectoryIfMissing True spec.sweepOutDir
  copySelectedProfile spec resolvedLoad
  variants <- traverse (runVariant spec (loadValue <$> resolvedLoad)) spec.sweepVariants
  let summaryPath = spec.sweepOutDir </> "summary.json"
  BL.writeFile summaryPath (encode (summaryJson spec resolvedLoad variants))
  putStrLn ("wrote " <> summaryPath)

data ResolvedLoad
  = ResolvedPreset String
  | ResolvedProfile FilePath LoadProfile

resolveLoadOverride :: LoadOverride -> IO ResolvedLoad
resolveLoadOverride = \case
  LoadPreset name -> pure (ResolvedPreset name)
  LoadProfileFile path -> ResolvedProfile path <$> loadLoadProfile path

loadValue :: ResolvedLoad -> Value
loadValue = \case
  ResolvedPreset name -> String (T.pack name)
  ResolvedProfile _ profile -> profile.loadProfileValue

copySelectedProfile :: SweepSpec -> Maybe ResolvedLoad -> IO ()
copySelectedProfile _ Nothing = pure ()
copySelectedProfile _ (Just ResolvedPreset{}) = pure ()
copySelectedProfile spec (Just (ResolvedProfile path _)) =
  copyFile path (spec.sweepOutDir </> "selected-load-profile.json")

runVariant :: SweepSpec -> Maybe Value -> SweepVariant -> IO (SweepVariant, [(Seed, RunScalars)])
runVariant spec selectedLoad variant = do
  -- the output directory is self-contained for reproduction: the exact
  -- config of every variant rides along with the traces it produced. When a
  -- load override is in force we write the effective config (with the load
  -- swapped) rather than copying, so the saved config matches what actually ran.
  let savedConfig = spec.sweepOutDir </> (variant.variantName <> ".config.json")
  writeEffectiveConfig selectedLoad variant.variantConfig savedConfig
  config <- parseSimConfig savedConfig
  runs <-
    traverse (runPoint spec variant.variantName config) (fromIntegral <$> [0 .. spec.sweepSeeds - 1])
  pure (variant, runs)

{- | Copy a variant config into the sweep output, optionally replacing its
@"load"@ field. Parsing the written file (rather than the source) keeps the
saved config and the run in lock-step.
-}
writeEffectiveConfig :: Maybe Value -> FilePath -> FilePath -> IO ()
writeEffectiveConfig Nothing src dest = copyFile src dest
writeEffectiveConfig (Just selectedLoad) src dest = do
  bytes <- BL.readFile src
  case eitherDecode bytes of
    Right (Object o) ->
      BL.writeFile dest (encode (Object (KeyMap.insert "load" selectedLoad o)))
    Right _ -> fail ("variant config " <> src <> " is not a JSON object")
    Left err -> fail ("cannot parse variant config " <> src <> ": " <> err)

runPoint :: SweepSpec -> String -> SimConfig -> Seed -> IO (Seed, RunScalars)
runPoint spec name config seed = do
  let tracePath = spec.sweepOutDir </> (name <> "-seed" <> show seed <> ".events.jsonl")
  result <-
    if spec.sweepSummaryOnly
      then runWithSeed config seed spec.sweepSlots
      else runWithSeedToFile config tracePath seed spec.sweepSlots
  let metrics = result._runResult
      scalars = headlineScalars metrics
  forceScalars scalars
  putStrLn (name <> " seed " <> show seed <> ": " <> progressLine metrics)
  pure (seed, scalars)

forceScalars :: RunScalars -> IO ()
forceScalars scalars =
  evaluate (foldl' (\acc (_, value) -> value `seq` acc) () scalars)

progressLine :: Metrics -> String
progressLine metrics =
  printf
    "service %.3f, latency %.1f slots, amplification %.3f"
    (serviceRate metrics)
    (latencyMeanSlots metrics)
    (loadAmplification metrics)

{- | The per-run scalars that get aggregated across seeds: one declaration
binding each scalar's name to its accessor, so producers and consumers cannot
drift apart by key typo. The full 'Metrics' detail (slices, percentiles,
price trace) stays available in each run's events trace and per-run analysis
when tracing is enabled. Summary-only runs intentionally retain just these
comparison scalars.
-}
headline :: [(String, Metrics -> Double)]
headline =
  [ ("units.total", \m -> int m.demandLoad.demandUnits)
  , ("units.served", \m -> int m.demandLoad.unitsServed)
  , ("units.abandoned", \m -> int m.demandLoad.unitsAbandoned)
  , ("units.unresolved", \m -> int m.demandLoad.unitsUnresolved)
  , ("units.serviceRate", serviceRate)
  , ("inclusion.standard.submitted", \m -> int (laneInclusionStats Standard m).submitted)
  , ("inclusion.standard.included", \m -> int (laneInclusionStats Standard m).included)
  , ("inclusion.standard.serviceRate", laneServiceRate Standard)
  , ("inclusion.priority.submitted", \m -> int (laneInclusionStats Priority m).submitted)
  , ("inclusion.priority.included", \m -> int (laneInclusionStats Priority m).included)
  , ("inclusion.priority.serviceRate", laneServiceRate Priority)
  , ("inclusion.urgent.submitted", \m -> int (urgentInclusionStats m).submitted)
  , ("inclusion.urgent.included", \m -> int (urgentInclusionStats m).included)
  , ("inclusion.urgent.serviceRate", urgentServiceRate)
  , ("load.amplification", loadAmplification)
  , ("load.attemptsMax", \m -> int m.demandLoad.attemptsMax)
  , ("load.postedFeeGrowthMean", \m -> m.demandLoad.postedFeeGrowthMean)
  , ("latency.meanSlots", latencyMeanSlots)
  , ("latency.standard.count", \m -> int (laneLatencyStats Standard m).statCount)
  , ("latency.standard.meanSlots", \m -> (laneLatencyStats Standard m).statMean)
  , ("latency.standard.meanBlocks", \m -> (laneBlockLatencyStats Standard m).statMean)
  , ("latency.priority.count", \m -> int (laneLatencyStats Priority m).statCount)
  , ("latency.priority.meanSlots", \m -> (laneLatencyStats Priority m).statMean)
  , ("latency.priority.meanBlocks", \m -> (laneBlockLatencyStats Priority m).statMean)
  , ("latency.urgent.count", \m -> int (urgentLatencyStats m).statCount)
  , ("latency.urgent.meanSlots", \m -> (urgentLatencyStats m).statMean)
  , ("latency.urgent.meanBlocks", \m -> (urgentBlockLatencyStats m).statMean)
  , ("latency.meanBlocks", latencyMeanBlocks)
  , ("value.retainedLovelace", \m -> lovelace (sumLovelace (fmap (.retainedValue) (Map.elems m.value))))
  , ("value.lostLovelace", \m -> lovelace (sumLovelace (fmap (.lostValue) (Map.elems m.value))))
  , ("value.unresolvedLovelace", \m -> lovelace (sumLovelace (fmap (.unresolvedValue) (Map.elems m.value))))
  , ("value.retainedRatio", retainedValueRatio)
  , ("value.standard.retainedLovelace", \m -> lovelace (laneValueOutcome Standard m).retainedValue)
  , ("value.standard.lostLovelace", \m -> lovelace (laneValueOutcome Standard m).lostValue)
  , ("value.standard.unresolvedLovelace", \m -> lovelace (laneValueOutcome Standard m).unresolvedValue)
  , ("value.standard.retainedRatio", laneRetainedValueRatio Standard)
  , ("value.priority.retainedLovelace", \m -> lovelace (laneValueOutcome Priority m).retainedValue)
  , ("value.priority.lostLovelace", \m -> lovelace (laneValueOutcome Priority m).lostValue)
  , ("value.priority.unresolvedLovelace", \m -> lovelace (laneValueOutcome Priority m).unresolvedValue)
  , ("value.priority.retainedRatio", laneRetainedValueRatio Priority)
  , ("value.urgent.retainedLovelace", \m -> lovelace (urgentValueOutcome m).retainedValue)
  , ("value.urgent.lostLovelace", \m -> lovelace (urgentValueOutcome m).lostValue)
  , ("value.urgent.unresolvedLovelace", \m -> lovelace (urgentValueOutcome m).unresolvedValue)
  , ("value.urgent.retainedRatio", urgentRetainedValueRatio)
  , ("revenue.feesCollectedLovelace", \m -> lovelace m.revenue.feesCollected)
  , ("revenue.refundsPaidLovelace", \m -> lovelace m.revenue.refundsPaid)
  , ("throughput.txPerSlot", \m -> m.throughput.txThroughput)
  , ("throughput.ebUtilization", \m -> m.throughput.ebUtilization)
  , ("price.maxJump", \m -> m.priceShock.maxPriceJump)
  , ("price.shockCount", \m -> int m.priceShock.shockCount)
  , ("price.settledCoefficientRange", \m -> m.priceStability.settledCoefficientRange)
  , ("price.oscillationReversalCount", \m -> int m.priceOscillation.oscillationReversalCount)
  , ("price.oscillationCycleCount", \m -> int m.priceOscillation.oscillationCycleCount)
  , ("price.oscillationMaxAmplitude", \m -> m.priceOscillation.maxOscillationAmplitude)
  , ("price.oscillationExcessTravel", \m -> m.priceOscillation.oscillationExcessTravel)
  ]
 where
  int = fromIntegral
  lovelace (Lovelace n) = fromInteger n

headlineScalars :: Metrics -> [(String, Double)]
headlineScalars metrics =
  [(key, scalar metrics) | (key, scalar) <- headline]

serviceRate :: Metrics -> Double
serviceRate m =
  ratio m.demandLoad.unitsServed m.demandLoad.demandUnits

laneServiceRate :: Lane -> Metrics -> Double
laneServiceRate lane metrics =
  ratio stats.included stats.submitted
 where
  stats = laneInclusionStats lane metrics

laneInclusionStats :: Lane -> Metrics -> InclusionStats
laneInclusionStats lane metrics =
  Map.findWithDefault (InclusionStats 0 0) lane metrics.laneInclusion

urgentServiceRate :: Metrics -> Double
urgentServiceRate metrics =
  ratio stats.included stats.submitted
 where
  stats = urgentInclusionStats metrics

urgentInclusionStats :: Metrics -> InclusionStats
urgentInclusionStats metrics =
  maybe
    (InclusionStats 0 0)
    (\urgency -> Map.findWithDefault (InclusionStats 0 0) urgency metrics.inclusion)
    (urgentClass metrics)

laneLatencyStats :: Lane -> Metrics -> DistStats Duration
laneLatencyStats lane metrics =
  Map.findWithDefault emptyLatency lane metrics.laneLatency
 where
  emptyLatency =
    DistStats
      { statCount = 0
      , statMean = 0
      , statMedian = Duration 0
      , statP95 = Duration 0
      , statMax = Duration 0
      }

laneBlockLatencyStats :: Lane -> Metrics -> DistStats Int
laneBlockLatencyStats lane metrics =
  Map.findWithDefault emptyBlockLatency lane metrics.laneActualBlockLatency

urgentLatencyStats :: Metrics -> DistStats Duration
urgentLatencyStats metrics =
  maybe
    emptyDurationStats
    (\urgency -> Map.findWithDefault emptyDurationStats urgency metrics.latency)
    (urgentClass metrics)

urgentBlockLatencyStats :: Metrics -> DistStats Int
urgentBlockLatencyStats metrics =
  maybe
    emptyBlockLatency
    (\urgency -> Map.findWithDefault emptyBlockLatency urgency metrics.actualBlockLatency)
    (urgentClass metrics)

laneValueOutcome :: Lane -> Metrics -> ValueOutcome
laneValueOutcome lane metrics =
  Map.findWithDefault emptyValue lane metrics.laneValue

urgentValueOutcome :: Metrics -> ValueOutcome
urgentValueOutcome metrics =
  maybe
    emptyValue
    (\urgency -> Map.findWithDefault emptyValue urgency metrics.value)
    (urgentClass metrics)

urgentClass :: Metrics -> Maybe Urgency
urgentClass metrics =
  case Map.keys metrics.inclusion of
    [] -> Nothing
    urgencies -> Just (maximumBy (comparing urgencyScore) urgencies)

urgencyScore :: Urgency -> Double
urgencyScore = \case
  Linear rate -> rate
  Exponential rate -> rate

retainedValueRatio :: Metrics -> Double
retainedValueRatio metrics =
  valueRetainedRatio
    ValueOutcome
      { retainedValue = sumLovelace (fmap (.retainedValue) outcomes)
      , lostValue = sumLovelace (fmap (.lostValue) outcomes)
      , unresolvedValue = sumLovelace (fmap (.unresolvedValue) outcomes)
      }
 where
  outcomes = Map.elems metrics.value

laneRetainedValueRatio :: Lane -> Metrics -> Double
laneRetainedValueRatio lane metrics =
  valueRetainedRatio (laneValueOutcome lane metrics)

urgentRetainedValueRatio :: Metrics -> Double
urgentRetainedValueRatio metrics =
  valueRetainedRatio (urgentValueOutcome metrics)

valueRetainedRatio :: ValueOutcome -> Double
valueRetainedRatio outcome =
  lovelaceRatio
    outcome.retainedValue
    (sumLovelace [outcome.retainedValue, outcome.lostValue])

emptyValue :: ValueOutcome
emptyValue =
  ValueOutcome
    { retainedValue = Lovelace 0
    , lostValue = Lovelace 0
    , unresolvedValue = Lovelace 0
    }

emptyDurationStats :: DistStats Duration
emptyDurationStats =
  DistStats
    { statCount = 0
    , statMean = 0
    , statMedian = Duration 0
    , statP95 = Duration 0
    , statMax = Duration 0
    }

emptyBlockLatency :: DistStats Int
emptyBlockLatency =
  DistStats
    { statCount = 0
    , statMean = 0
    , statMedian = 0
    , statP95 = 0
    , statMax = 0
    }

lovelaceRatio :: Lovelace -> Lovelace -> Double
lovelaceRatio _ (Lovelace denominator) | denominator <= 0 = 0
lovelaceRatio (Lovelace numerator) (Lovelace denominator) =
  fromInteger numerator / fromInteger denominator

loadAmplification :: Metrics -> Double
loadAmplification m =
  ratio m.demandLoad.submissionAttempts m.demandLoad.demandUnits

latencyMeanSlots :: Metrics -> Double
latencyMeanSlots m =
  weightedMean
    [ (fromIntegral stats.statCount, stats.statMean)
    | stats <- Map.elems m.latency
    ]

latencyMeanBlocks :: Metrics -> Double
latencyMeanBlocks m =
  weightedMean
    [ (fromIntegral stats.statCount, stats.statMean)
    | stats <- Map.elems m.actualBlockLatency
    ]

-- | The resolved spec is embedded so the summary is self-describing: which
-- experiment, which designs, which configs produced these numbers.
summaryJson :: SweepSpec -> Maybe ResolvedLoad -> [(SweepVariant, [(Seed, RunScalars)])] -> Value
summaryJson spec resolvedLoad variants =
  object
    [ "description" .= spec.sweepDescription
    , "slots" .= spec.sweepSlots
    , "seeds" .= spec.sweepSeeds
    , "summaryOnly" .= spec.sweepSummaryOnly
    , "loadOverride" .= fmap loadOverrideJson resolvedLoad
    , "loadProfile" .= (resolvedLoad >>= selectedProfileJson)
    , "variants" .= fmap variantJson variants
    ]
 where
  loadOverrideJson = \case
    ResolvedPreset name ->
      object
        [ "type" .= ("preset" :: String)
        , "name" .= name
        ]
    ResolvedProfile source profile ->
      object
        [ "type" .= ("profile" :: String)
        , "name" .= profile.loadProfileName
        , "source" .= source
        , "copy" .= ("selected-load-profile.json" :: FilePath)
        ]
  selectedProfileJson = \case
    ResolvedPreset{} -> Nothing
    ResolvedProfile source profile ->
      Just $
        object
          [ "name" .= profile.loadProfileName
          , "description" .= profile.loadProfileDescription
          , "source" .= source
          , "copy" .= ("selected-load-profile.json" :: FilePath)
          ]
  variantJson (variant, runs) =
    object
      [ "name" .= variant.variantName
      , "config" .= variant.variantConfig
      , "runs" .= fmap runJson runs
      , "aggregates" .= object (fmap aggregateJson (aggregate runs))
      ]
  runJson (seed, scalars) =
    object
      [ "seed" .= seed
      , "scalars" .= object [Key.fromString key .= value | (key, value) <- scalars]
      ]
  aggregateJson (key, stats) =
    Key.fromString key .= statsJson stats
  statsJson stats =
    object
      [ "mean" .= stats.statsMean
      , "stddev" .= stats.statsStdDev
      , "min" .= stats.statsMin
      , "max" .= stats.statsMax
      ]

data SummaryStats = SummaryStats
  { statsMean :: Double
  , statsStdDev :: Double
  , statsMin :: Double
  , statsMax :: Double
  }

aggregate :: [(Seed, RunScalars)] -> [(String, SummaryStats)]
aggregate [] = []
aggregate runs =
  [ (key, summaryStats [lookupScalar key scalars | (_, scalars) <- runs])
  | (key, _) <- headline
  ]

lookupScalar :: String -> RunScalars -> Double
lookupScalar key scalars =
  fromMaybe (error ("internal error: missing sweep scalar " <> show key)) (lookup key scalars)

-- | Sample standard deviation; zero when fewer than two observations.
summaryStats :: [Double] -> SummaryStats
summaryStats [] = SummaryStats 0 0 0 0
summaryStats xs =
  SummaryStats
    { statsMean = mu
    , statsStdDev =
        if n < 2
          then 0
          else sqrt (sum [(x - mu) ^ (2 :: Int) | x <- xs] / fromIntegral (n - 1))
    , statsMin = minimum xs
    , statsMax = maximum xs
    }
 where
  n = length xs
  mu = sum xs / fromIntegral n
