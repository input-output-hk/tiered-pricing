module Design where

import Types (Duration)

data Design = Design
  { designLaneStructure :: LaneStructure
  , designPricing :: LanePricing
  , designReservationPolicy :: ReservationPolicy
  , designSelection :: SelectionPolicy
  , designFeeSemantics :: FeeSemantics
  , designControllers :: ControllerConfig
  }
  deriving stock (Eq, Show)

data SelectionPolicy
  = Fifo
  | PriorityFirst
  | FifoWithStandardCap Double
  deriving stock (Eq, Show)

data FeeSemantics
  = FixedFee
  | Eip1559 -- User submits max fee they're willing to pay, node refunds difference,
  | HonourSubmissionQuoteFor Duration
  deriving stock (Eq, Show)

data ReservationPolicy
  = PriorityReservationRb Int
  | NoReservation
  deriving stock (Eq, Show)

data LaneStructure = One | Two deriving stock (Eq, Show)

data LanePricing
  = NoDynamic
  | StandardOnlyDynamic
  | PriorityOnlyDynamic
  | BothDynamic
  deriving stock (Eq, Show)

defaultDesign :: Design
defaultDesign =
  Design
    { designLaneStructure = Two
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
  deriving stock (Eq, Show)

data ControllerSignal
  = CapacityWeightedWindow Int
  | PriorityReservationUtil
  deriving stock (Eq, Show)

data ControllerConfig = ControllerConfig
  { standardController :: Maybe Eip1559Controller
  , priorityController :: Maybe Eip1559Controller
  , multiplierFloor :: Maybe Double
  , absoluteCoeffFloor :: Double
  }
  deriving stock (Eq, Show)

defaultControllerConfig :: ControllerConfig
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
