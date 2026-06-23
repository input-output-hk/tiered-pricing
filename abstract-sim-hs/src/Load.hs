module Load (
  ArrivalProcess (..),
  Burst (..),
  BurstEffect (..),
  arrivalRateAt,
  tryBurstEffectAt,
  moderateLoad,
  congestedLoad,
  burstLoad,
  severeCongestionLoad,
) where

import Data.Aeson (FromJSON (..), Value (String), withObject, (.:))
import Json (Alt (..), taggedSum)
import Types (SlotNo (..))

data ArrivalProcess
  = ConstantLoad Double
  | BurstLoad [Burst]
  deriving stock (Eq, Show)

{- | A bare string selects one of the curated presets; an object gives an
explicit process. The @"burst"@ tag means the preset as a string and a
custom burst list as an object.
-}
instance FromJSON ArrivalProcess where
  parseJSON value@(String _) =
    flip
      (taggedSum "arrival process preset")
      value
      [ ("moderate", Nullary moderateLoad)
      , ("congested", Nullary congestedLoad)
      , ("burst", Nullary burstLoad)
      , ("severe-congestion", Nullary severeCongestionLoad)
      ]
  parseJSON value =
    flip (withObject "ArrivalProcess") value \obj -> do
      tag <- obj .: "type"
      case tag :: String of
        "constant" -> ConstantLoad <$> obj .: "rate"
        "burst" -> BurstLoad <$> obj .: "bursts"
        _ -> fail ("unknown arrival process type: " <> tag)

moderateLoad :: ArrivalProcess
moderateLoad = ConstantLoad 2.0

congestedLoad :: ArrivalProcess
congestedLoad = ConstantLoad 20.0

burstLoad :: ArrivalProcess
burstLoad =
  BurstLoad
    [ Burst
        { baseRate = 2.0
        , burstRate = 40.0
        , burstStart = SlotNo 1_000
        , burstEnd = SlotNo 1_500
        , burstEffect = BurstEffect 1 1
        }
    ]

severeCongestionLoad :: ArrivalProcess
severeCongestionLoad =
  BurstLoad
    [ Burst
        { baseRate = 40.0
        , burstRate = 160.0
        , burstStart = SlotNo 250
        , burstEnd = SlotNo 1_750
        , burstEffect = BurstEffect 1 1
        }
    ]

-- TODO I don't like that `Burst`s can overlap. Feels weird.
data Burst = Burst
  { baseRate :: Double
  , burstRate :: Double
  , burstStart :: SlotNo
  , burstEnd :: SlotNo
  , burstEffect :: BurstEffect
  }
  deriving stock (Eq, Show)

instance FromJSON Burst where
  parseJSON =
    withObject "Burst" \obj ->
      Burst
        <$> obj .: "baseRate"
        <*> obj .: "burstRate"
        <*> (SlotNo <$> obj .: "burstStart")
        <*> (SlotNo <$> obj .: "burstEnd")
        <*> obj .: "burstEffect"

data BurstEffect = BurstEffect
  { valueMultiplier :: Double
  , urgencyMultiplier :: Double
  }
  deriving (Eq, Show)

instance FromJSON BurstEffect where
  parseJSON =
    withObject "BurstEffect" \obj ->
      BurstEffect
        <$> obj .: "valueMultiplier"
        <*> obj .: "urgencyMultiplier"

-- | Overlapping bursts compound multiplicatively.
instance Semigroup BurstEffect where
  a <> b =
    BurstEffect
      { valueMultiplier = a.valueMultiplier * b.valueMultiplier
      , urgencyMultiplier = a.urgencyMultiplier * b.urgencyMultiplier
      }

burstActiveAt :: SlotNo -> Burst -> Bool
burstActiveAt slot burst =
  burst.burstStart <= slot && slot < burst.burstEnd

tryBurstEffectAt :: ArrivalProcess -> SlotNo -> Maybe BurstEffect
tryBurstEffectAt (BurstLoad bursts) slot =
  foldr (<>) Nothing [Just burst.burstEffect | burst <- bursts, burstActiveAt slot burst]
tryBurstEffectAt _ _ = Nothing

arrivalRateAt :: ArrivalProcess -> SlotNo -> Double
arrivalRateAt (ConstantLoad r) _ = r
arrivalRateAt (BurstLoad bursts) slot =
  sum (fmap rateAt bursts)
 where
  rateAt burst
    | burstActiveAt slot burst = burst.burstRate
    | otherwise = burst.baseRate
