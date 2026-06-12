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
  BlockLatencyStats (..),
  DemandLoad (..),
  LatencyStats (..),
  Metrics (..),
  PriceShock (..),
  PriceStability (..),
  Revenue (..),
  Throughput (..),
  ValueOutcome (..),
  ratio,
  sumLovelace,
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

runVariant :: SweepSpec -> SweepVariant -> IO (SweepVariant, [(Seed, [(String, Double)])])
runVariant spec variant = do
  -- the output directory is self-contained for reproduction: the exact
  -- config of every variant rides along with the traces it produced
  copyFile variant.variantConfig (spec.sweepOutDir </> (variant.variantName <> ".config.json"))
  config <- parseSimConfig variant.variantConfig
  runs <-
    traverse (runPoint spec variant.variantName config) (fromIntegral <$> [0 .. spec.sweepSeeds - 1])
  pure (variant, runs)

runPoint :: SweepSpec -> String -> SimConfig -> Seed -> IO (Seed, [(String, Double)])
runPoint spec name config seed = do
  let tracePath = spec.sweepOutDir </> (name <> "-seed" <> show seed <> ".events.jsonl")
  result <- runWithSeedToFile config tracePath seed spec.sweepSlots
  let scalars = headlineScalars result._runResult
  putStrLn (name <> " seed " <> show seed <> ": " <> progressLine scalars)
  pure (seed, scalars)

progressLine :: [(String, Double)] -> String
progressLine scalars =
  printf
    "service %.3f, latency %.1f slots, amplification %.3f"
    (scalar "units.serviceRate")
    (scalar "latency.meanSlots")
    (scalar "load.amplification")
 where
  scalar key = fromMaybe 0 (lookup key scalars)

{- | The per-run scalars that get aggregated across seeds. The full 'Metrics'
detail (slices, percentiles, price trace) stays available in each run's
events trace and per-run analysis; this list is what design comparisons are
made on.
-}
headlineScalars :: Metrics -> [(String, Double)]
headlineScalars metrics =
  [ ("units.total", int load.demandUnits)
  , ("units.served", int load.unitsServed)
  , ("units.abandoned", int load.unitsAbandoned)
  , ("units.unresolved", int load.unitsUnresolved)
  , ("units.serviceRate", ratio load.unitsServed load.demandUnits)
  , ("load.amplification", ratio load.submissionAttempts load.demandUnits)
  , ("load.attemptsMax", int load.attemptsMax)
  , ("load.postedFeeGrowthMean", load.postedFeeGrowthMean)
  , ("latency.meanSlots", weightedMean latencyWeights)
  , ("latency.meanBlocks", weightedMean blockLatencyWeights)
  , ("value.retainedLovelace", lovelace (sumLovelace (fmap (.retainedValue) values)))
  , ("value.lostLovelace", lovelace (sumLovelace (fmap (.lostValue) values)))
  , ("value.unresolvedLovelace", lovelace (sumLovelace (fmap (.unresolvedValue) values)))
  , ("revenue.feesCollectedLovelace", lovelace metrics.revenue.feesCollected)
  , ("revenue.refundsPaidLovelace", lovelace metrics.revenue.refundsPaid)
  , ("throughput.txPerSlot", metrics.throughput.txThroughput)
  , ("throughput.ebUtilization", metrics.throughput.ebUtilization)
  , ("price.maxJump", metrics.priceShock.maxPriceJump)
  , ("price.shockCount", int metrics.priceShock.shockCount)
  , ("price.oscillationAmplitude", metrics.priceStability.oscillationAmplitude)
  ]
 where
  load = metrics.demandLoad
  values = Map.elems metrics.value
  latencyWeights =
    [ (fromIntegral stats.latencyCount, stats.latencyMean)
    | stats <- Map.elems metrics.latency
    ]
  blockLatencyWeights =
    [ (fromIntegral stats.blockLatencyCount, stats.blockLatencyMean)
    | stats <- Map.elems metrics.actualBlockLatency
    ]
  int = fromIntegral
  lovelace (Lovelace n) = fromInteger n

weightedMean :: [(Double, Double)] -> Double
weightedMean weights
  | totalWeight <= 0 = 0
  | otherwise = sum [w * x | (w, x) <- weights] / totalWeight
 where
  totalWeight = sum (fmap fst weights)

-- | The resolved spec is embedded so the summary is self-describing: which
-- experiment, which designs, which configs produced these numbers.
summaryJson :: SweepSpec -> [(SweepVariant, [(Seed, [(String, Double)])])] -> Value
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

aggregate :: [(Seed, [(String, Double)])] -> [(String, SummaryStats)]
aggregate runs =
  case runs of
    [] -> []
    (_, firstScalars) : _ ->
      [ (key, summaryStats [value | (_, scalars) <- runs, Just value <- [lookup key scalars]])
      | (key, _) <- firstScalars
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
