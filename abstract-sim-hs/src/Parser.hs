module Parser (
  ParseSimConfig (..),
  ParseCurves (..),
  ParseCurve (..),
  ParseCurvePoint (..),
  ParseArrivalProcess (..),
  ParseBurst (..),
  ParseBurstEffect (..),
  ParseActorPopulation (..),
  ParseActorProfile (..),
  ParseActorPolicy (..),
  ParseLaneLatencyEstimate (..),
  ParseLaneStructure (..),
  ParseLanePricing (..),
  ParseReservationPolicy (..),
  ParseSelectionPolicy (..),
  ParseFeeSemantics (..),
  ParseEip1559Controller (..),
  ParseControllerSignal (..),
  ParseControllerConfig (..),
  ParseDesign (..),
  ParseError (..),
  fromParseSimConfig,
  fromParseDesign,
  parseSimConfig,
  parseDesign,
) where

import Actor (Actor (..), ActorId (..), ActorPolicy (..), ActorProfile (..), LaneLatencyEstimate (..))
import Config (SimConfig (..))
import Curve
  ( Curve (..)
  , CurvePoint (..)
  , Curves (..)
  , ExUnitsCurve (..)
  , ScriptSizeCurve (..)
  , TxSizeCurve (..)
  , TxValueCurve (..)
  , curvesDefault
  )
import Data.Aeson (FromJSON (..), Value (..), eitherDecode, withObject, (.:), (.:?))
import Data.Aeson.Types qualified as Aeson
import Data.ByteString.Lazy qualified as B
import Data.List.NonEmpty (NonEmpty (..), nonEmpty)
import Data.Maybe (isJust)
import Design qualified
import Load
  ( ArrivalProcess (..)
  , Burst (..)
  , BurstEffect (..)
  , burstLoad
  , congestedLoad
  , moderateLoad
  , severeCongestionLoad
  )
import Types (Duration (..), SlotNo (..))

data ParseSimConfig = ParseSimConfig
  { parseSimConfigDesign :: ParseDesign
  , parseSimConfigCurves :: ParseCurves
  , parseSimConfigF :: Double
  , parseSimConfigD :: Int
  , parseSimConfigLoad :: ParseArrivalProcess
  , parseSimConfigActors :: [ParseActorPopulation]
  , parseSimConfigRbTxBytesCap :: Int
  , parseSimConfigRbExUnitsCap :: Int
  , parseSimConfigEbTxBytesCap :: Int
  , parseSimConfigEbStructureBytesCap :: Int
  , parseSimConfigEbExUnitsCap :: Int
  , parseSimConfigMempoolBytesCap :: Int
  , parseSimConfigLaneLatencyEstimate :: ParseLaneLatencyEstimate
  , parseSimConfigPriceConvergenceBandPct :: Double
  , parseSimConfigLoadChangePct :: Double
  }
  deriving stock (Eq, Show)

instance FromJSON ParseSimConfig where
  parseJSON =
    withObject "ParseSimConfig" \object -> do
      parseSimConfigD <- parseD object
      ParseSimConfig
        <$> object .: "design"
        <*> object .: "curves"
        <*> object .: "f"
        <*> pure parseSimConfigD
        <*> object .: "load"
        <*> object .: "actors"
        <*> object .: "rbTxBytesCap"
        <*> object .: "rbExUnitsCap"
        <*> object .: "ebTxBytesCap"
        <*> object .: "ebStructureBytesCap"
        <*> object .: "ebExUnitsCap"
        <*> object .: "mempoolBytesCap"
        <*> object .: "laneLatencyEstimate"
        <*> object .: "priceConvergenceBandPct"
        <*> object .: "loadChangePct"
   where
    parseD object = do
      md <- object .:? "d"
      case md of
        Just d -> pure d
        Nothing -> object .: "D"

data ParseCurves
  = DefaultCurvesP
  | ParseCurves ParseCurve ParseCurve ParseCurve ParseCurve
  deriving stock (Eq, Show)

