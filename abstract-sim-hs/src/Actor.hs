module Actor (
  Actor (..),
  ActorId (..),
  ActorProfile (..),
  ActorPolicy (..),
  LaneLatencyEstimate (..),
  TxSubmission (..),
  defaultHonestPolicy,
  generateTransaction,
) where

import Curve (Curves (..), ExUnitsCurve (..), ScriptSizeCurve (..), TxSizeCurve (..), TxValueCurve (..), sampleCurve)
import Data.Set qualified as Set
import Pricing (Prices, quotedFeeFor)
import Transaction (Lane (..), Script (..), Tx (..), TxBody (..), TxSample (..), hash, retainedValueFor)
import Types (Duration, Lovelace (Lovelace), SlotNo, Urgency (..))

newtype ActorId = ActorId Int deriving (Eq, Ord, Show)

-- A transaction-submitting entity
data Actor = Actor
  { _actorProfile :: ActorProfile
  , _actorId :: ActorId
  }
  deriving (Eq, Show)

data ActorProfile = Honest ActorPolicy
  deriving (Eq, Show)

data ActorPolicy = ActorPolicy
  { actorFeeBuffer :: Double
  , actorMinValueFeeMultiple :: Double
  }
  deriving (Eq, Show)

data LaneLatencyEstimate = LaneLatencyEstimate
  { expectedStandardLatency :: Duration
  , expectedPriorityLatency :: Duration
  }
  deriving (Eq, Show)

defaultHonestPolicy :: ActorPolicy
defaultHonestPolicy =
  ActorPolicy
    { actorFeeBuffer = 1.10
    , actorMinValueFeeMultiple = 1.0
    }

data TxSubmission = TxSubmission {submissionActor :: ActorId, submissionTx :: Tx}

generateTransaction :: SlotNo -> Actor -> Prices -> LaneLatencyEstimate -> Curves -> TxSample -> Maybe Tx
generateTransaction slot (Actor (Honest policy) _) prices latencyEstimate (Curves{..}) (TxSample{..}) = do
  lane <- chooseLane policy latencyEstimate urgency txValue standardFee priorityFee
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
          , _txFee = scaleLovelace policy.actorFeeBuffer quotedFee
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
  txValue = Lovelace (sampleTxValue curveTxValue)
  urgency = sampleUrgency sampleUrgencyP
  standardFee = quotedFeeFor prices Standard txSize
  priorityFee = quotedFeeFor prices Priority txSize
  sampleTxSize (TxSizeCurve c) = round (sampleCurve c sampleTxSizeP)
  sampleScriptSize (ScriptSizeCurve c) = round (sampleCurve c sampleScriptSizeP)
  sampleExUnits (ExUnitsCurve c) = round (sampleCurve c sampleExUnitsP)
  sampleTxValue (TxValueCurve c) = round (sampleCurve c sampleTxValueP)

chooseLane :: ActorPolicy -> LaneLatencyEstimate -> Urgency -> Lovelace -> Lovelace -> Lovelace -> Maybe Lane
chooseLane policy latencyEstimate urgency value standardFee priorityFee
  | priorityUtility > standardUtility && priorityUtility >= 0 = Just Priority
  | standardUtility >= 0 = Just Standard
  | priorityUtility >= 0 = Just Priority
  | otherwise = Nothing
 where
  retainedValueAfter latency =
    retainedValueFor latency urgency value

  standardUtility =
    lovelaceDifference
      (retainedValueAfter latencyEstimate.expectedStandardLatency)
      (scaleLovelace policy.actorMinValueFeeMultiple standardFee)

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

sampleUrgency :: Double -> Urgency
sampleUrgency p
  | p < veryLowCutoff = Exponential 0.0005
  | p < mediumCutoff = Exponential 0.002
  | p < highCutoff = Exponential 0.006
  | otherwise = Exponential 0.015
 where
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
