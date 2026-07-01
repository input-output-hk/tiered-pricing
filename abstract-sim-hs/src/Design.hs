module Design (
  Design (..),
  SelectionPolicy (..),
  FeeSemantics (..),
  PriorityPremiumScope (..),
  ReservationPolicy (..),
  LaneStructure (..),
  Eip1559Controller (..),
  ControllerSignal (..),
  ControllerConfig (..),
  defaultDesign,
  defaultControllerConfig,
) where

import Data.Aeson (FromJSON (..), withObject, (.:), (.:?))
import Json (Alt (..), taggedSum)
import Types (Duration (..), PerLane (..))

data Design = Design
  { designLaneStructure :: LaneStructure
  , designReservationPolicy :: ReservationPolicy
  , designSelection :: SelectionPolicy
  , designFeeSemantics :: FeeSemantics
  , designPriorityPremiumScope :: PriorityPremiumScope
  , designControllers :: ControllerConfig
  }
  deriving stock (Eq, Show)

instance FromJSON Design where
  parseJSON =
    withObject "Design" \obj ->
      Design
        <$> obj .: "laneStructure"
        <*> obj .: "reservationPolicy"
        <*> obj .: "selection"
        <*> obj .: "feeSemantics"
        <*> obj .: "priorityPremiumScope"
        <*> obj .: "controllers"

data SelectionPolicy
  = Fifo
  | PriorityFirst
  | FifoWithStandardCap Double
  deriving stock (Eq, Show)

instance FromJSON SelectionPolicy where
  parseJSON =
    taggedSum
      "selection policy"
      [ ("fifo", Nullary Fifo)
      , ("priority-first", Nullary PriorityFirst)
      , ("fifo-with-standard-cap", WithFields \obj -> FifoWithStandardCap <$> obj .: "standardCap")
      ]

data FeeSemantics
  = FixedFee
  | Eip1559 -- User submits max fee they're willing to pay, node refunds difference,
  | HonourSubmissionQuoteFor Duration
  deriving stock (Eq, Show)

instance FromJSON FeeSemantics where
  parseJSON =
    taggedSum
      "fee semantics"
      [ ("fixed-fee", Nullary FixedFee)
      , ("eip1559", Nullary Eip1559)
      , ("honour-submission-quote-for", WithFields \obj -> HonourSubmissionQuoteFor . Duration <$> obj .: "durationSlots")
      ]

{- | Where the priority premium applies — what posting a priority fee buys.
'PremiumEverywhere' charges the priority quote wherever the tx lands.
'PremiumRbOnly' charges it only for RB inclusion: a priority tx landing in
an EB received standard service, so it is refunded down to the standard
quote (the Giorgos design). Only the realised fee is affected — mempool
validity is checked against the posted lane's quote in both scopes.
-}
data PriorityPremiumScope
  = PremiumEverywhere
  | PremiumRbOnly
  deriving stock (Eq, Show)

instance FromJSON PriorityPremiumScope where
  parseJSON =
    taggedSum
      "priority premium scope"
      [ ("everywhere", Nullary PremiumEverywhere)
      , ("rb-only", Nullary PremiumRbOnly)
      ]

data ReservationPolicy
  = PriorityReservationRb Int
  | PriorityReservationRbIfEbNeeded Int
  | NoReservation
  deriving stock (Eq, Show)

instance FromJSON ReservationPolicy where
  parseJSON =
    taggedSum
      "reservation policy"
      [ ("no-reservation", Nullary NoReservation)
      , ("priority-reservation-rb", WithFields \obj -> PriorityReservationRb <$> obj .: "bytes")
      , ("priority-reservation-rb-if-eb-needed", WithFields \obj -> PriorityReservationRbIfEbNeeded <$> obj .: "bytes")
      ]

data LaneStructure = One | Two deriving stock (Eq, Show)

instance FromJSON LaneStructure where
  parseJSON =
    taggedSum
      "lane structure"
      [ ("one", Nullary One)
      , ("two", Nullary Two)
      ]

defaultDesign :: Design
defaultDesign =
  Design
    { designLaneStructure = Two
    , designReservationPolicy = PriorityReservationRb 90_112
    , designSelection = Fifo
    , designFeeSemantics = Eip1559
    , designPriorityPremiumScope = PremiumEverywhere
    , designControllers = defaultControllerConfig
    }

data Eip1559Controller = Eip1559Controller
  { controllerTargetUtilisation :: Double
  , controllerMaxChangeDenominator :: Int
  , controllerInitialCoefficient :: Double
  , controllerSignal :: ControllerSignal
  }
  deriving stock (Eq, Show)

instance FromJSON Eip1559Controller where
  parseJSON =
    withObject "Eip1559Controller" \obj ->
      Eip1559Controller
        <$> obj .: "targetUtilisation"
        <*> obj .: "maxChangeDenominator"
        <*> obj .: "initialCoefficient"
        <*> obj .: "signal"

data ControllerSignal
  = CapacityWeightedWindow Int
  | PriorityReservationUtil
  | PriorityReservationWindow Int
  deriving stock (Eq, Show)

instance FromJSON ControllerSignal where
  parseJSON =
    taggedSum
      "controller signal"
      [ ("priority-reservation-util", Nullary PriorityReservationUtil)
      , ("priority-reservation-window", WithFields \obj -> PriorityReservationWindow <$> obj .: "window")
      , ("capacity-weighted-window", WithFields \obj -> CapacityWeightedWindow <$> obj .: "window")
      ]

data ControllerConfig = ControllerConfig
  { laneControllers :: PerLane (Maybe Eip1559Controller)
  -- ^ a lane without a controller never re-prices
  , multiplierFloor :: Maybe Double
  , absoluteCoeffFloor :: Double
  }
  deriving stock (Eq, Show)

instance FromJSON ControllerConfig where
  parseJSON =
    withObject "ControllerConfig" \obj -> do
      standard <- obj .:? "standardController"
      priority <- obj .:? "priorityController"
      multiplierFloor <- obj .:? "multiplierFloor"
      absoluteCoeffFloor <- obj .: "absoluteCoeffFloor"
      pure
        ControllerConfig
          { laneControllers = PerLane{perStandard = standard, perPriority = priority}
          , multiplierFloor
          , absoluteCoeffFloor
          }

defaultControllerConfig :: ControllerConfig
defaultControllerConfig =
  ControllerConfig
    { laneControllers =
        PerLane
          { perStandard =
              Just
                Eip1559Controller
                  { controllerTargetUtilisation = 0.50
                  , controllerMaxChangeDenominator = 8
                  , controllerInitialCoefficient = 1.0
                  , controllerSignal = CapacityWeightedWindow 20
                  }
          , perPriority =
              Just
                Eip1559Controller
                  { controllerTargetUtilisation = 0.50
                  , controllerMaxChangeDenominator = 8
                  , controllerInitialCoefficient = 16.0
                  , controllerSignal = PriorityReservationUtil
                  }
          }
    , multiplierFloor = Nothing -- Just 16.0
    , absoluteCoeffFloor = 1.0
    }