instance FromJSON ParseCurves where
  parseJSON value@(String _) =
    parseTag "ParseCurves" value >>= \case
      "default" -> pure DefaultCurvesP
      tag -> fail ("unknown curves preset: " <> tag)
  parseJSON value =
    withObject "ParseCurves" parse value
   where
    parse object = do
      tag <- object .:? "type"
      case tag of
        Nothing ->
          ParseCurves
            <$> object .: "txSize"
            <*> object .: "scriptSize"
            <*> object .: "exUnits"
            <*> object .: "txValue"
        Just ("default" :: String) ->
          pure DefaultCurvesP
        Just unknown ->
          fail ("unknown curves type: " <> unknown)

data ParseCurve
  = ConstantP Double
  | PiecewiseLinearP (NonEmpty ParseCurvePoint)
  deriving stock (Eq, Show)

instance FromJSON ParseCurve where
  parseJSON value@(Number _) =
    ConstantP <$> parseJSON value
  parseJSON value =
    withObject "ParseCurve" parse value
   where
    parse object = do
      tag <- object .: "type"
      case tag of
        "constant" -> ConstantP <$> object .: "value"
        "piecewise-linear" -> do
          points <- object .: "points"
          case nonEmpty points of
            Just nonEmptyPoints -> pure (PiecewiseLinearP nonEmptyPoints)
            Nothing -> fail "piecewise-linear curve requires at least one point"
        _ -> fail ("unknown curve type: " <> tag)

data ParseCurvePoint = ParseCurvePoint
  { parseCurveP :: Double
  , parseCurveValue :: Double
  }
  deriving stock (Eq, Show)

instance FromJSON ParseCurvePoint where
  parseJSON =
    withObject "ParseCurvePoint" \object ->
      ParseCurvePoint
        <$> object .: "p"
        <*> object .: "value"

data ParseArrivalProcess
  = ModerateLoadP
  | CongestedLoadP
  | BurstLoadPresetP
  | SevereCongestionLoadP
  | ConstantLoadP Double
  | BurstLoadP [ParseBurst]
  deriving stock (Eq, Show)

instance FromJSON ParseArrivalProcess where
  parseJSON value@(String _) =
    parseTag "ParseArrivalProcess" value >>= \case
      "moderate" -> pure ModerateLoadP
      "congested" -> pure CongestedLoadP
      "burst" -> pure BurstLoadPresetP
      "severe-congestion" -> pure SevereCongestionLoadP
      tag -> fail ("unknown arrival process preset: " <> tag)
  parseJSON value =
    withObject "ParseArrivalProcess" parse value
   where
    parse object = do
      tag <- object .: "type"
      case tag of
        "constant" -> ConstantLoadP <$> object .: "rate"
        "burst" -> BurstLoadP <$> object .: "bursts"
        _ -> fail ("unknown arrival process type: " <> tag)

data ParseBurst = ParseBurst
  { parseBaseRate :: Double
  , parseBurstRate :: Double
  , parseBurstStart :: Int
  , parseBurstEnd :: Int
  , parseBurstEffect :: ParseBurstEffect
  }
  deriving stock (Eq, Show)

instance FromJSON ParseBurst where
  parseJSON =
    withObject "ParseBurst" \object ->
      ParseBurst
        <$> object .: "baseRate"
        <*> object .: "burstRate"
        <*> object .: "burstStart"
        <*> object .: "burstEnd"
        <*> object .: "burstEffect"

data ParseBurstEffect = ParseBurstEffect
  { parseValueMultiplier :: Double
  , parseUrgencyMultiplier :: Double
  }
  deriving stock (Eq, Show)

instance FromJSON ParseBurstEffect where
  parseJSON =
    withObject "ParseBurstEffect" \object ->
      ParseBurstEffect
        <$> object .: "valueMultiplier"
        <*> object .: "urgencyMultiplier"

