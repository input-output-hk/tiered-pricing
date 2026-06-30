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
    , "  abstract-sim-hs-exe sweep MANIFEST [--seeds N] [--slots N] [--out DIR]"
    , "      [--load-profile FILE]"
    , "      the experiment sweep defined by MANIFEST (see config/sweeps/):"
    , "      one traced run per variant x seed, per-variant aggregates in"
    , "      DIR/summary.json; --load-profile applies one workload to every"
    , "      variant without changing its config; other flags override the manifest"
    ]
