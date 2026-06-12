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

Every (variant, seed) pair is one full traced run. The output directory is
self-contained for reproduction: the resolved spec is embedded in
@summary.json@ and each variant's config is copied alongside the traces.
@f@ and @D@ are not sweep axes: they stay fixed at the justified values in
each config.
-}
module Sweep (
  SweepSpec (..),
  SweepVariant (..),
  SweepOverrides (..),
  applyOverrides,
  loadSweepSpec,
  parseSweepArgs,
  runSweep,
) where

import Config (SimConfig)
import Data.Aeson (FromJSON (..), Value, eitherDecode, encode, object, withObject, (.:), (.:?), (.=))
import Data.Aeson.Key qualified as Key
import Data.ByteString.Lazy qualified as BL
import Data.List (nub)
import Data.Map.Strict qualified as Map
import Data.Maybe (fromMaybe)
import Metrics (
  DemandLoad (..),
  DistStats (..),
  Metrics (..),
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
import Run (Run (..), Seed, runWithSeedToFile)
import System.Directory (copyFile, createDirectoryIfMissing)
import System.FilePath ((</>))
import Text.Printf (printf)
import Types (Lovelace (..))

data SweepSpec = SweepSpec
  { sweepDescription :: Maybe String
  , sweepSeeds :: Int
  , sweepSlots :: Int
  , sweepOutDir :: FilePath
  , sweepVariants :: [SweepVariant]
  }
  deriving (Eq, Show)

data SweepVariant = SweepVariant
  { variantName :: String
  , variantConfig :: FilePath
  }
  deriving (Eq, Show)

-- | Command-line overrides on top of the manifest, for quick iteration
-- without editing the committed experiment definition.
data SweepOverrides = SweepOverrides
  { overrideSeeds :: Maybe Int
  , overrideSlots :: Maybe Int
  , overrideOut :: Maybe FilePath
  }
  deriving (Eq, Show)

parseSweepArgs :: [String] -> Either String (FilePath, SweepOverrides)
parseSweepArgs = go (Nothing, SweepOverrides Nothing Nothing Nothing)
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
    }

data ParseSweepSpec = ParseSweepSpec
  { parseSweepDescription :: Maybe String
  , parseSweepSeeds :: Maybe Int
  , parseSweepSlots :: Maybe Int
  , parseSweepOut :: Maybe FilePath
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
  createDirectoryIfMissing True spec.sweepOutDir
  variants <- traverse (runVariant spec) spec.sweepVariants
  let summaryPath = spec.sweepOutDir </> "summary.json"
  BL.writeFile summaryPath (encode (summaryJson spec variants))
  putStrLn ("wrote " <> summaryPath)

runVariant :: SweepSpec -> SweepVariant -> IO (SweepVariant, [(Seed, Metrics)])
runVariant spec variant = do
  -- the output directory is self-contained for reproduction: the exact
  -- config of every variant rides along with the traces it produced
  copyFile variant.variantConfig (spec.sweepOutDir </> (variant.variantName <> ".config.json"))
  config <- parseSimConfig variant.variantConfig
  runs <-
    traverse (runPoint spec variant.variantName config) (fromIntegral <$> [0 .. spec.sweepSeeds - 1])
  pure (variant, runs)

runPoint :: SweepSpec -> String -> SimConfig -> Seed -> IO (Seed, Metrics)
runPoint spec name config seed = do
  let tracePath = spec.sweepOutDir </> (name <> "-seed" <> show seed <> ".events.jsonl")
  result <- runWithSeedToFile config tracePath seed spec.sweepSlots
  putStrLn (name <> " seed " <> show seed <> ": " <> progressLine result._runResult)
  pure (seed, result._runResult)

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
price trace) stays available in each run's events trace and per-run analysis;
this list is what design comparisons are made on.
-}
headline :: [(String, Metrics -> Double)]
headline =
  [ ("units.total", \m -> int m.demandLoad.demandUnits)
  , ("units.served", \m -> int m.demandLoad.unitsServed)
  , ("units.abandoned", \m -> int m.demandLoad.unitsAbandoned)
  , ("units.unresolved", \m -> int m.demandLoad.unitsUnresolved)
  , ("units.serviceRate", serviceRate)
  , ("load.amplification", loadAmplification)
  , ("load.attemptsMax", \m -> int m.demandLoad.attemptsMax)
  , ("load.postedFeeGrowthMean", \m -> m.demandLoad.postedFeeGrowthMean)
  , ("latency.meanSlots", latencyMeanSlots)
  , ("latency.meanBlocks", latencyMeanBlocks)
  , ("value.retainedLovelace", \m -> lovelace (sumLovelace (fmap (.retainedValue) (Map.elems m.value))))
  , ("value.lostLovelace", \m -> lovelace (sumLovelace (fmap (.lostValue) (Map.elems m.value))))
  , ("value.unresolvedLovelace", \m -> lovelace (sumLovelace (fmap (.unresolvedValue) (Map.elems m.value))))
  , ("revenue.feesCollectedLovelace", \m -> lovelace m.revenue.feesCollected)
  , ("revenue.refundsPaidLovelace", \m -> lovelace m.revenue.refundsPaid)
  , ("throughput.txPerSlot", \m -> m.throughput.txThroughput)
  , ("throughput.ebUtilization", \m -> m.throughput.ebUtilization)
  , ("price.maxJump", \m -> m.priceShock.maxPriceJump)
  , ("price.shockCount", \m -> int m.priceShock.shockCount)
  , ("price.oscillationAmplitude", \m -> m.priceStability.oscillationAmplitude)
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
summaryJson :: SweepSpec -> [(SweepVariant, [(Seed, Metrics)])] -> Value
summaryJson spec variants =
  object
    [ "description" .= spec.sweepDescription
    , "slots" .= spec.sweepSlots
    , "seeds" .= spec.sweepSeeds
    , "variants" .= fmap variantJson variants
    ]
 where
  variantJson (variant, runs) =
    object
      [ "name" .= variant.variantName
      , "config" .= variant.variantConfig
      , "runs" .= fmap runJson runs
      , "aggregates" .= object (fmap aggregateJson (aggregate runs))
      ]
  runJson (seed, metrics) =
    object
      [ "seed" .= seed
      , "scalars" .= object [Key.fromString key .= value | (key, value) <- headlineScalars metrics]
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

aggregate :: [(Seed, Metrics)] -> [(String, SummaryStats)]
aggregate [] = []
aggregate runs =
  [ (key, summaryStats [scalar metrics | (_, metrics) <- runs])
  | (key, scalar) <- headline
  ]

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