data ParseActorPopulation = ParseActorPopulation
  { parseActorCount :: Int
  , parseActorProfile :: ParseActorProfile
  }
  deriving stock (Eq, Show)

instance FromJSON ParseActorPopulation where
  parseJSON =
    withObject "ParseActorPopulation" \object ->
      ParseActorPopulation
        <$> object .: "count"
        <*> object .: "profile"

data ParseActorProfile
  = HonestProfileP ParseActorPolicy
  deriving stock (Eq, Show)

instance FromJSON ParseActorProfile where
  parseJSON =
    withObject "ParseActorProfile" \object -> do
      tag <- object .: "type"
      case tag of
        "honest" ->
          HonestProfileP
            <$> ( ParseActorPolicy
                    <$> object .: "feeBuffer"
                    <*> object .: "minValueFeeMultiple"
                )
        _ -> fail ("unknown actor profile: " <> tag)

data ParseActorPolicy = ParseActorPolicy
  { parseActorFeeBuffer :: Double
  , parseActorMinValueFeeMultiple :: Double
  }
  deriving stock (Eq, Show)

data ParseLaneLatencyEstimate = ParseLaneLatencyEstimate
  { parseExpectedStandardLatency :: Int
  , parseExpectedPriorityLatency :: Int
  }
  deriving stock (Eq, Show)

instance FromJSON ParseLaneLatencyEstimate where
  parseJSON =
    withObject "ParseLaneLatencyEstimate" \object ->
      ParseLaneLatencyEstimate
        <$> object .: "expectedStandardLatency"
        <*> object .: "expectedPriorityLatency"

data ParseLaneStructure = OneP | TwoP
  deriving stock (Eq, Show)

instance FromJSON ParseLaneStructure where
  parseJSON value =
    parseTag "ParseLaneStructure" value >>= \case
      "one" -> pure OneP
      "two" -> pure TwoP
      tag -> fail ("unknown lane structure: " <> tag)

data ParseLanePricing = NoDynamicP | StandardOnlyDynamicP | PriorityOnlyDynamicP | BothDynamicP
  deriving stock (Eq, Show)

instance FromJSON ParseLanePricing where
  parseJSON value =
    parseTag "ParseLanePricing" value >>= \case
      "no-dynamic" -> pure NoDynamicP
      "standard-only-dynamic" -> pure StandardOnlyDynamicP
      "priority-only-dynamic" -> pure PriorityOnlyDynamicP
      "both-dynamic" -> pure BothDynamicP
      tag -> fail ("unknown lane pricing: " <> tag)

data ParseReservationPolicy
  = PriorityReservationRb Int
  | NoReservation
  deriving stock (Eq, Show)

instance FromJSON ParseReservationPolicy where
  parseJSON value@(String _) =
    parseTag "ParseReservationPolicy" value >>= \case
      "no-reservation" -> pure NoReservation
      tag -> fail ("reservation policy " <> tag <> " requires an object")
  parseJSON value =
    withObject "ParseReservationPolicy" parse value
   where
    parse object = do
      tag <- object .: "type"
      case tag of
        "no-reservation" -> pure NoReservation
        "priority-reservation-rb" -> PriorityReservationRb <$> object .: "bytes"
        _ -> fail ("unknown reservation policy: " <> tag)

data ParseSelectionPolicy
  = FifoP
  | PriorityFirstP
  | FifoWithStandardCapP Double
  deriving stock (Eq, Show)

instance FromJSON ParseSelectionPolicy where
  parseJSON value@(String _) =
    parseTag "ParseSelectionPolicy" value >>= \case
      "fifo" -> pure FifoP
      "priority-first" -> pure PriorityFirstP
      tag -> fail ("selection policy " <> tag <> " requires an object")
  parseJSON value =
    withObject "ParseSelectionPolicy" parse value
   where
    parse object = do
      tag <- object .: "type"
      case tag of
        "fifo" -> pure FifoP
        "priority-first" -> pure PriorityFirstP
        "fifo-with-standard-cap" -> FifoWithStandardCapP <$> object .: "standardCap"
        _ -> fail ("unknown selection policy: " <> tag)

