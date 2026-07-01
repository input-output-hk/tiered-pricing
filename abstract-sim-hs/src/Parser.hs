{- | Loading and validating config files.

The JSON shapes of the domain types are owned by their modules ("Design",
"Curve", "Load", "Retry", "Actor") as 'Data.Aeson.FromJSON' instances; this
module keeps only the genuinely file-level concerns: the sim-config
envelope with its d\/D alias and retry default, actor-population expansion,
and cross-field design validation.
-}
module Parser (
  RawSimConfig (..),
  ActorPopulation (..),
  ParseError (..),
  fromRawSimConfig,
  validateDesign,
  parseSimConfig,
  parseDesign,
) where

import Actor (Actor (..), ActorId (..), ActorType, LaneLatencyEstimate)
import Config (SimConfig (..))
import Curve (Curves)
import Data.Aeson (FromJSON (..), eitherDecode, withObject, (.:), (.:?))
import Data.ByteString.Lazy qualified as B
import Data.List.NonEmpty (NonEmpty, nonEmpty)
import Data.Maybe (fromMaybe)
import Design (ControllerConfig (..), Design (..), LaneStructure (..), ReservationPolicy (..), SelectionPolicy (..))
import Load (ArrivalProcess)
import Retry (RetryPolicy, noRetries)
import Types (PerLane (..))

{- | The sim-config file in raw form: domain values throughout, plus the few
file-level concerns — the d\/D alias, the optional retry policy, actor
populations awaiting expansion — that 'fromRawSimConfig' resolves.
-}
data RawSimConfig = RawSimConfig
  { rawDesign :: Design
  , rawCurves :: Curves
  , rawF :: Double
  , rawD :: Int
  , rawLoad :: ArrivalProcess
  , rawActors :: [ActorPopulation]
  , rawRbTxBytesCap :: Int
  , rawRbExUnitsCap :: Int
  , rawEbTxBytesCap :: Int
  , rawEbStructureBytesCap :: Int
  , rawEbExUnitsCap :: Int
  , rawMempoolBytesCap :: Int
  , rawAdmissionHeadroomUpdates :: Int
  , rawLaneLatencyEstimate :: LaneLatencyEstimate
  , rawPriceConvergenceBandPct :: Double
  , rawRetryPolicy :: Maybe RetryPolicy
  }

instance FromJSON RawSimConfig where
  parseJSON =
    withObject "RawSimConfig" \obj -> do
      d <- parseD obj
      RawSimConfig
        <$> obj .: "design"
        <*> obj .: "curves"
        <*> obj .: "f"
        <*> pure d
        <*> obj .: "load"
        <*> obj .: "actors"
        <*> obj .: "rbTxBytesCap"
        <*> obj .: "rbExUnitsCap"
        <*> obj .: "ebTxBytesCap"
        <*> obj .: "ebStructureBytesCap"
        <*> obj .: "ebExUnitsCap"
        <*> obj .: "mempoolBytesCap"
        <*> obj .: "admissionHeadroomUpdates"
        <*> obj .: "laneLatencyEstimate"
        <*> obj .: "priceConvergenceBandPct"
        <*> obj .:? "retryPolicy"
   where
    parseD obj = do
      md <- obj .:? "d"
      case md of
        Just d -> pure d
        Nothing -> obj .: "D"

-- | A count of identical actors; expansion assigns the ids.
data ActorPopulation = ActorPopulation
  { populationCount :: Int
  , populationType :: ActorType
  , populationFeeBuffer :: Double
  , populationMinValueFeeMultiple :: Double
  , populationValueMultiplier :: Double
  , populationUrgencyMultiplier :: Double
  }

instance FromJSON ActorPopulation where
  parseJSON =
    withObject "ActorPopulation" \obj ->
      ActorPopulation
        <$> obj .: "count"
        <*> obj .: "type"
        <*> obj .: "feeBuffer"
        <*> obj .: "minValueFeeMultiple"
        <*> optionalMultiplier obj "valueMultiplier"
        <*> optionalMultiplier obj "urgencyMultiplier"
   where
    optionalMultiplier obj field =
      fromMaybe 1.0 <$> obj .:? field

