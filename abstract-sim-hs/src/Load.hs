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

import Types (SlotNo (..))

data ArrivalProcess
  = ConstantLoad Double
  | BurstLoad [Burst]
  deriving stock (Eq, Show)

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

data BurstEffect = BurstEffect
  { valueMultiplier :: Double
  , urgencyMultiplier :: Double
  }
  deriving (Eq, Show)

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