data ParseFeeSemantics
  = FixedFeeP
  | Eip1559P -- User submits max fee they're willing to pay, node refunds difference,
  | HonourSubmissionQuoteForP Int
  deriving stock (Eq, Show)

instance FromJSON ParseFeeSemantics where
  parseJSON value@(String _) =
    parseTag "ParseFeeSemantics" value >>= \case
      "fixed-fee" -> pure FixedFeeP
      "eip1559" -> pure Eip1559P
      tag -> fail ("fee semantics " <> tag <> " requires an object")
  parseJSON value =
    withObject "ParseFeeSemantics" parse value
   where
    parse object = do
      tag <- object .: "type"
      case tag of
        "fixed-fee" -> pure FixedFeeP
        "eip1559" -> pure Eip1559P
        "honour-submission-quote-for" -> HonourSubmissionQuoteForP <$> object .: "durationSlots"
        _ -> fail ("unknown fee semantics: " <> tag)

data ParseEip1559Controller = ParseEip1559Controller
  { parseControllerTargetUtilisation :: Double
  , parseControllerMaxChangeDenominator :: Int
  , parseControllerInitialCoefficient :: Double
  , parseControllerSignal :: ParseControllerSignal
  }
  deriving stock (Eq, Show)

instance FromJSON ParseEip1559Controller where
  parseJSON =
    withObject "ParseEip1559Controller" \object ->
      ParseEip1559Controller
        <$> object .: "targetUtilisation"
        <*> object .: "maxChangeDenominator"
        <*> object .: "initialCoefficient"
        <*> object .: "signal"

data ParseControllerSignal
  = CapacityWeightedWindowP Int
  | PriorityReservationUtilP
  deriving stock (Eq, Show)

instance FromJSON ParseControllerSignal where
  parseJSON value@(String _) =
    parseTag "ParseControllerSignal" value >>= \case
      "priority-reservation-util" -> pure PriorityReservationUtilP
      tag -> fail ("controller signal " <> tag <> " requires an object")
  parseJSON value =
    withObject "ParseControllerSignal" parse value
   where
    parse object = do
      tag <- object .: "type"
      case tag of
        "priority-reservation-util" -> pure PriorityReservationUtilP
        "capacity-weighted-window" -> CapacityWeightedWindowP <$> object .: "window"
        _ -> fail ("unknown controller signal: " <> tag)

data ParseControllerConfig = ParseControllerConfig
  { parseStandardController :: Maybe ParseEip1559Controller
  , parsePriorityController :: Maybe ParseEip1559Controller
  , parseMultiplierFloor :: Maybe Double
  , parseAbsoluteCoeffFloor :: Double
  }
  deriving stock (Eq, Show)

instance FromJSON ParseControllerConfig where
  parseJSON =
    withObject "ParseControllerConfig" \object ->
      ParseControllerConfig
        <$> object .:? "standardController"
        <*> object .:? "priorityController"
        <*> object .:? "multiplierFloor"
        <*> object .: "absoluteCoeffFloor"

data ParseDesign = ParseDesign
  { parseDesignLaneStructure :: ParseLaneStructure
  , parseDesignPricing :: ParseLanePricing
  , parseDesignReservationPolicy :: ParseReservationPolicy
  , parseDesignSelection :: ParseSelectionPolicy
  , parseDesignFeeSemantics :: ParseFeeSemantics
  , parseDesignControllers :: ParseControllerConfig
  }
  deriving stock (Eq, Show)

instance FromJSON ParseDesign where
  parseJSON =
    withObject "ParseDesign" \object ->
      ParseDesign
        <$> object .: "laneStructure"
        <*> object .: "pricing"
        <*> object .: "reservationPolicy"
        <*> object .: "selection"
        <*> object .: "feeSemantics"
        <*> object .: "controllers"

