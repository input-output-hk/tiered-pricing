module Result where

import Metrics (Metrics)

type Stream a = [a]

newtype Result = Result (Stream Metrics)
  deriving (Eq, Show)
