module Load where

import Types (SlotNo (..))

data ArrivalProcess
  = ConstantLoad Double
  | BurstLoad [Burst]

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
        }
    ]

data Burst = Burst
  { baseRate :: Double
  , burstRate :: Double
  , burstStart :: SlotNo
  , burstEnd :: SlotNo
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