parseTag :: String -> Value -> Aeson.Parser String
parseTag _ value@(String _) =
  parseJSON value
parseTag name value =
  withObject name (.: "type") value

parseSimConfig :: FilePath -> IO SimConfig
parseSimConfig =
  parseFileWith "sim config" fromParseSimConfig

parseDesign :: FilePath -> IO Design.Design
parseDesign =
  parseFileWith "design" fromParseDesign

parseFileWith ::
  (FromJSON parsed) =>
  String ->
  (parsed -> Either ParseError domain) ->
  FilePath ->
  IO domain
parseFileWith label convert fp = do
  cfg <- B.readFile fp
  case eitherDecode cfg of
    Left err ->
      fail ("cannot parse " <> label <> " " <> fp <> ": " <> err)
    Right parsed ->
      case convert parsed of
        Left err -> fail ("invalid " <> label <> " " <> fp <> ": " <> show err)
        Right domain -> pure domain

fromParseSimConfig :: ParseSimConfig -> Either ParseError SimConfig
fromParseSimConfig pc = do
  simConfigDesign <- fromParseDesign pc.parseSimConfigDesign
  simConfigActors <- toActors pc.parseSimConfigActors
  pure
    SimConfig
      { simConfigDesign
      , simConfigCurves = toCurves pc.parseSimConfigCurves
      , simConfigF = pc.parseSimConfigF
      , simConfigD = pc.parseSimConfigD
      , simConfigLoad = toArrivalProcess pc.parseSimConfigLoad
      , simConfigActors
      , simConfigRbTxBytesCap = pc.parseSimConfigRbTxBytesCap
      , simConfigRbExUnitsCap = pc.parseSimConfigRbExUnitsCap
      , simConfigEbTxBytesCap = pc.parseSimConfigEbTxBytesCap
      , simConfigEbStructureBytesCap = pc.parseSimConfigEbStructureBytesCap
      , simConfigEbExUnitsCap = pc.parseSimConfigEbExUnitsCap
      , simConfigMempoolBytesCap = pc.parseSimConfigMempoolBytesCap
      , simConfigLaneLatencyEstimate = toLaneLatencyEstimate pc.parseSimConfigLaneLatencyEstimate
      , simConfigPriceConvergenceBandPct = pc.parseSimConfigPriceConvergenceBandPct
      , simConfigLoadChangePct = pc.parseSimConfigLoadChangePct
      }

fromParseDesign :: ParseDesign -> Either ParseError Design.Design
fromParseDesign pd =
  do
    validateDesign pd
    pure
      Design.Design
        { Design.designLaneStructure = toDesignLaneStructure pd.parseDesignLaneStructure
        , Design.designPricing = toDesignPricing pd.parseDesignPricing
        , Design.designReservationPolicy = toDesignReservationPolicy pd.parseDesignReservationPolicy
        , Design.designSelection = toDesignSelectionPolicy pd.parseDesignSelection
        , Design.designFeeSemantics = toDesignFeeSemantics pd.parseDesignFeeSemantics
        , Design.designControllers = toDesignControllerConfig pd.parseDesignControllers
        }

validateDesign :: ParseDesign -> Either ParseError ()
validateDesign pd = do
  validateOneLaneSemantics pd
  validatePricingControllers pd

