module Design where

import Types (Duration)

data Design (s :: LaneStructure) = Design
  { designLaneStructure :: SLaneStructure s
  , designPricing :: LanePricing s
  , designReservationPolicy :: ReservationPolicy s
  , designSelection :: SelectionPolicy s
  , designFeeSemantics :: FeeSemantics
  , designControllers :: ControllerConfig s
  }

data SelectionPolicy s where
  Fifo :: SelectionPolicy s
  PriorityFirst :: SelectionPolicy 'Two
  FifoWithStandardCap :: Double -> SelectionPolicy 'Two

data FeeSemantics
  = FixedFee
  | Eip1559 -- User submits max fee they're willing to pay, node refunds difference,
  | HonourSubmissionQuoteFor Duration

data ReservationPolicy (s :: LaneStructure) where
  PriorityReservationRb :: Int -> ReservationPolicy 'Two
  NoReservation :: ReservationPolicy s

data LaneStructure = One | Two deriving stock (Eq, Show)

data SLaneStructure (s :: LaneStructure) where
  SOne :: SLaneStructure 'One
  STwo :: SLaneStructure 'Two

data LanePricing (s :: LaneStructure) where
  NoDynamic :: LanePricing s
  StandardOnlyDynamic :: LanePricing s
  PriorityOnlyDynamic :: LanePricing 'Two
  BothDynamic :: LanePricing 'Two

defaultDesign :: Design 'Two
defaultDesign =
  Design
    { designLaneStructure = STwo
    , designPricing = BothDynamic
    , designReservationPolicy = PriorityReservationRb 90_112
    , designSelection = Fifo
    , designFeeSemantics = Eip1559
    , designControllers = defaultControllerConfig
    }

data Eip1559Controller = Eip1559Controller
  { controllerTargetUtilisation :: Double
  , controllerMaxChangeDenominator :: Int
  , controllerInitialCoefficient :: Double
  , controllerSignal :: ControllerSignal
  }

data ControllerSignal
  = CapacityWeightedWindow Int
  | PriorityReservationUtil

data ControllerConfig s = ControllerConfig
  { standardController :: Maybe Eip1559Controller
  , priorityController :: Maybe Eip1559Controller
  , multiplierFloor :: Maybe Double
  , absoluteCoeffFloor :: Double
  }

defaultControllerConfig :: ControllerConfig 'Two
defaultControllerConfig =
  ControllerConfig
    { standardController =
        Just
          Eip1559Controller
            { controllerTargetUtilisation = 0.50
            , controllerMaxChangeDenominator = 8
            , controllerInitialCoefficient = 1.0
            , controllerSignal = CapacityWeightedWindow 20
            }
    , priorityController =
        Just
          Eip1559Controller
            { controllerTargetUtilisation = 0.50
            , controllerMaxChangeDenominator = 8
            , controllerInitialCoefficient = 16.0
            , controllerSignal = PriorityReservationUtil
            }
    , multiplierFloor = Nothing -- Just 16.0
    , absoluteCoeffFloor = 1.0
    }
