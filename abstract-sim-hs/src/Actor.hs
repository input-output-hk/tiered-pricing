module Actor (
  Actor (..),
  ActorId (..),
  ActorType (..),
  Demand (..),
  LaneLatencyEstimate (..),
  Provenance (..),
  TxSubmission (..),
  generateTransaction,
  resubmitTransaction,
) where

import Curve (Curves (..), ExUnitsCurve (..), ScriptSizeCurve (..), TxSizeCurve (..), TxValueCurve (..), sampleCurve)
import Data.Aeson (ToJSON (..))
import Data.Set qualified as Set
import Design (LaneStructure (..))
import Load (BurstEffect (..))
import Pricing (Prices, quotedFeeFor)
import Transaction (Lane (..), Script (..), Tx (..), TxBody (..), TxSample (..), hash, retainedValueFor)
import Types (Duration (..), Lovelace (Lovelace), SlotNo, Urgency (..), addDurations, diffSlots, expectedBlockDelay)

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

-- | Where a generated tx comes from: a fresh demand unit, or the
-- resubmission of one whose earlier attempt failed.
data Provenance
  = FreshDemand
  | -- | origin tx number, attempt number of this submission, origin
    -- submission slot
    ResubmissionOf Int Int SlotNo
  deriving (Eq, Show)

{- | The payload of a demand unit: what the submitter wants on-chain,
independent of any one attempt's pricing. Resubmissions re-quote the fee but
never resample the payload.
-}
data Demand = Demand
  { demandValue :: Lovelace
  , demandUrgency :: Urgency
  , demandSize :: Int
  , demandScript :: Script
  }

generateTransaction :: LaneStructure -> Int -> Double -> SlotNo -> Actor -> Prices -> LaneLatencyEstimate -> Curves -> TxSample -> Maybe BurstEffect -> Maybe Tx
generateTransaction laneStructure counter f slot actor prices latencyEstimate (Curves{..}) (TxSample{..}) burstEffect =
  submitDemand laneStructure FreshDemand counter f slot actor prices latencyEstimate (Duration 0) actor.actorFeeBuffer demand
 where
  demand =
    Demand
      { demandValue = txValue
      , demandUrgency = urgency
      , demandSize = sampleTxSize curveTxSize
      , demandScript =
          Script
            { _scriptSize = sampleScriptSize curveScriptSize
            , _scriptExUnits = sampleExUnits curveExUnits
            }
      }
  (valueBurstMultiplier, urgencyBurstMultiplier) = case burstEffect of
    Just be -> (be.valueMultiplier, be.urgencyMultiplier)
    Nothing -> (1, 1)
  txValue =
    scaleLovelace (actor.actorValueMultiplier * valueBurstMultiplier) $
      Lovelace (sampleTxValue curveTxValue)
  urgency =
    scaleUrgency (actor.actorUrgencyMultiplier * urgencyBurstMultiplier) $
      sampleUrgency sampleUrgencyP
  sampleTxSize (TxSizeCurve c) = round (sampleCurve c sampleTxSizeP)
  sampleScriptSize (ScriptSizeCurve c) = round (sampleCurve c sampleScriptSizeP)
  sampleExUnits (ExUnitsCurve c) = round (sampleCurve c sampleExUnitsP)
  sampleTxValue (TxValueCurve c) = round (sampleCurve c sampleTxValueP)

{- | Resubmit a failed demand unit: same payload, fee re-quoted at current
prices with the (possibly escalated) buffer, and the lane\/utility decision
re-run with the time already waited counted against the retained value — when
congestion has eaten the surplus, the demand exits ('Nothing').
-}
resubmitTransaction :: LaneStructure -> Int -> Int -> SlotNo -> Int -> Double -> SlotNo -> Actor -> Prices -> LaneLatencyEstimate -> Double -> Demand -> Maybe Tx
resubmitTransaction laneStructure origin attempt originSubmitted counter f slot actor prices latencyEstimate escalationFactor demand =
  submitDemand laneStructure (ResubmissionOf origin attempt originSubmitted) counter f slot actor prices latencyEstimate alreadyElapsed escalatedBuffer demand
 where
  alreadyElapsed = diffSlots slot originSubmitted
  escalatedBuffer = actor.actorFeeBuffer * escalationFactor ^ max 0 (attempt - 1)

-- | The shared submission core: decide the lane (or decline), quote, post.
submitDemand :: LaneStructure -> Provenance -> Int -> Double -> SlotNo -> Actor -> Prices -> LaneLatencyEstimate -> Duration -> Double -> Demand -> Maybe Tx
submitDemand laneStructure provenance counter f slot actor prices latencyEstimate alreadyElapsed feeBuffer demand = do
  lane <- case laneStructure of
    One -> Just Standard
    Two -> chooseLane actor f latencyEstimate alreadyElapsed demand.demandUrgency demand.demandValue standardFee priorityFee
  let quotedFee = quotedFeeFor prices lane demand.demandSize demand.demandScript
      txBody =
        TxBody
          { _txSize = demand.demandSize
          , _txScript = demand.demandScript
          , _txDependsOn = Set.empty
          , _txFee = scaleLovelace feeBuffer quotedFee
          , _txNumber = counter
          }
  pure
    Tx
      { txId = hash txBody
      , txBody = txBody
      , txSubmitted = slot
      , txValue = demand.demandValue
      , txUrgency = demand.demandUrgency
      , txLane = lane
      , txOriginNumber =
          case provenance of
            FreshDemand -> counter
            ResubmissionOf origin _ _ -> origin
      , txAttempt =
          case provenance of
            FreshDemand -> 1
            ResubmissionOf _ attempt _ -> attempt
      , txOriginSubmitted =
          case provenance of
            FreshDemand -> slot
            ResubmissionOf _ _ originSubmitted -> originSubmitted
      }
 where
  standardFee = quotedFeeFor prices Standard demand.demandSize demand.demandScript
  priorityFee = quotedFeeFor prices Priority demand.demandSize demand.demandScript

{- | Lane choice by expected utility. @alreadyElapsed@ is the time the demand
unit has waited across earlier attempts (zero for fresh demand): retained
value decays over elapsed wait plus the expected latency ahead, so demand
whose surplus congestion has already consumed declines to resubmit.
-}
chooseLane :: Actor -> Double -> LaneLatencyEstimate -> Duration -> Urgency -> Lovelace -> Lovelace -> Lovelace -> Maybe Lane
chooseLane actor f latencyEstimate alreadyElapsed urgency value standardFee priorityFee
  | actor.actorType == Patient = Just Standard
  | actor.actorType == Impatient = Just Priority
  | priorityUtility > standardUtility && priorityUtility >= 0 = Just Priority
  | standardUtility >= 0 = Just Standard
  | priorityUtility >= 0 = Just Priority
  | otherwise = Nothing
 where
  retainedValueAfter latency =
    retainedValueFor (expectedBlockDelay f (addDurations alreadyElapsed latency)) urgency value

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
