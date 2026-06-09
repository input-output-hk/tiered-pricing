module Actor (
  Actor (..),
  ActorId (..),
  ActorType (..),
  LaneLatencyEstimate (..),
  TxSubmission (..),
  generateTransaction,
) where

import Curve (Curves (..), ExUnitsCurve (..), ScriptSizeCurve (..), TxSizeCurve (..), TxValueCurve (..), sampleCurve)
import Data.Aeson (ToJSON (..))
import Data.Set qualified as Set
import Load (BurstEffect (..))
import Pricing (Prices, quotedFeeFor)
import Transaction (Lane (..), Script (..), Tx (..), TxBody (..), TxSample (..), hash, retainedValueFor)
import Types (Duration, Lovelace (Lovelace), SlotNo, Urgency (..), expectedBlockDelay)

newtype ActorId = ActorId Int deriving (Eq, Ord, Show)

instance ToJSON ActorId where
  toJSON (ActorId n) = toJSON n

-- A transaction-submitting entity
data Actor = Actor
  { _actorId :: ActorId
  , actorType :: ActorType
  , actorFeeBuffer :: Double
  , actorMinValueFeeMultiple :: Double
  , actorValueMultiplier :: Double
  , actorUrgencyMultiplier :: Double
  }
  deriving (Eq, Show)

data ActorType = Honest | Patient | Impatient
  deriving (Eq, Show)

data LaneLatencyEstimate = LaneLatencyEstimate
  { expectedStandardLatency :: Duration
  , expectedPriorityLatency :: Duration
  }
  deriving (Eq, Show)

data TxSubmission = TxSubmission {submissionActor :: ActorId, submissionTx :: Tx}

generateTransaction :: Double -> SlotNo -> Actor -> Prices -> LaneLatencyEstimate -> Curves -> TxSample -> Maybe BurstEffect -> Maybe Tx
generateTransaction f slot actor prices latencyEstimate (Curves{..}) (TxSample{..}) burstEffect = do
  lane <- chooseLane actor f latencyEstimate urgency txValue standardFee priorityFee
  let quotedFee = quotedFeeFor prices lane txSize
      txBody =
        TxBody
          { _txSize = txSize
          , _txScript =
              Script
                { _scriptSize = scriptSize
                , _scriptExUnits = exUnits
                }
          , _txDependsOn = Set.empty
          , _txFee = scaleLovelace actor.actorFeeBuffer quotedFee
          }
  pure
    Tx
      { txId = hash txBody
      , txBody = txBody
      , txSubmitted = slot
      , txValue = txValue
      , txUrgency = urgency
      , txLane = lane
      }
 where
  txSize = sampleTxSize curveTxSize
  scriptSize = sampleScriptSize curveScriptSize
  exUnits = sampleExUnits curveExUnits
  (valueBurstMultiplier, urgencyBurstMultiplier) = case burstEffect of
    Just be -> (be.valueMultiplier, be.urgencyMultiplier)
    Nothing -> (1, 1)
  txValue =
    scaleLovelace (actor.actorValueMultiplier * valueBurstMultiplier) $
      Lovelace (sampleTxValue curveTxValue)
  urgency =
    scaleUrgency (actor.actorUrgencyMultiplier * urgencyBurstMultiplier) $
      sampleUrgency sampleUrgencyP
  standardFee = quotedFeeFor prices Standard txSize
  priorityFee = quotedFeeFor prices Priority txSize
  sampleTxSize (TxSizeCurve c) = round (sampleCurve c sampleTxSizeP)
  sampleScriptSize (ScriptSizeCurve c) = round (sampleCurve c sampleScriptSizeP)
  sampleExUnits (ExUnitsCurve c) = round (sampleCurve c sampleExUnitsP)
  sampleTxValue (TxValueCurve c) = round (sampleCurve c sampleTxValueP)

chooseLane :: Actor -> Double -> LaneLatencyEstimate -> Urgency -> Lovelace -> Lovelace -> Lovelace -> Maybe Lane
chooseLane actor f latencyEstimate urgency value standardFee priorityFee
  | actor.actorType == Patient = Just Standard
  | actor.actorType == Impatient = Just Priority
  | priorityUtility > standardUtility && priorityUtility >= 0 = Just Priority
  | standardUtility >= 0 = Just Standard
  | priorityUtility >= 0 = Just Priority
  | otherwise = Nothing
 where
  retainedValueAfter latency =
    retainedValueFor (expectedBlockDelay f latency) urgency value

  standardUtility =
    lovelaceDifference
      (retainedValueAfter latencyEstimate.expectedStandardLatency)
      (scaleLovelace actor.actorMinValueFeeMultiple standardFee)

  priorityUtility =
    lovelaceDifference
      (retainedValueAfter latencyEstimate.expectedPriorityLatency)
      priorityFee

lovelaceDifference :: Lovelace -> Lovelace -> Integer
lovelaceDifference (Lovelace a) (Lovelace b) =
  a - b

scaleLovelace :: Double -> Lovelace -> Lovelace
scaleLovelace coefficient (Lovelace lovelace) =
  Lovelace (ceiling (fromInteger lovelace * coefficient))

scaleUrgency :: Double -> Urgency -> Urgency
scaleUrgency coefficient (Linear urg) =
  Linear (urg * coefficient)
scaleUrgency coefficient (Exponential urg) =
  Exponential (urg * coefficient)

sampleUrgency :: Double -> Urgency
sampleUrgency p
  | p < veryLowCutoff = Exponential 0.01
  | p < mediumCutoff = Exponential 0.04
  | p < highCutoff = Exponential 0.12
  | otherwise = Exponential 0.30
 where
  -- Rates are per expected ranking block. These preserve the old approximate
  -- half-lives after converting from slot delay with f = 0.05.
  veryLowPct = 75.0
  mediumPct = 17.0
  highPct = 6.5
  criticalPct = 1.5

  totalPct =
    veryLowPct + mediumPct + highPct + criticalPct

  probability pct =
    pct / totalPct

  veryLowCutoff =
    probability veryLowPct

  mediumCutoff =
    veryLowCutoff + probability mediumPct

  highCutoff =
    mediumCutoff + probability highPct
