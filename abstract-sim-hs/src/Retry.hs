module Retry (
  FailureResponse (..),
  RetryPolicy (..),
  PendingRetry (..),
  noRetries,
  defaultRetryPolicy,
  capture,
) where

import Actor (ActorId)
import Data.Aeson (FromJSON (..), withObject, (.:))
import Data.Either (partitionEithers)
import Json (Alt (..), taggedSum)
import Data.Foldable (toList)
import Data.List.NonEmpty qualified as NE
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Maybe (mapMaybe)
import Data.Sequence (Seq)
import Event (SimEvent (..))
import Transaction (Demand, DemandId, EvictionReason (..), RejectReason (..), Tx (..), TxId)
import Types (Duration (..), SlotNo)

-- | What happens to a demand unit after a given failure.
data FailureResponse
  = -- | the failure is final
    Abandon
  | {- | minimum delay (detection + decision time), plus a uniformly-drawn
    jitter window modelling dispersed detection: polling phase, relay
    latency, wallet attentiveness. The wake slot is
    @failure + delay + U(0, jitter)@. Without jitter, cohort failures (a
    price step evicting a whole batch at once) re-land as a synchronized
    retry wave in a single future slot.
    -}
    ResubmitAfter Duration Duration
  deriving stock (Eq, Show)

instance FromJSON FailureResponse where
  parseJSON =
    taggedSum
      "failure response"
      [ ("abandon", Nullary Abandon)
      ,
        ( "resubmit-after"
        , WithFields \obj ->
            ResubmitAfter
              <$> (Duration <$> obj .: "delaySlots")
              <*> (Duration <$> obj .: "jitterSlots")
        )
      ]

{- | How rejected and evicted demand re-enters the simulation, per failure
reason.
-}
data RetryPolicy = RetryPolicy
  { retryFeeTooLow :: FailureResponse
  {- ^ admission rejected the posted fee ('Transaction.FeeTooLow'). The
  resubmission re-quotes at then-current prices, so a short backoff
  suffices: the fresh quote is the remedy.
  -}
  , retryMempoolFull :: FailureResponse
  {- ^ admission found no space ('Transaction.MempoolFull'). The fee was
  fine, the mempool wasn't; space frees on block production, so back off
  around one expected RB interval (~1\/f slots) — immediate retries
  mostly bounce again.
  -}
  , retryEvicted :: FailureResponse
  {- ^ evicted as stale at selection ('Transaction.FeeTooLowAtSelection').
  Same remedy as a fee rejection: re-quote and resubmit.
  -}
  , retryMaxAttempts :: Int
  {- ^ hard cap on resubmissions per demand unit. The economic bound is the
  actor's utility re-check (decaying value against risen quotes); this is
  insurance against configs where that never bites.
  -}
  , retryEscalationFactor :: Double
  {- ^ extra multiplier on the actor's fee buffer per attempt (compounding).
  1.0 means no escalation beyond the re-quote itself — re-applying the
  buffer to a risen quote already escalates in absolute terms.
  -}
  }
  deriving stock (Eq, Show)

instance FromJSON RetryPolicy where
  parseJSON =
    withObject "RetryPolicy" \obj ->
      RetryPolicy
        <$> obj .: "feeTooLow"
        <*> obj .: "mempoolFull"
        <*> obj .: "evicted"
        <*> obj .: "maxAttempts"
        <*> obj .: "escalationFactor"

-- | Every failure is final; reproduces pre-retry behaviour.
noRetries :: RetryPolicy
noRetries =
  RetryPolicy
    { retryFeeTooLow = Abandon
    , retryMempoolFull = Abandon
    , retryEvicted = Abandon
    , retryMaxAttempts = 0
    , retryEscalationFactor = 1.0
    }