validateOneLaneSemantics :: ParseDesign -> Either ParseError ()
validateOneLaneSemantics pd =
  case pd.parseDesignLaneStructure of
    TwoP -> Right ()
    OneP -> do
      case pd.parseDesignPricing of
        NoDynamicP -> Right ()
        StandardOnlyDynamicP -> Right ()
        PriorityOnlyDynamicP ->
          Left (MismatchedLaneSemantics "cannot have priority-only dynamic pricing with a single lane structure")
        BothDynamicP ->
          Left (MismatchedLaneSemantics "cannot have priority lane pricing semantics with a single lane structure")
      case pd.parseDesignReservationPolicy of
        NoReservation -> Right ()
        PriorityReservationRb{} ->
          Left (MismatchedLaneSemantics "cannot reserve priority ranking-block bytes with a single lane structure")
      case pd.parseDesignSelection of
        FifoP -> Right ()
        PriorityFirstP ->
          Left (MismatchedLaneSemantics "cannot use priority-first selection with a single lane structure")
        FifoWithStandardCapP{} ->
          Left (MismatchedLaneSemantics "cannot cap standard-lane FIFO selection with a single lane structure")
      case pd.parseDesignControllers.parsePriorityController of
        Nothing -> Right ()
        Just{} ->
          Left (MismatchedLaneSemantics "cannot configure a priority controller with a single lane structure")

validatePricingControllers :: ParseDesign -> Either ParseError ()
validatePricingControllers pd =
  let controllers = pd.parseDesignControllers
      hasStandard = isJust controllers.parseStandardController
      hasPriority = isJust controllers.parsePriorityController
   in case (pd.parseDesignPricing, hasStandard, hasPriority) of
        (NoDynamicP, False, False) -> Right ()
        (StandardOnlyDynamicP, True, False) -> Right ()
        (PriorityOnlyDynamicP, False, True) -> Right ()
        (BothDynamicP, True, True) -> Right ()
        _ ->
          Left
            ( MismatchedLaneSemantics
                "lane pricing must match configured standardController and priorityController presence"
            )

toCurves :: ParseCurves -> Curves
toCurves = \case
  DefaultCurvesP -> curvesDefault
  ParseCurves parseCurveTxSize parseCurveScriptSize parseCurveExUnits parseCurveTxValue ->
    Curves
      { curveTxSize = TxSizeCurve (toCurve parseCurveTxSize)
      , curveScriptSize = ScriptSizeCurve (toCurve parseCurveScriptSize)
      , curveExUnits = ExUnitsCurve (toCurve parseCurveExUnits)
      , curveTxValue = TxValueCurve (toCurve parseCurveTxValue)
      }

toCurve :: ParseCurve -> Curve
toCurve = \case
  ConstantP value -> Constant value
  PiecewiseLinearP points -> PiecewiseLinear (toCurvePoint <$> points)

toCurvePoint :: ParseCurvePoint -> CurvePoint
toCurvePoint point =
  CurvePoint
    { curveP = point.parseCurveP
    , curveValue = point.parseCurveValue
    }

toArrivalProcess :: ParseArrivalProcess -> ArrivalProcess
toArrivalProcess = \case
  ModerateLoadP -> moderateLoad
  CongestedLoadP -> congestedLoad
  BurstLoadPresetP -> burstLoad
  SevereCongestionLoadP -> severeCongestionLoad
  ConstantLoadP rate -> ConstantLoad rate
  BurstLoadP bursts -> BurstLoad (toBurst <$> bursts)

toBurst :: ParseBurst -> Burst
toBurst burst =
  Burst
    { baseRate = burst.parseBaseRate
    , burstRate = burst.parseBurstRate
    , burstStart = SlotNo burst.parseBurstStart
    , burstEnd = SlotNo burst.parseBurstEnd
    , burstEffect = toBurstEffect burst.parseBurstEffect
    }

toBurstEffect :: ParseBurstEffect -> BurstEffect
toBurstEffect effect =
  BurstEffect
    { valueMultiplier = effect.parseValueMultiplier
    , urgencyMultiplier = effect.parseUrgencyMultiplier
    }

toActors :: [ParseActorPopulation] -> Either ParseError [Actor]
toActors populations = do
  profiles <- concat <$> traverse expandActorPopulation populations
  case zipWith actorWithId [0 ..] profiles of
    [] -> Left (InvalidActorConfig "at least one actor must be configured")
    actors -> Right actors
 where
  actorWithId actorId profile =
    Actor profile (ActorId actorId)

