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
    , "  abstract-sim-hs-exe sweep MANIFEST [--seed-start N] [--seeds N] [--slots N] [--out DIR]"
    , "      [--load PRESET | --load-profile FILE]"
    , "      [--summary-only]"
    , "      the experiment sweep defined by MANIFEST (see config/sweeps/):"
    , "      one run per variant x seed, per-variant aggregates in"
    , "      DIR/summary.json; event traces are written unless --summary-only"
    , "      is used. Flags override the manifest for quick iteration."
    , "      --seed-start selects the first seed (default 0), allowing a"
    , "      confirmation run to use seeds disjoint from an exploratory screen."
    , "      --load forces every variant onto one load preset (e.g. low,"
    , "      severe-congestion); --load-profile accepts an explicit profile file"
    , "      and records it with the effective configs in the output directory"
    ]