defaultRetryPolicy :: RetryPolicy
defaultRetryPolicy =
  RetryPolicy
    { -- first-hop rejection is fast, but mechanism-design.md models
      -- non-admission detection as observing mempools, which spreads it
      retryFeeTooLow = ResubmitAfter (Duration 2) (Duration 6)
    , -- space frees on block production; jitter avoids the whole bounced
      -- cohort re-arriving in the same slot
      retryMempoolFull = ResubmitAfter (Duration 20) (Duration 20)
    , -- eviction is unsignalled: detection is by polling or by timing out
      -- on expected inclusion, roughly an RB interval or two
      retryEvicted = ResubmitAfter (Duration 10) (Duration 30)
    , retryMaxAttempts = 5
    , -- flat re-bidding (1.0) is pure load amplification: a rejected fee
      -- resubmitted unchanged is rejected again, up to maxAttempts times
      retryEscalationFactor = 1.2
    }

data PendingRetry = PendingRetry
  { actorId :: ActorId
  , demand :: Demand
  -- ^ the payload, carried with its original undecayed value: decay is
  -- computed from 'submittedAt' at re-decision time, never baked in here
  -- (else it would compound across attempts)
  , submittedAt :: SlotNo
  -- ^ the demand unit's first submission — the value-decay anchor
  , attemptNumber :: Int
  -- ^ the attempt the resubmission will carry (failed attempt + 1)
  , originalTxNumber :: DemandId
  , failedAt :: SlotNo
  -- ^ when the failure happened; the wake slot is computed from here
  , retryDelay :: Duration
  -- ^ policy minimum delay, snapshotted at capture
  , retryJitter :: Duration
  -- ^ policy jitter window; the engine draws @U(0, jitter)@ once, at
  -- enqueue time
  }

{- | Scan one slot's events for failed demand and decide, per the policy, what
comes back: resubmissions to queue, and the origins of demand units abandoned
here (Abandon-policy failures and attempt-cap exhaustion) — the engine turns
the latter into 'Event.TxAbandoned' so every terminal failure is visible in
the trace. Pure by design: the jitter draw is the engine's job ('Sim' owns
the RNG), so each entry carries its failure slot and response and the engine
computes @failedAt + retryDelay + U(0, retryJitter)@ once, at enqueue time.
-}
capture :: RetryPolicy -> Map TxId ActorId -> Map TxId Tx -> Seq SimEvent -> ([PendingRetry], [DemandId])
capture policy actors txs events =
  (pendings, abandonedOrigins)
 where
  (abandonedOrigins, pendings) =
    partitionEithers (mapMaybe f (toList events))

  -- Rejected txs never entered the mempool, so they are absent from the
  -- engine's tx map; their bodies and actors travel in the same slot's
  -- TxSubmitted events.
  submitted =
    Map.fromList
      [ (tx.txId, (actorId, tx))
      | TxSubmitted _ actorId tx <- toList events
      ]

  f event = case event of
    TxRejected slot txId reasons -> do
      (actorId, tx) <- Map.lookup txId submitted
      pure $
        decide slot actorId tx (foldr1 combine (rejectResponse <$> NE.toList reasons))
    TxEvicted slot txId reason -> do
      tx <- Map.lookup txId txs
      actorId <- Map.lookup txId actors
      pure $ decide slot actorId tx (evictResponse reason)
    _ -> Nothing

  rejectResponse = \case
    FeeTooLow{} -> policy.retryFeeTooLow
    MempoolFull{} -> policy.retryMempoolFull

  evictResponse = \case
    FeeTooLowAtSelection{} -> policy.retryEvicted

  -- A tx rejected for several reasons at once must outlast all of them:
  -- Abandon dominates, otherwise wait out the slowest component.
  combine Abandon _ = Abandon
  combine _ Abandon = Abandon
  combine (ResubmitAfter delay1 jitter1) (ResubmitAfter delay2 jitter2) =
    ResubmitAfter (max delay1 delay2) (max jitter1 jitter2)

  decide failedSlot actorId tx = \case
    Abandon -> Left tx.txOriginNumber
    ResubmitAfter delay jitter
      | tx.txAttempt > policy.retryMaxAttempts -> Left tx.txOriginNumber
      | otherwise ->
          Right
            PendingRetry
              { actorId
              , demand = tx.txDemand
              , submittedAt = tx.txOriginSubmitted
              , attemptNumber = tx.txAttempt + 1
              , originalTxNumber = tx.txOriginNumber
              , failedAt = failedSlot
              , retryDelay = delay
              , retryJitter = jitter
              }

