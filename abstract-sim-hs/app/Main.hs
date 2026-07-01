module Main (main) where

import Run (run')
import Sweep (applyOverrides, loadSweepSpec, parseSweepArgs, runSweep)
import System.Environment (getArgs)
import System.Exit (die)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> run'
    "sweep" : rest ->
      case parseSweepArgs rest of
        Left err -> die (err <> "\n\n" <> usage)
        Right (manifestPath, overrides) -> do
          spec <- loadSweepSpec manifestPath
          runSweep (applyOverrides overrides spec)
    _ -> die usage

usage :: String
usage =
  unlines
    [ "usage:"
    , "  abstract-sim-hs-exe"
    , "      single traced run of config/default-sim-config.json"
    , "  abstract-sim-hs-exe sweep MANIFEST [--seeds N] [--slots N] [--out DIR] [--load PRESET]"
    , "      the experiment sweep defined by MANIFEST (see config/sweeps/):"
    , "      one traced run per variant x seed, per-variant aggregates in"
    , "      DIR/summary.json; flags override the manifest for quick iteration."
    , "      --load forces every variant onto one load preset (e.g. low,"
    , "      severe-congestion), overriding the load in each variant config"
    ]
