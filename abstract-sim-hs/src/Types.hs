{- | Shared domain types.

Value/time units and the urgency signal. These are domain-wide concepts (a
transaction carries an 'Urgency'; balances and fees are 'Lovelace'), so they
live here rather than in any single consumer like "Metrics".
-}
module Types (
  Lovelace (..),
  Duration (..),
  BlockDelay (..),
  Urgency (..),
  SlotNo (..),
  addDuration,
  diffSlots,
  expectedBlockDelay,
) where

import Data.Aeson (ToJSON (..), object, (.=))

-- | Value in lovelace (integral; 1 ADA = 1e6 lovelace).
newtype Lovelace = Lovelace {unLovelace :: Integer}
  deriving (Eq, Ord, Show)

instance ToJSON Lovelace where
  toJSON (Lovelace n) = toJSON n

-- | A duration or latency measured in slots.
newtype Duration = Duration Int
  deriving (Eq, Ord, Show)

instance ToJSON Duration where
  toJSON (Duration n) = toJSON n

-- | A duration measured in expected ranking blocks.
newtype BlockDelay = BlockDelay Double
  deriving (Eq, Ord, Show)

instance ToJSON BlockDelay where
  toJSON (BlockDelay n) = toJSON n

{- | An absolute slot number — a point in time. Differences between slots are
'Duration's, so 'SlotNo' deliberately has no 'Num' instance (slot × slot,
slot + slot, and negate are nonsense); use 'addDuration' and 'diffSlots'.
-}
newtype SlotNo = SlotNo Int
  deriving (Eq, Ord, Show)

instance ToJSON SlotNo where
  toJSON (SlotNo n) = toJSON n

-- | Advance a slot by a duration: @addDuration d s == s + d@.
addDuration :: Duration -> SlotNo -> SlotNo
addDuration (Duration d) (SlotNo s) = SlotNo (s + d)

-- | The signed gap between two slots: @diffSlots a b == a - b@.
diffSlots :: SlotNo -> SlotNo -> Duration
diffSlots (SlotNo a) (SlotNo b) = Duration (a - b)

-- | Convert slot latency to expected ranking-block latency.
expectedBlockDelay :: Double -> Duration -> BlockDelay
expectedBlockDelay f (Duration slots) =
  BlockDelay (max 0 f * fromIntegral (max 0 slots))

data Urgency = Linear Double | Exponential Double
  deriving (Eq, Ord, Show)

instance ToJSON Urgency where
  toJSON = \case
    Linear rate ->
      object
        [ "tag" .= ("Linear" :: String)
        , "rate" .= rate
        ]
    Exponential rate ->
      object
        [ "tag" .= ("Exponential" :: String)
        , "rate" .= rate
        ]
