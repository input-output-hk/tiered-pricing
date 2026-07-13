module Actor (
  Actor (..),
  ActorId (..),
  ActorType (..),
  LaneLatencyEstimate (..),
  SubmissionEnv (..),
  TxSubmission (..),
  generateTransaction,
  resubmitTransaction,
) where

import Curve (Curves (..), ExUnitsCurve (..), ScriptSizeCurve (..), TxSizeCurve (..), TxValueCurve (..), sampleCurve)
import Data.Aeson (FromJSON (..), ToJSON (..), withObject, (.:))
import Data.Set qualified as Set
import Json (Alt (..), taggedSum)
import Design (LaneStructure (..), PriorityPremiumScope)
import Load (BurstEffect (..))
import Pricing (Prices, quotedFeeFor, requiredMaxFeeFor)
import Transaction (Demand (..), Lane (..), Provenance (..), Script (..), Tx (..), TxBody (..), TxSample (..), hash, retainedValueFor)
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

instance FromJSON ActorType where
  parseJSON =
    taggedSum
      "actor type"
      [ ("honest", Nullary Honest)
      , ("patient", Nullary Patient)
      , ("impatient", Nullary Impatient)
      ]

data LaneLatencyEstimate = LaneLatencyEstimate
  { expectedStandardLatency :: Duration
  , expectedPriorityLatency :: Duration
  }
  deriving (Eq, Show)

instance FromJSON LaneLatencyEstimate where
  parseJSON =
    withObject "LaneLatencyEstimate" \obj ->
      LaneLatencyEstimate
        <$> (Duration <$> obj .: "expectedStandardLatency")
        <*> (Duration <$> obj .: "expectedPriorityLatency")

data TxSubmission = TxSubmission {submissionActor :: ActorId, submissionTx :: Tx}

{- | The pure environment a submission decision reads: lane structure,
premium scope, chain parameters, the clock, current prices, and latency
expectations. The engine gathers it once per slot and shares it between fresh
submissions and retries.
-}
data SubmissionEnv = SubmissionEnv
  { envLaneStructure :: LaneStructure
  , envPriorityPremiumScope :: PriorityPremiumScope
  , envF :: Double
  , envSlot :: SlotNo
  , envPrices :: Prices
  , envLatency :: LaneLatencyEstimate
  }

-- | Sample a fresh demand unit from the curves and submit its first attempt.
generateTransaction :: SubmissionEnv -> Int -> Actor -> Curves -> TxSample -> Maybe BurstEffect -> Maybe Tx
generateTransaction env counter actor (Curves{..}) (TxSample{..}) burstEffect =
  submitDemand env FreshDemand counter actor.actorFeeBuffer actor demand
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
resubmitTransaction :: SubmissionEnv -> Provenance -> Int -> Double -> Actor -> Demand -> Maybe Tx
resubmitTransaction env provenance counter escalationFactor actor demand =
  submitDemand env provenance counter escalatedBuffer actor demand
 where
  escalatedBuffer =
    case provenance of
      FreshDemand -> actor.actorFeeBuffer
      ResubmissionOf _ attempt _ ->
        actor.actorFeeBuffer * escalationFactor ^ max 0 (attempt - 1)

-- | The shared submission core: decide the lane (or decline), quote, post.
-- An rb-only priority decision and cap use the larger possible inclusion
-- quote; settlement later refunds to the quote at the actual inclusion point.
submitDemand :: SubmissionEnv -> Provenance -> Int -> Double -> Actor -> Demand -> Maybe Tx
submitDemand env provenance counter feeBuffer actor demand = do
  lane <- case env.envLaneStructure of
    One -> chooseSingleLane actor env.envF env.envLatency alreadyElapsed demand.demandUrgency demand.demandValue standardFee
    Two -> chooseLane actor env.envF env.envLatency alreadyElapsed demand.demandUrgency demand.demandValue standardFee priorityDecisionFee
  let maxFeeQuote =
        case lane of
          Standard -> standardFee
          Priority -> priorityDecisionFee
      txBody =
        TxBody
          { _txSize = demand.demandSize
          , _txScript = demand.demandScript
          , _txDependsOn = Set.empty
          , _txFee = scaleLovelace feeBuffer maxFeeQuote
          , _txNumber = counter
          }
  pure
    Tx
      { txId = hash txBody
      , txBody = txBody
      , txSubmitted = env.envSlot
      , txDemand = demand
      , txLane = lane
      , txProvenance = provenance
      }
 where
  -- Time the demand unit has already waited across earlier attempts;
  -- definitionally zero for fresh demand.
  alreadyElapsed =
    case provenance of
      FreshDemand -> Duration 0
      ResubmissionOf _ _ originSubmitted -> diffSlots env.envSlot originSubmitted
  standardFee = quotedFeeFor env.envPrices Standard demand.demandSize demand.demandScript
  priorityDecisionFee =
    requiredMaxFeeFor env.envPriorityPremiumScope env.envPrices Priority demand.demandSize demand.demandScript

{- | The single-lane analogue of 'chooseLane': an honest actor submits only
when the standard-lane fee clears its value-based reservation price, so demand
is price-elastic; the fixed-disposition actors always submit (Standard is the
only lane). Without the reservation gate, single-lane demand never sheds as the
fee rises and the EIP-1559 controller has nothing to push against.
-}
chooseSingleLane :: Actor -> Double -> LaneLatencyEstimate -> Duration -> Urgency -> Lovelace -> Lovelace -> Maybe Lane
chooseSingleLane actor f latencyEstimate alreadyElapsed urgency value standardFee
  | actor.actorType == Patient || actor.actorType == Impatient = Just Standard
  | standardLaneUtility actor f latencyEstimate alreadyElapsed urgency value standardFee >= 0 = Just Standard
  | otherwise = Nothing

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
  standardUtility =
    standardLaneUtility actor f latencyEstimate alreadyElapsed urgency value standardFee
  priorityUtility =
    lovelaceDifference
      (retainedValueFor (expectedBlockDelay f (addDurations alreadyElapsed latencyEstimate.expectedPriorityLatency)) urgency value)
      priorityFee

{- | Expected surplus on the standard lane: retained value after the standard
wait, less the fee scaled by the actor's minimum value\/fee multiple. A
non-negative result means the fee is still within the actor's value-based
reservation price; negative means the fee has outrun what the tx is worth.
-}
standardLaneUtility :: Actor -> Double -> LaneLatencyEstimate -> Duration -> Urgency -> Lovelace -> Lovelace -> Integer
standardLaneUtility actor f latencyEstimate alreadyElapsed urgency value standardFee =
  lovelaceDifference
    (retainedValueFor (expectedBlockDelay f (addDurations alreadyElapsed latencyEstimate.expectedStandardLatency)) urgency value)
    (scaleLovelace actor.actorMinValueFeeMultiple standardFee)

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