parseSimConfig :: FilePath -> IO SimConfig
parseSimConfig =
  parseFileWith "sim config" fromRawSimConfig

parseDesign :: FilePath -> IO Design
parseDesign =
  parseFileWith "design" \design -> design <$ validateDesign design

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

fromRawSimConfig :: RawSimConfig -> Either ParseError SimConfig
fromRawSimConfig raw = do
  validateDesign raw.rawDesign
  simConfigActors <- toActors raw.rawActors
  pure
    SimConfig
      { simConfigDesign = raw.rawDesign
      , simConfigCurves = raw.rawCurves
      , simConfigF = raw.rawF
      , simConfigD = raw.rawD
      , simConfigLoad = raw.rawLoad
      , simConfigActors
      , simConfigRbTxBytesCap = raw.rawRbTxBytesCap
      , simConfigRbExUnitsCap = raw.rawRbExUnitsCap
      , simConfigEbTxBytesCap = raw.rawEbTxBytesCap
      , simConfigEbStructureBytesCap = raw.rawEbStructureBytesCap
      , simConfigEbExUnitsCap = raw.rawEbExUnitsCap
      , simConfigMempoolBytesCap = raw.rawMempoolBytesCap
      , simConfigAdmissionHeadroomUpdates = raw.rawAdmissionHeadroomUpdates
      , simConfigLaneLatencyEstimate = raw.rawLaneLatencyEstimate
      , simConfigPriceConvergenceBandPct = raw.rawPriceConvergenceBandPct
      , simConfigRetryPolicy = fromMaybe noRetries raw.rawRetryPolicy
      }

{- | Cross-field design rules the per-type parsers cannot see: a single-lane
structure admits no priority-lane machinery.
-}
validateDesign :: Design -> Either ParseError ()
validateDesign design =
  case design.designLaneStructure of
    Two -> Right ()
    One -> do
      case design.designReservationPolicy of
        NoReservation -> Right ()
        PriorityReservationRb{} ->
          Left (MismatchedLaneSemantics "cannot reserve priority ranking-block bytes with a single lane structure")
        PriorityReservationRbIfEbNeeded{} ->
          Left (MismatchedLaneSemantics "cannot conditionally reserve priority ranking-block bytes with a single lane structure")
      case design.designSelection of
        Fifo -> Right ()
        PriorityFirst ->
          Left (MismatchedLaneSemantics "cannot use priority-first selection with a single lane structure")
        FifoWithStandardCap{} ->
          Left (MismatchedLaneSemantics "cannot cap standard-lane FIFO selection with a single lane structure")
      case design.designControllers.laneControllers.perPriority of
        Nothing -> Right ()
        Just{} ->
          Left (MismatchedLaneSemantics "cannot configure a priority controller with a single lane structure")

toActors :: [ActorPopulation] -> Either ParseError (NonEmpty Actor)
toActors populations = do
  actorTemplates <- concat <$> traverse expandActorPopulation populations
  case nonEmpty (zipWith actorWithId [0 ..] actorTemplates) of
    Nothing -> Left (InvalidActorConfig "at least one actor must be configured")
    Just actors -> Right actors
 where
  actorWithId actorId population =
    Actor
      { _actorId = ActorId actorId
      , actorType = population.populationType
      , actorFeeBuffer = population.populationFeeBuffer
      , actorMinValueFeeMultiple = population.populationMinValueFeeMultiple
      , actorValueMultiplier = population.populationValueMultiplier
      , actorUrgencyMultiplier = population.populationUrgencyMultiplier
      }

expandActorPopulation :: ActorPopulation -> Either ParseError [ActorPopulation]
expandActorPopulation population
  | population.populationCount <= 0 =
      Left (InvalidActorConfig "actor population count must be positive")
  | otherwise =
      Right (replicate population.populationCount population)

data ParseError
  = MismatchedLaneSemantics String
  | InvalidActorConfig String
  deriving stock (Eq, Show)
