{-# LANGUAGE DerivingStrategies #-}
{-# LANGUAGE GeneralizedNewtypeDeriving #-}

module Sim where

import Actor (Actor (..), ActorId, TxSubmission (TxSubmission), generateTransaction, resubmitTransaction)
import Block (BlockSummary (..), EbId (..), EndorserBlock (..), InclusionPoint (..), PendingEb (..), RankingBlock (..), mkEndorserBlockSummary, mkRankingBlockSummary, selectByBlockCapacity, selectByBlockCapacityFrom, selectFifoWithStandardCap, selectPriorityByBlockCapacity, selectedTxBodies)
import Chain (Chain (..), emptyChain)
import Config (SimConfig (..))
import Control.Monad (join, replicateM)
import Control.Monad.Reader (MonadReader (..), ReaderT, asks)
import Control.Monad.State.Strict (MonadState (..), State, gets, modify')
import Curve (Curve, sampleCurve)
import Data.Either (partitionEithers)
import Data.Foldable (Foldable (toList), traverse_)
import Data.List (find)
import Data.List.NonEmpty (NonEmpty (..))
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Maybe (catMaybes, mapMaybe)
import Data.Sequence (Seq, singleton, (><), (|>))
import Data.Sequence qualified as Seq
import Data.Set qualified as Set
import Design (ControllerConfig (..), ControllerSignal (..), Design (..), Eip1559Controller (..), FeeSemantics (..), ReservationPolicy (..), SelectionPolicy (..))
import Event (SimEvent (..))
import Load (arrivalRateAt, tryBurstEffectAt)
import Mempool (Mempool (..), admitToMempool, emptyMempool, removeFromMempool, setMempoolTxIds)
import Pricing (PriceUpdate (..), Prices (..), initialPrices, quotedFee, realisedFee, updatePrices, worstCaseNextPrices)
import Retry (PendingRetry (..), RetryPolicy (..), capture, due)
import System.Random (StdGen, uniformR)
import Transaction (EvictionReason (..), RejectReason (..), Tx (..), TxBody (..), TxId (..), TxSample (..))
import Types (Duration (Duration), Lovelace (..), SlotNo (SlotNo), addDuration, diffSlots)

data SimSt = SimSt
  { _simChain :: Chain
  , _simMempool :: Mempool
  , _simActors :: [Actor]
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
  , _simRetryQueue :: Seq (SlotNo, PendingRetry)
  -- ^ pending resubmissions with their wake slots, jitter already drawn
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

runSim :: Seq SimEvent -> Int -> SimM (Seq SimEvent)
runSim events 0 = pure events
runSim events numSlots = do
  simEvents <- step
  runSim (events >< simEvents) (numSlots - 1)

initSimSt :: SimConfig -> StdGen -> SimSt
initSimSt conf rng =
  SimSt
    { _simChain = emptyChain
    , _simMempool = emptyMempool
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

{- | The fee a tx must post to enter the mempool: the quote after
'simConfigAdmissionHeadroomUpdates' worst-case controller steps.

This is the admission-side complement of the EB producer headroom in
'announceEndorserBlock', and with a horizon of 1 it is deliberately
semi-redundant with it: nothing enters the mempool that a producer would
refuse to put in an EB at today's prices, so unserviceable txs are rejected
at the door (visibly, resubmittably) instead of sitting against the mempool
cap forever. The producer check is still required because prices may keep
rising while an admitted tx waits its turn — this check keeps the mempool
serviceable, that one keeps certified EBs valid. 'FixedFee' never re-prices,
so it needs no headroom.
-}
admissionRequiredFee :: Tx -> SimM Lovelace
admissionRequiredFee tx = do
  prices <- gets _simPrices
  design <- asks simConfigDesign
  headroomUpdates <- asks simConfigAdmissionHeadroomUpdates
  let admissionPrices =
        case design.designFeeSemantics of
          FixedFee -> prices
          _ ->
            iterate (worstCaseNextPrices design.designControllers) prices
              !! max 0 headroomUpdates
  pure (quotedFee admissionPrices tx)

admitTxSubmission :: TxSubmission -> SimM (Seq SimEvent)
admitTxSubmission (TxSubmission actorId tx) = do
  simSt <- get
  requiredFee <- admissionRequiredFee tx
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

actorStep :: SimM [TxSubmission]
actorStep = do
  slot <- gets _simSlot
  load <- asks simConfigLoad
  let burstEffect = tryBurstEffectAt load slot
  n <- sampleArrivalCount (arrivalRateAt load slot)
  actors <- gets _simActors
  curves <- asks simConfigCurves
  prices <- gets _simPrices
  latencyEstimate <- asks simConfigLaneLatencyEstimate
  f <- asks simConfigF
  d <- asks simConfigDesign
  catMaybes <$> replicateM n do
    actor <- pickActor actors
    txSample <- drawTxSample
    modify' \st -> st{_simTxCounter = st._simTxCounter + 1}
    c <- gets _simTxCounter
    pure (TxSubmission actor._actorId <$> generateTransaction d.designLaneStructure c f slot actor prices latencyEstimate curves txSample burstEffect)

{- | Fire due resubmissions: drain the queue, let each demand unit's actor
re-decide at current prices (with the time already waited counted against its
retained value), and emit 'TxAbandoned' for demand that declines — the moment
its remaining value is definitively lost.
-}
retryStep :: SimM ([TxSubmission], Seq SimEvent)
retryStep = do
  now <- gets _simSlot
  (ready, rest) <- gets (due now . _simRetryQueue)
  modify' \st -> st{_simRetryQueue = rest}
  design <- asks simConfigDesign
  latencyEstimate <- asks simConfigLaneLatencyEstimate
  f <- asks simConfigF
  policy <- asks simConfigRetryPolicy
  actors <- gets _simActors
  prices <- gets _simPrices
  outcomes <-
    traverse (fire design latencyEstimate f policy.retryEscalationFactor actors now prices) ready
  let (abandoned, submissions) = partitionEithers outcomes
  pure (submissions, Seq.fromList abandoned)
 where
  fire design latencyEstimate f escalation actors now prices pending = do
    modify' \st -> st{_simTxCounter = st._simTxCounter + 1}
    c <- gets _simTxCounter
    let resubmission = do
          actor <- find (\a -> a._actorId == pending.actorId) actors
          resubmitTransaction
            design.designLaneStructure
            pending.originalTxNumber
            pending.attemptNumber
            pending.submittedAt
            c
            f
            now
            actor
            prices
            latencyEstimate
            escalation
            pending.demand
    pure case resubmission of
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
    modify' \st -> st{_simRetryQueue = st._simRetryQueue |> (wakeAt, pending)}

pickActor :: [Actor] -> SimM Actor
pickActor [] = error "pickActor: no actors configured"
pickActor actors = do
  i <- draw (uniformR (0, length actors - 1))
  pure (actors !! i)

priceStep :: SimM (Seq SimEvent)
priceStep = do
  design <- asks simConfigDesign
  recentBlocks <- gets _simRecentBlocks
  prices <- gets _simPrices
  slot <- gets _simSlot
  let (newPrices, updates) = updatePrices design recentBlocks prices
  modify' \st -> st{_simPrices = newPrices}
  pure $ Seq.fromList (fmap (priceUpdateEvent slot) updates)

recordBlockEvents :: Seq SimEvent -> SimM ()
recordBlockEvents events =
  case blockSummaries events of
    Seq.Empty -> pure ()
    summaries -> do
      retain <- asks (retentionWindow . simConfigDesign)
      modify' \st ->
        let kept = st._simRecentBlocks <> summaries
         in st{_simRecentBlocks = Seq.drop (Seq.length kept - retain) kept}

{- | How far back the price controllers can read '_simRecentBlocks'. Only the
controllers consume it, and none looks past its largest window, so we keep
exactly that many summaries and drop the rest to bound memory and the
per-update scan cost.
-}
retentionWindow :: Design -> Int
retentionWindow design =
  maximum (1 : concatMap controllerWindow [controllers.standardController, controllers.priorityController])
 where
  controllers = design.designControllers
  controllerWindow Nothing = []
  controllerWindow (Just controller) =
    case controller.controllerSignal of
      CapacityWeightedWindow windowSize -> [windowSize]
      PriorityReservationUtil -> [1]

blockSummaries :: Seq SimEvent -> Seq BlockSummary
blockSummaries events =
  Seq.fromList (mapMaybe blockSummary (toList events))

blockSummary :: SimEvent -> Maybe BlockSummary
blockSummary (BlockProduced _ summary) = Just summary
blockSummary _ = Nothing

isBlockProduced :: SimEvent -> Bool
isBlockProduced BlockProduced{} = True
isBlockProduced _ = False

priceUpdateEvent :: SlotNo -> PriceUpdate -> SimEvent
priceUpdateEvent slot update =
  PriceUpdated
    slot
    update.priceUpdateLane
    update.priceUpdateOldCoeff
    update.priceUpdateNewCoeff
    update.priceUpdateUtilisation

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

drawCurve :: Curve -> SimM Double
drawCurve c = sampleCurve c <$> draw (uniformR (0, 1))

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

produceRankingBlock :: SimM (Seq SimEvent)
produceRankingBlock = do
  pendingEb <- gets _simPendingEb
  rbTxBytesCap <- asks simConfigRbTxBytesCap
  rbExUnitsCap <- asks simConfigRbExUnitsCap
  simTxs <- gets _simTxs
  design <- asks simConfigDesign
  mempool <- gets _simMempool
  slot <- gets _simSlot
  let rbPriorityTxBytesCap = priorityTxBytesCap design rbTxBytesCap
  (mempoolAfterRb, rbEvents) <- case pendingEb of
    Just pending -> do
      certEb <- ebCertifiedAt slot pending
      ebValid <- pendingEbStillValid design slot simTxs pending
      modify' \st -> st{_simPendingEb = Nothing}
      if certEb && ebValid
        then certifyPendingEb slot rbPriorityTxBytesCap rbExUnitsCap simTxs pending mempool
        else producePraosBlock design slot rbTxBytesCap rbExUnitsCap simTxs mempool
    Nothing ->
      producePraosBlock design slot rbTxBytesCap rbExUnitsCap simTxs mempool
  ebEvents <- announceEndorserBlock slot rbPriorityTxBytesCap rbExUnitsCap simTxs mempoolAfterRb
  pure (rbEvents >< ebEvents)
 where
  -- P(EB certifies) = (1 - f)^(D - 1)
  ebCertifiedAt :: SlotNo -> PendingEb -> SimM Bool
  ebCertifiedAt slot pending = do
    d <- asks simConfigD
    pure (diffSlots slot pending.pendingEbAnnounced >= Duration d)

  -- An EB containing any stale tx fails certification validation outright;
  -- it is discarded like a timing failure. Its txs were never removed from
  -- the mempool, where the stale ones are evicted by the normal
  -- block-construction sweep ('evictStaleFees').
  pendingEbStillValid :: Design -> SlotNo -> Map TxId Tx -> PendingEb -> SimM Bool
  pendingEbStillValid design slot simTxs pending = do
    ebs <- gets _simEbs
    prices <- gets _simPrices
    let txStillValid txId =
          case Map.lookup txId simTxs of
            Nothing -> True
            Just tx -> txFeeStillValid design.designFeeSemantics slot prices tx
    pure (all txStillValid (maybe mempty _ebTxs (Map.lookup pending.pendingEbId ebs)))

  appendRankingBlock :: SlotNo -> RankingBlock -> SimM ()
  appendRankingBlock slot block =
    modify' \st ->
      st
        { _simChain =
            Chain
              { _chainBlocks = st._simChain._chainBlocks |> (slot, block)
              , _chainTip = Just slot
              }
        }
  certifyPendingEb :: SlotNo -> Int -> Int -> Map TxId Tx -> PendingEb -> Mempool -> SimM (Mempool, Seq SimEvent)
  certifyPendingEb slot rbPriorityTxBytesCap rbExUnitsCap simTxs pending mempool = do
    ebTxBytesCap <- asks simConfigEbTxBytesCap
    ebExUnitsCap <- asks simConfigEbExUnitsCap
    ebs <- gets _simEbs
    prices <- gets _simPrices
    design <- asks simConfigDesign
    -- 'pendingEbStillValid' has already vouched for every tx in the EB, so
    -- the whole announced set is included as-is.
    let ebTxIdSet =
          maybe mempty _ebTxs (Map.lookup pending.pendingEbId ebs)
        ebTxs = selectedTxBodies simTxs (Seq.fromList (Set.toList ebTxIdSet))
        block = CertifyingBlock pending.pendingEbId
        mempool' = removeFromMempool simTxs ebTxIdSet mempool
        summary =
          RankingBlockProduced
            (mkRankingBlockSummary block 0 0 0 0 [])
        certifiedEbSummary =
          EndorserBlockCertified
            (mkEndorserBlockSummary pending.pendingEbId ebTxBytesCap ebExUnitsCap rbPriorityTxBytesCap rbExUnitsCap ebTxs)
        events =
          Seq.fromList
            ( BlockProduced slot summary
                : BlockProduced slot certifiedEbSummary
                : fmap (includedEvent design prices slot (IncludedInEb pending.pendingEbId)) ebTxs
            )
    appendRankingBlock slot block
    modify' \st -> st{_simMempool = mempool'}
    pure (mempool', events)

  producePraosBlock :: Design -> SlotNo -> Int -> Int -> Map TxId Tx -> Mempool -> SimM (Mempool, Seq SimEvent)
  producePraosBlock design slot rbTxBytesCap rbExUnitsCap simTxs mempool = do
    prices <- gets _simPrices
    let (feeCheckedMempool, evictionEvents) =
          evictStaleFees design.designFeeSemantics slot prices simTxs mempool
        (selectedTxs, remainingMempool, (_usedBytes, _usedExUnits)) =
          selectRankingBlockTxs design rbTxBytesCap rbExUnitsCap simTxs feeCheckedMempool.mempoolTxIds
        selectedTxIds = toList selectedTxs
        selectedTxBodyList = selectedTxBodies simTxs selectedTxs
        mempool' = setMempoolTxIds simTxs remainingMempool
        block = PraosBlock selectedTxIds
        summary =
          RankingBlockProduced
            (mkRankingBlockSummary block rbTxBytesCap rbExUnitsCap (priorityTxBytesCap design rbTxBytesCap) rbExUnitsCap selectedTxBodyList)
        events =
          BlockProduced slot summary
            : fmap (includedEvent design prices slot IncludedInRb) selectedTxBodyList
    appendRankingBlock slot block
    modify' \st -> st{_simMempool = mempool'}
    pure (mempool', evictionEvents >< Seq.fromList events)

  -- The realised fee is quoted at the inclusion slot, matching the staleness
  -- check the tx just passed; 'Pricing.realisedFee' dispatches on the
  -- design's fee semantics.
  includedEvent :: Design -> Prices -> SlotNo -> InclusionPoint -> Tx -> SimEvent
  includedEvent design prices slot inclusionPoint tx =
    TxIncluded slot tx.txId inclusionPoint (realisedFee design.designFeeSemantics prices tx)

  selectRankingBlockTxs ::
    Design ->
    Int ->
    Int ->
    Map TxId Tx ->
    Seq TxId ->
    (Seq TxId, Seq TxId, (Int, Int))
  selectRankingBlockTxs design rbTxBytesCap rbExUnitsCap simTxs mempool =
    case design.designReservationPolicy of
      -- The reservation rule admits only priority txs to RBs, so every
      -- selection policy collapses to priority-only FIFO under it.
      PriorityReservationRb reservationBytes ->
        selectPriorityByBlockCapacity
          (min rbTxBytesCap reservationBytes)
          rbExUnitsCap
          simTxs
          mempool
      NoReservation ->
        selectTxsByPolicy design.designSelection rbTxBytesCap rbExUnitsCap simTxs mempool

  -- How a producer orders the mempool into a block, absent any reservation
  -- rule. EBs always use this directly: the RB reservation does not constrain
  -- EB content.
  selectTxsByPolicy ::
    SelectionPolicy ->
    Int ->
    Int ->
    Map TxId Tx ->
    Seq TxId ->
    (Seq TxId, Seq TxId, (Int, Int))
  selectTxsByPolicy selection byteCap exUnitCap simTxs mempool =
    case selection of
      Fifo ->
        selectByBlockCapacity byteCap exUnitCap simTxs mempool
      PriorityFirst ->
        let (prioritySelected, afterPriority, priorityUsage) =
              selectPriorityByBlockCapacity byteCap exUnitCap simTxs mempool
            (standardSelected, remainingMempool, totalUsage) =
              selectByBlockCapacityFrom priorityUsage byteCap exUnitCap simTxs afterPriority
         in (prioritySelected <> standardSelected, remainingMempool, totalUsage)
      FifoWithStandardCap standardShare ->
        selectFifoWithStandardCap standardShare byteCap exUnitCap simTxs mempool

  priorityTxBytesCap :: Design -> Int -> Int
  priorityTxBytesCap design rbTxBytesCap =
    case design.designReservationPolicy of
      PriorityReservationRb reservationBytes -> min rbTxBytesCap reservationBytes
      NoReservation -> rbTxBytesCap

  announceEndorserBlock :: SlotNo -> Int -> Int -> Map TxId Tx -> Mempool -> SimM (Seq SimEvent)
  announceEndorserBlock slot rbPriorityTxBytesCap rbExUnitsCap simTxs mempool = do
    ebTxBytesCap <- asks simConfigEbTxBytesCap
    ebExUnitsCap <- asks simConfigEbExUnitsCap
    ebs <- gets _simEbs
    prices <- gets _simPrices
    design <- asks simConfigDesign
    let (feeCheckedMempool, evictionEvents) =
          evictStaleFees design.designFeeSemantics slot prices simTxs mempool
        -- Producer headroom: fill the EB only with txs that stay valid
        -- through the single price update that can fire before the
        -- certification check, so a prudent producer's EB cannot fail
        -- validation. Semi-redundant with 'admissionRequiredFee': admission
        -- with a horizon >= 1 guarantees this headroom at entry, but prices
        -- may have risen while the tx waited, so the producer re-checks
        -- against current prices — that check is the mempool-hygiene
        -- heuristic, this is the exact protocol-safety bound. Ineligible
        -- txs are not evicted: they stay in the mempool (RB-eligible only
        -- as the reservation policy allows) and regain EB eligibility if
        -- prices fall.
        headroomPrices = worstCaseNextPrices design.designControllers prices
        ebEligible txId =
          case Map.lookup txId simTxs of
            Nothing -> False
            Just tx ->
              case design.designFeeSemantics of
                FixedFee -> True
                -- For HonourSubmissionQuoteFor the honour window may expire
                -- before the certification check, so only the price bound
                -- guarantees safety.
                _ -> tx.txBody._txFee >= quotedFee headroomPrices tx
        (selectedTxs, _remainingMempool, (_usedBytes, _usedExUnits)) =
          selectTxsByPolicy design.designSelection ebTxBytesCap ebExUnitsCap simTxs (Seq.filter ebEligible feeCheckedMempool.mempoolTxIds)
        selectedTxIds = toList selectedTxs
        selectedTxBodyList = selectedTxBodies simTxs selectedTxs
    if null selectedTxIds
      then do
        modify' \st -> st{_simMempool = feeCheckedMempool}
        pure evictionEvents
      else do
        let ebId = nextEbId ebs
            eb =
              EndorserBlock
                { _ebTxs = Set.fromList selectedTxIds
                , _ebId = ebId
                }
            summary =
              EndorserBlockAnnounced
                (mkEndorserBlockSummary ebId ebTxBytesCap ebExUnitsCap rbPriorityTxBytesCap rbExUnitsCap selectedTxBodyList)
        modify' \st ->
          st
            { _simMempool = feeCheckedMempool
            , _simEbs = Map.insert ebId eb st._simEbs
            , _simPendingEb = Just (PendingEb ebId slot)
            }
        pure $ evictionEvents |> BlockProduced slot summary

  evictStaleFees :: FeeSemantics -> SlotNo -> Prices -> Map TxId Tx -> Mempool -> (Mempool, Seq SimEvent)
  evictStaleFees semantics slot prices simTxs mempool =
    (setMempoolTxIds simTxs keptTxIds, evictions)
   where
    (keptTxIds, evictions) =
      foldl' checkTx (mempty, mempty) mempool.mempoolTxIds

    checkTx (kept, events) txId =
      case Map.lookup txId simTxs of
        Nothing ->
          (kept |> txId, events)
        Just tx
          | txFeeStillValid semantics slot prices tx ->
              (kept |> txId, events)
          | otherwise ->
              (kept, events |> staleFeeEviction slot prices txId tx)

  -- Staleness per the design's fee semantics: under Eip1559 the posted max
  -- fee must still cover the current quote; HonourSubmissionQuoteFor defers
  -- that check until the honour window after submission has elapsed; FixedFee
  -- never goes stale.
  txFeeStillValid :: FeeSemantics -> SlotNo -> Prices -> Tx -> Bool
  txFeeStillValid semantics slot prices tx =
    case semantics of
      FixedFee -> True
      Eip1559 -> coversCurrentQuote
      HonourSubmissionQuoteFor honourFor ->
        diffSlots slot tx.txSubmitted <= honourFor || coversCurrentQuote
   where
    coversCurrentQuote =
      tx.txBody._txFee >= quotedFee prices tx

  staleFeeEviction :: SlotNo -> Prices -> TxId -> Tx -> SimEvent
  staleFeeEviction slot prices txId tx =
    TxEvicted slot txId (FeeTooLowAtSelection tx.txBody._txFee (quotedFee prices tx))

  nextEbId :: Map EbId EndorserBlock -> EbId
  nextEbId ebs =
    case Map.lookupMax ebs of
      Nothing -> EbId 0
      Just (EbId n, _) -> EbId (n + 1)
