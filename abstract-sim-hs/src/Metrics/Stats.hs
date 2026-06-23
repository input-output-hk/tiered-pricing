{- | Distribution summaries and small numeric helpers, shared by the metric
families and the sweep aggregator. A leaf module: imports nothing from the
rest of the simulator, so anything may use it without layering concerns.
-}
module Metrics.Stats (
  DistStats (..),
  summarize,
  quantile,
  mean,
  maximumOrZero,
  ratio,
  weightedMean,
) where

import Data.List (sort)

{- | A five-number summary of an integer-valued sample, with the order
statistics carried at a domain-specific type (e.g. 'Types.Duration') via
'Functor': @fmap Duration . summarize@.
-}
data DistStats a = DistStats
  { statCount :: Int
  -- ^ number of observations contributing to this summary
  , statMean :: Double
  , statMedian :: a
  , statP95 :: a
  , statMax :: a
  }
  deriving stock (Eq, Show, Functor)

-- | Summarise a sample; all-zero stats when empty.
summarize :: [Int] -> DistStats Int
summarize sample =
  case sort sample of
    [] ->
      DistStats
        { statCount = 0
        , statMean = 0
        , statMedian = 0
        , statP95 = 0
        , statMax = 0
        }
    xs ->
      DistStats
        { statCount = n
        , statMean = fromIntegral (sum xs) / fromIntegral n
        , statMedian = quantile 0.50 xs
        , statP95 = quantile 0.95 xs
        , statMax = last xs
        }
     where
      n = length xs

-- | Nearest-rank quantile of an already-sorted, non-empty sample.
quantile :: Double -> [Int] -> Int
quantile q xs =
  xs !! index
 where
  n = length xs
  index = min (n - 1) (max 0 (ceiling (q * fromIntegral n) - 1))

mean :: [Double] -> Double
mean [] = 0
mean xs = sum xs / fromIntegral (length xs)

maximumOrZero :: [Double] -> Double
maximumOrZero [] = 0
maximumOrZero xs = maximum xs

ratio :: Int -> Int -> Double
ratio _ denominator | denominator <= 0 = 0
ratio numerator denominator =
  fromIntegral numerator / fromIntegral denominator

weightedMean :: [(Double, Double)] -> Double
weightedMean weights
  | totalWeight <= 0 = 0
  | otherwise = sum [w * x | (w, x) <- weights] / totalWeight
 where
  totalWeight = sum (fmap fst weights)
