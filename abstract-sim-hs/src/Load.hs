module Load where

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

tryBurstEffectAt :: ArrivalProcess -> SlotNo -> Maybe BurstEffect
tryBurstEffectAt (BurstLoad bursts) slot =
  foldr combineActiveBurstEffect Nothing (fmap (burstEffectAt slot) bursts)
tryBurstEffectAt _ _ = Nothing

combineActiveBurstEffect :: Maybe BurstEffect -> Maybe BurstEffect -> Maybe BurstEffect
combineActiveBurstEffect Nothing acc = acc
combineActiveBurstEffect (Just effect) Nothing = Just effect
combineActiveBurstEffect (Just effect) (Just acc) = Just (combineBurstEffects effect acc)

combineBurstEffects :: BurstEffect -> BurstEffect -> BurstEffect
combineBurstEffects a b =
  BurstEffect
    { valueMultiplier = a.valueMultiplier * b.valueMultiplier
    , urgencyMultiplier = a.urgencyMultiplier * b.urgencyMultiplier
    }

arrivalRateAt :: ArrivalProcess -> SlotNo -> Double
arrivalRateAt (ConstantLoad r) _ = r
arrivalRateAt (BurstLoad bursts) slot =
  sum (fmap (burstRateAt slot) bursts)

burstRateAt :: SlotNo -> Burst -> Double
burstRateAt slot burst
  | slot >= burstStart && slot < burstEnd = burstRate
  | otherwise = baseRate
 where
  burstStart = burst.burstStart
  burstEnd = burst.burstEnd
  burstRate = burst.burstRate
  baseRate = burst.baseRate

burstEffectAt :: SlotNo -> Burst -> Maybe BurstEffect
burstEffectAt slot burst
  | slot >= burstStart && slot < burstEnd = burstRate
  | otherwise = baseRate
 where
  burstStart = burst.burstStart
  burstEnd = burst.burstEnd
  burstRate = Just burst.burstEffect
  baseRate = Nothing
