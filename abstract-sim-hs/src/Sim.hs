{-# LANGUAGE DerivingStrategies #-}
{-# LANGUAGE GeneralizedNewtypeDeriving #-}

module Sim (
  SimM (..),
  SimSt (..),
  initSimSt,
  step,
) where

import Actor (Actor (..), ActorId, SubmissionEnv (..), TxSubmission (TxSubmission), generateTransaction, resubmitTransaction)
import Block (BlockSummary (..), EbId, EndorserBlock (..), InclusionPoint (..), PendingEb (..), mkBlockUsage, nextEbId, prioritySignalCapacity, selectRbTxs, selectTxsByPolicy)
import Config (SimConfig (..))
import Control.Monad (join, replicateM)
import Control.Monad.Reader (MonadReader (..), ReaderT, asks)
import Control.Monad.State.Strict (MonadState (..), State, gets, modify')
import Data.Either (partitionEithers)
import Data.Foldable (Foldable (fold, toList), traverse_)
import Data.List.NonEmpty (NonEmpty (..))
import Data.List.NonEmpty qualified as NE
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Maybe (catMaybes, mapMaybe)
import Data.Sequence (Seq, singleton, (><), (|>))
import Data.Sequence qualified as Seq
import Data.Set qualified as Set
import Design (Design (..), FeeSemantics)
import Event (SimEvent (..))
import Load (arrivalRateAt, tryBurstEffectAt)
import Data.List (sortOn)
import Mempool (Mempool (..), admitToMempool, emptyMempool, removeFromMempool, setMempoolTxs)
import Pricing (Prices (..), admissionRequiredFee, coversProducerHeadroom, feeStillValid, initialPrices, quotedFee, realisedFee, retentionWindow, updatePrices)
import Resource (Bytes (..), ExUnits (..), Resources (..))
import Retry (PendingRetry (..), RetryPolicy (..), capture)
import System.Random (StdGen, uniformR)
import Transaction (EvictionReason (..), Provenance (..), RejectReason (..), Tx (..), TxBody (..), TxId (..), TxSample (..))
import Types (Duration (Duration), Lovelace (..), SlotNo (SlotNo), addDuration, diffSlots)

data SimSt = SimSt
  { _simMempool :: Mempool
  , _simActors :: NonEmpty Actor
  , _simEbs :: Map EbId EndorserBlock
  , _simPendingEb :: Maybe PendingEb
  , _simTxs :: Map TxId Tx
  , _simSlot :: SlotNo
  , _simRng :: StdGen
  , _simPrices :: Prices
  , _simRecentBlocks :: Seq BlockSummary
  , _simTxCounter :: Int
  , _simTxActors :: Map TxId ActorId
  {- ^ submitting actor of every admitted tx; 'Retry.capture' needs it to
  attribute evictions (rejected txs never enter, their actor travels in
  the same slot's TxSubmitted event)
  -}
  , _simRetryQueue :: Map SlotNo (Seq PendingRetry)
  -- ^ pending resubmissions keyed by wake slot (jitter already drawn);
  -- within a slot, enqueue order is preserved
  }

newtype SimM a = SimM {unSimM :: ReaderT SimConfig (State SimSt) a}
  deriving newtype (Functor, Applicative, Monad, MonadReader SimConfig, MonadState SimSt)

data AdmissionCheck a
  = AdmissionRejected (NonEmpty RejectReason)
  | AdmissionAccepted a

instance Functor AdmissionCheck where
  fmap f (AdmissionAccepted a) = AdmissionAccepted (f a)
  fmap _ (AdmissionRejected reasons) = AdmissionRejected reasons

instance Applicative AdmissionCheck where
  pure = AdmissionAccepted

  AdmissionAccepted f <*> AdmissionAccepted a = AdmissionAccepted (f a)
  AdmissionRejected reasons1 <*> AdmissionRejected reasons2 = AdmissionRejected (reasons1 <> reasons2)
  AdmissionRejected reasons <*> AdmissionAccepted _ = AdmissionRejected reasons
  AdmissionAccepted _ <*> AdmissionRejected reasons = AdmissionRejected reasons

initSimSt :: SimConfig -> StdGen -> SimSt
initSimSt conf rng =
  SimSt
    { _simMempool = emptyMempool
    , _simActors = conf.simConfigActors
    , _simEbs = mempty
    , _simPendingEb = Nothing
    , _simTxs = mempty
    , _simSlot = SlotNo 0
    , _simRng = rng
    , _simPrices = initialPrices conf.simConfigDesign
    , _simRecentBlocks = mempty
    , _simTxCounter = 0
    , _simTxActors = mempty
    , _simRetryQueue = mempty
    }

step :: SimM (Seq SimEvent)
step = do
  (retries, retryEvents) <- retryStep
  txs <- actorStep
  submittedEvents <- traverse admitTxSubmission (txs <> retries)
  blockEvents <- blockStep
  recordBlockEvents blockEvents
  priceEvents <-
    if any isBlockProduced blockEvents
      then priceStep
      else pure mempty
  let slotEvents = retryEvents <> join (Seq.fromList submittedEvents) <> blockEvents <> priceEvents
  abandonEvents <- captureRetries slotEvents
  advanceSlot
  pure (slotEvents <> abandonEvents)

admitTxSubmission :: TxSubmission -> SimM (Seq SimEvent)
admitTxSubmission (TxSubmission actorId tx) = do
  simSt <- get
  prices <- gets _simPrices
  design <- asks simConfigDesign
  headroomUpdates <- asks simConfigAdmissionHeadroomUpdates
  let requiredFee =
        admissionRequiredFee design.designControllers headroomUpdates design.designFeeSemantics prices tx
  mempool <- gets _simMempool
  simTxs <- gets _simTxs
  mempoolBytesCap <- asks simConfigMempoolBytesCap
  case validateAdmission mempoolBytesCap mempool tx requiredFee of
    AdmissionRejected reasons ->
      pure $ singleton (TxSubmitted simSt._simSlot actorId tx) |> TxRejected simSt._simSlot tx.txId reasons
    AdmissionAccepted () -> do
      let newMempool = admitToMempool mempool tx
          newSimTxs = Map.insert tx.txId tx simTxs
      modify' \st ->
        st
          { _simMempool = newMempool
          , _simTxs = newSimTxs
          , _simTxActors = Map.insert tx.txId actorId st._simTxActors
          }
      slot <- gets _simSlot
      pure $ singleton (TxSubmitted simSt._simSlot actorId tx) |> TxAdmitted slot tx.txId

validateAdmission :: Int -> Mempool -> Tx -> Lovelace -> AdmissionCheck ()
validateAdmission mempoolBytesCap mempool tx requiredFee =
  (\_ _ -> ())
    <$> checkFee tx requiredFee
    <*> checkMempoolBytes mempoolBytesCap mempool tx

checkFee :: Tx -> Lovelace -> AdmissionCheck ()
checkFee tx requiredFee
  | tx.txBody._txFee < requiredFee = reject (FeeTooLow tx.txBody._txFee requiredFee)
  | otherwise = AdmissionAccepted ()

checkMempoolBytes :: Int -> Mempool -> Tx -> AdmissionCheck ()
checkMempoolBytes mempoolBytesCap mempool tx
  | currentMempoolBytes + txBytes > mempoolBytesCap =
      reject (MempoolFull currentMempoolBytes txBytes mempoolBytesCap)
  | otherwise = AdmissionAccepted ()
 where
  currentMempoolBytes = mempool.mempoolBytes
  txBytes = tx.txBody._txSize

reject :: RejectReason -> AdmissionCheck ()
reject reason = AdmissionRejected (reason :| [])

{- | The submission environment for the current slot, shared by fresh
submissions ('actorStep') and retries ('retryStep').
-}
submissionEnv :: SimM SubmissionEnv
submissionEnv = do
  design <- asks simConfigDesign
  f <- asks simConfigF
  slot <- gets _simSlot
  prices <- gets _simPrices
  latencyEstimate <- asks simConfigLaneLatencyEstimate
  pure
    SubmissionEnv
      { envLaneStructure = design.designLaneStructure
      , envF = f
      , envSlot = slot
      , envPrices = prices
      , envLatency = latencyEstimate
      }

actorStep :: SimM [TxSubmission]
actorStep = do
  slot <- gets _simSlot
  load <- asks simConfigLoad
  let burstEffect = tryBurstEffectAt load slot
  n <- sampleArrivalCount (arrivalRateAt load slot)
  actors <- gets _simActors
  curves <- asks simConfigCurves
  env <- submissionEnv
  catMaybes <$> replicateM n do
    actor <- pickActor actors
    txSample <- drawTxSample
    modify' \st -> st{_simTxCounter = st._simTxCounter + 1}
    c <- gets _simTxCounter
    pure (TxSubmission actor._actorId <$> generateTransaction env c actor curves txSample burstEffect)

{- | Fire due resubmissions: drain the queue, let each demand unit's actor
re-decide at current prices (with the time already waited counted against its
retained value), and emit 'TxAbandoned' for demand that declines — the moment
its remaining value is definitively lost.
-}
retryStep :: SimM ([TxSubmission], Seq SimEvent)
retryStep = do
  now <- gets _simSlot
  (dueQueue, rest) <- gets (Map.spanAntitone (<= now) . _simRetryQueue)
  let ready = toList (fold dueQueue)
  modify' \st -> st{_simRetryQueue = rest}
  policy <- asks simConfigRetryPolicy
  actorsById <- gets (actorMap . _simActors)
  env <- submissionEnv
  outcomes <-
    traverse (fire env policy.retryEscalationFactor actorsById now) ready
  let (abandoned, submissions) = partitionEithers outcomes
  pure (submissions, Seq.fromList abandoned)
 where
  actorMap actors =
    Map.fromList [(actor._actorId, actor) | actor <- toList actors]

  fire env escalation actorsById now pending = do
    modify' \st -> st{_simTxCounter = st._simTxCounter + 1}
    c <- gets _simTxCounter
    -- A dangling ActorId is an engine invariant breach, not an economic
    -- decline; it must not masquerade as a TxAbandoned in the trace.
    let actor =
          case Map.lookup pending.actorId actorsById of
            Just found -> found
            Nothing -> error ("retryStep: unknown " <> show pending.actorId)
        provenance =
          ResubmissionOf pending.originalTxNumber pending.attemptNumber pending.submittedAt
    pure case resubmitTransaction env provenance c escalation actor pending.demand of
      Just tx -> Right (TxSubmission pending.actorId tx)
      Nothing -> Left (TxAbandoned now pending.originalTxNumber)

{- | Distil this slot's events into queued resubmissions: 'Retry.capture'
decides (pure) what comes back and when, and this shell does the things only
the engine can — draw each entry's jitter from the seeded RNG, enqueue at the
computed wake slot, and emit 'TxAbandoned' for demand units whose failure was
terminal at capture (Abandon policy or attempt cap), so the trace records
every demand unit's end.
-}
captureRetries :: Seq SimEvent -> SimM (Seq SimEvent)
captureRetries events = do
  policy <- asks simConfigRetryPolicy
  actors <- gets _simTxActors
  txs <- gets _simTxs
  let (pendings, abandonedOrigins) = capture policy actors txs events
  traverse_ enqueue pendings
  slot <- gets _simSlot
  pure (Seq.fromList (fmap (TxAbandoned slot) abandonedOrigins))
 where
  enqueue pending = do
    let Duration jitterWindow = pending.retryJitter
    jitter <- draw (uniformR (0, jitterWindow))
    let wakeAt =
          addDuration (Duration jitter) (addDuration pending.retryDelay pending.failedAt)
    modify' \st ->
      st{_simRetryQueue = Map.insertWith (flip (<>)) wakeAt (singleton pending) st._simRetryQueue}

pickActor :: NonEmpty Actor -> SimM Actor
pickActor actors = do
  i <- draw (uniformR (0, NE.length actors - 1))
  pure (actors NE.!! i)

priceStep :: SimM (Seq SimEvent)
priceStep = do
  design <- asks simConfigDesign
  recentBlocks <- gets _simRecentBlocks
  prices <- gets _simPrices
  slot <- gets _simSlot
  let (newPrices, updates) = updatePrices design recentBlocks prices
  modify' \st -> st{_simPrices = newPrices}
  pure $ Seq.fromList (fmap (PriceUpdated slot) updates)

recordBlockEvents :: Seq SimEvent -> SimM ()
recordBlockEvents events =
  case blockSummaries events of
    Seq.Empty -> pure ()
    summaries -> do
      -- Only the controllers read '_simRecentBlocks', and none looks past
      -- its largest signal window, so keep exactly that many summaries to
      -- bound memory and the per-update scan cost.
      retain <- asks (retentionWindow . (.designControllers) . simConfigDesign)
      modify' \st ->
        let kept = st._simRecentBlocks <> summaries
         in st{_simRecentBlocks = Seq.drop (Seq.length kept - retain) kept}

blockSummaries :: Seq SimEvent -> Seq BlockSummary
blockSummaries events =
  Seq.fromList (mapMaybe blockSummary (toList events))

blockSummary :: SimEvent -> Maybe BlockSummary
blockSummary (BlockProduced _ summary) = Just summary
blockSummary _ = Nothing

isBlockProduced :: SimEvent -> Bool
isBlockProduced BlockProduced{} = True
isBlockProduced _ = False

advanceSlot :: SimM ()
advanceSlot = modify' \st ->
  st{_simSlot = addDuration (Duration 1) st._simSlot}

{- | Poisson arrival count with mean @rate@: the number of unit-exponential
inter-arrival times that fit in a window of length @rate@. The summed
log-space form stays numerically safe at any rate, unlike Knuth's
product-of-uniforms. Arrivals were previously @floor rate@ + Bernoulli — the
same mean but near-zero variance, which understated congestion burstiness
and fed the price controllers an unnaturally clean demand signal.
-}
sampleArrivalCount :: Double -> SimM Int
sampleArrivalCount rate
  | rate <= 0 = pure 0
  | otherwise = go 0 0
 where
  go count elapsed = do
    u <- draw (uniformR (0, 1))
    let elapsed' = elapsed - log (max 1e-300 u)
    if elapsed' >= rate
      then pure count
      else go (count + 1) elapsed'

drawTxSample :: SimM TxSample
drawTxSample =
  TxSample
    <$> draw (uniformR (0, 1))
    <*> draw (uniformR (0, 1))
    <*> draw (uniformR (0, 1))
    <*> draw (uniformR (0, 1))
    <*> draw (uniformR (0, 1))

draw :: (StdGen -> (a, StdGen)) -> SimM a
draw f = do
  g <- gets _simRng
  let (x, g') = f g
  modify' \s -> s{_simRng = g'}
  pure x

roll :: Double -> SimM Bool
roll p = (< p) <$> draw (uniformR (0, 1))

rollRbProduction :: SimM Bool
rollRbProduction = do
  f <- asks simConfigF
  roll f

blockStep :: SimM (Seq SimEvent)
blockStep = do
  produceRb <- rollRbProduction
  if produceRb
    then produceRankingBlock
    else pure mempty

{- | One slot's block production: certify or replace the pending EB with a
Praos RB, then announce a fresh EB over what remains in the mempool. The
mempool travels through '_simMempool' alone: each producer reads it, sweeps
or selects, and writes back, so the next stage always starts from the
previous stage's result.
-}
produceRankingBlock :: SimM (Seq SimEvent)
produceRankingBlock = do
  pendingEb <- gets _simPendingEb
  rbCapacity <- rbCapacityFromConfig
  design <- asks simConfigDesign
  slot <- gets _simSlot
  let rbSignalCapacity = prioritySignalCapacity design.designReservationPolicy rbCapacity
  rbEvents <- case pendingEb of
    Just pending -> do
      certEb <- ebCertifiedAt slot pending
      ebValid <- pendingEbStillValid design slot pending
      modify' \st -> st{_simPendingEb = Nothing}
      if certEb && ebValid
        then certifyPendingEb slot rbSignalCapacity pending
        else producePraosBlock design slot rbCapacity
    Nothing ->
      producePraosBlock design slot rbCapacity
  ebEvents <- announceEndorserBlock slot rbSignalCapacity
  pure (rbEvents >< ebEvents)

rbCapacityFromConfig :: SimM Resources
rbCapacityFromConfig =
  Resources
    <$> (Bytes <$> asks simConfigRbTxBytesCap)
    <*> (ExUnits <$> asks simConfigRbExUnitsCap)

ebCapacityFromConfig :: SimM Resources
ebCapacityFromConfig =
  Resources
    <$> (Bytes <$> asks simConfigEbTxBytesCap)
    <*> (ExUnits <$> asks simConfigEbExUnitsCap)

-- | P(EB certifies) = (1 - f)^(D - 1)
ebCertifiedAt :: SlotNo -> PendingEb -> SimM Bool
ebCertifiedAt slot pending = do
  d <- asks simConfigD
  pure (diffSlots slot pending.pendingEbAnnounced >= Duration d)

{- | An EB containing any stale tx fails certification validation outright;
it is discarded like a timing failure. Its txs were never removed from the
mempool, where the stale ones are evicted by the normal block-construction
sweep ('evictStaleFees').
-}
pendingEbStillValid :: Design -> SlotNo -> PendingEb -> SimM Bool
pendingEbStillValid design slot pending = do
  ebs <- gets _simEbs
  prices <- gets _simPrices
  pure
    ( all
        (feeStillValid design.designFeeSemantics slot prices)
        (maybe [] _ebTxs (Map.lookup pending.pendingEbId ebs))
    )

certifyPendingEb :: SlotNo -> Resources -> PendingEb -> SimM (Seq SimEvent)
certifyPendingEb slot rbSignalCapacity pending = do
  ebCapacity <- ebCapacityFromConfig
  ebs <- gets _simEbs
  prices <- gets _simPrices
  design <- asks simConfigDesign
  mempool <- gets _simMempool
  -- 'pendingEbStillValid' has already vouched for every tx in the EB, so
  -- the whole announced payload is included as-is. Inclusion events are
  -- emitted in ascending-id order, matching the historical iteration order
  -- of the certified tx set.
  let ebTxs = sortOn (.txId) (maybe [] _ebTxs (Map.lookup pending.pendingEbId ebs))
      mempool' = removeFromMempool (Set.fromList (fmap (.txId) ebTxs)) mempool
      certifiedEbSummary =
        EbCertified pending.pendingEbId (mkBlockUsage ebCapacity rbSignalCapacity ebTxs)
      events =
        Seq.fromList
          ( BlockProduced slot (RbCertifying pending.pendingEbId)
              : BlockProduced slot certifiedEbSummary
              : fmap (includedEvent design prices slot (IncludedInEb pending.pendingEbId)) ebTxs
          )
  modify' \st -> st{_simMempool = mempool'}
  pure events

producePraosBlock :: Design -> SlotNo -> Resources -> SimM (Seq SimEvent)
producePraosBlock design slot rbCapacity = do
  prices <- gets _simPrices
  mempool <- gets _simMempool
  let (feeCheckedMempool, evictionEvents) =
        evictStaleFees design.designFeeSemantics slot prices mempool
      (selectedTxs, remainingTxs, _usage) =
        selectRbTxs design.designSelection design.designReservationPolicy rbCapacity feeCheckedMempool.mempoolTxs
      selectedTxList = toList selectedTxs
      mempool' = setMempoolTxs remainingTxs
      summary =
        RbPraos
          (fmap (.txId) selectedTxList)
          (mkBlockUsage rbCapacity (prioritySignalCapacity design.designReservationPolicy rbCapacity) selectedTxList)
      events =
        BlockProduced slot summary
          : fmap (includedEvent design prices slot IncludedInRb) selectedTxList
  modify' \st -> st{_simMempool = mempool'}
  pure (evictionEvents >< Seq.fromList events)

{- | The realised fee is quoted at the inclusion slot, matching the staleness
check the tx just passed; 'Pricing.realisedFee' dispatches on the design's
fee semantics.
-}
includedEvent :: Design -> Prices -> SlotNo -> InclusionPoint -> Tx -> SimEvent
includedEvent design prices slot inclusionPoint tx =
  TxIncluded slot tx.txId inclusionPoint (realisedFee design.designFeeSemantics prices tx)

announceEndorserBlock :: SlotNo -> Resources -> SimM (Seq SimEvent)
announceEndorserBlock slot rbSignalCapacity = do
  ebCapacity <- ebCapacityFromConfig
  ebs <- gets _simEbs
  prices <- gets _simPrices
  design <- asks simConfigDesign
  mempool <- gets _simMempool
  let (feeCheckedMempool, evictionEvents) =
        evictStaleFees design.designFeeSemantics slot prices mempool
      -- Producer headroom ('Pricing.coversProducerHeadroom'): fill the EB
      -- only with txs that stay valid through the single price update that
      -- can fire before the certification check, so a prudent producer's
      -- EB cannot fail validation. Ineligible txs are not evicted: they
      -- stay in the mempool (RB-eligible only as the reservation policy
      -- allows) and regain EB eligibility if prices fall.
      ebEligible =
        coversProducerHeadroom design.designControllers design.designFeeSemantics prices
      (selectedTxs, _remainingTxs, _usage) =
        selectTxsByPolicy design.designSelection ebCapacity (Seq.filter ebEligible feeCheckedMempool.mempoolTxs)
      selectedTxList = toList selectedTxs
  if null selectedTxList
    then do
      modify' \st -> st{_simMempool = feeCheckedMempool}
      pure evictionEvents
    else do
      let ebId = nextEbId ebs
          eb =
            EndorserBlock
              { _ebTxs = selectedTxList
              , _ebId = ebId
              }
          summary =
            EbAnnounced ebId (mkBlockUsage ebCapacity rbSignalCapacity selectedTxList)
      modify' \st ->
        st
          { _simMempool = feeCheckedMempool
          , _simEbs = Map.insert ebId eb st._simEbs
          , _simPendingEb = Just (PendingEb ebId slot)
          }
      pure $ evictionEvents |> BlockProduced slot summary

evictStaleFees :: FeeSemantics -> SlotNo -> Prices -> Mempool -> (Mempool, Seq SimEvent)
evictStaleFees semantics slot prices mempool =
  (setMempoolTxs keptTxs, evictions)
 where
  (keptTxs, evictions) =
    foldl' checkTx (mempty, mempty) mempool.mempoolTxs

  checkTx (kept, events) tx
    | feeStillValid semantics slot prices tx =
        (kept |> tx, events)
    | otherwise =
        (kept, events |> staleFeeEviction slot prices tx)

staleFeeEviction :: SlotNo -> Prices -> Tx -> SimEvent
staleFeeEviction slot prices tx =
  TxEvicted slot tx.txId (FeeTooLowAtSelection tx.txBody._txFee (quotedFee prices tx))