expandActorPopulation :: ParseActorPopulation -> Either ParseError [ActorProfile]
expandActorPopulation population
  | population.parseActorCount <= 0 =
      Left (InvalidActorConfig "actor population count must be positive")
  | otherwise =
      Right (replicate population.parseActorCount (toActorProfile population.parseActorProfile))

toActorProfile :: ParseActorProfile -> ActorProfile
toActorProfile = \case
  HonestProfileP policy -> Honest (toActorPolicy policy)

toActorPolicy :: ParseActorPolicy -> ActorPolicy
toActorPolicy policy =
  ActorPolicy
    { actorFeeBuffer = policy.parseActorFeeBuffer
    , actorMinValueFeeMultiple = policy.parseActorMinValueFeeMultiple
    }

toLaneLatencyEstimate :: ParseLaneLatencyEstimate -> LaneLatencyEstimate
toLaneLatencyEstimate estimate =
  LaneLatencyEstimate
    { expectedStandardLatency = Duration estimate.parseExpectedStandardLatency
    , expectedPriorityLatency = Duration estimate.parseExpectedPriorityLatency
    }

toDesignLaneStructure :: ParseLaneStructure -> Design.LaneStructure
toDesignLaneStructure pls =
  case pls of
    OneP -> Design.One
    TwoP -> Design.Two

toDesignPricing :: ParseLanePricing -> Design.LanePricing
toDesignPricing plp = case plp of
  NoDynamicP -> Design.NoDynamic
  StandardOnlyDynamicP -> Design.StandardOnlyDynamic
  PriorityOnlyDynamicP -> Design.PriorityOnlyDynamic
  BothDynamicP -> Design.BothDynamic

toDesignReservationPolicy :: ParseReservationPolicy -> Design.ReservationPolicy
toDesignReservationPolicy = \case
  PriorityReservationRb bytes -> Design.PriorityReservationRb bytes
  NoReservation -> Design.NoReservation

toDesignSelectionPolicy :: ParseSelectionPolicy -> Design.SelectionPolicy
toDesignSelectionPolicy = \case
  FifoP -> Design.Fifo
  PriorityFirstP -> Design.PriorityFirst
  FifoWithStandardCapP standardCap -> Design.FifoWithStandardCap standardCap

toDesignFeeSemantics :: ParseFeeSemantics -> Design.FeeSemantics
toDesignFeeSemantics = \case
  FixedFeeP -> Design.FixedFee
  Eip1559P -> Design.Eip1559
  HonourSubmissionQuoteForP durationSlots -> Design.HonourSubmissionQuoteFor (Duration durationSlots)

toDesignControllerConfig :: ParseControllerConfig -> Design.ControllerConfig
toDesignControllerConfig controllers =
  Design.ControllerConfig
    { Design.standardController = toEip1559Controller <$> controllers.parseStandardController
    , Design.priorityController = toEip1559Controller <$> controllers.parsePriorityController
    , Design.multiplierFloor = controllers.parseMultiplierFloor
    , Design.absoluteCoeffFloor = controllers.parseAbsoluteCoeffFloor
    }

toEip1559Controller :: ParseEip1559Controller -> Design.Eip1559Controller
toEip1559Controller controller =
  Design.Eip1559Controller
    { Design.controllerTargetUtilisation = controller.parseControllerTargetUtilisation
    , Design.controllerMaxChangeDenominator = controller.parseControllerMaxChangeDenominator
    , Design.controllerInitialCoefficient = controller.parseControllerInitialCoefficient
    , Design.controllerSignal = toControllerSignal controller.parseControllerSignal
    }

toControllerSignal :: ParseControllerSignal -> Design.ControllerSignal
toControllerSignal = \case
  CapacityWeightedWindowP windowSize -> Design.CapacityWeightedWindow windowSize
  PriorityReservationUtilP -> Design.PriorityReservationUtil

data ParseError
  = MismatchedLaneSemantics String
  | InvalidActorConfig String
  deriving stock (Eq, Show)
