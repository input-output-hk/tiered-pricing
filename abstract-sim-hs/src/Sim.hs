{-# LANGUAGE DerivingStrategies #-}
{-# LANGUAGE GeneralizedNewtypeDeriving #-}

module Sim where

import Actor (Actor (..), TxSubmission (TxSubmission), generateTransaction)
import Block (BlockSummary (..), EbId (..), EndorserBlock (..), InclusionPoint (..), PendingEb (..), RankingBlock (..), mkEndorserBlockSummary, mkRankingBlockSummary, selectByBlockCapacity, selectPriorityByBlockCapacity, selectedTxBodies)
import Chain (Chain (..), emptyChain)
import Config (SimConfig (..))
import Control.Monad (join, replicateM)
import Control.Monad.Reader (MonadReader (..), ReaderT, asks)
import Control.Monad.State.Strict (MonadState (..), State, gets, modify')
import Curve (Curve, sampleCurve)
import Data.Foldable (Foldable (toList))
import Data.List.NonEmpty (NonEmpty (..))
import Data.Map (Map)
import Data.Map qualified as Map
import Data.Maybe (catMaybes, mapMaybe)
import Data.Sequence (Seq, singleton, (<|), (><), (|>))
import Data.Sequence qualified as Seq
import Data.Set qualified as Set
import Design (ControllerConfig (..), ControllerSignal (..), Design (..), Eip1559Controller (..), ReservationPolicy (..))
import Event (SimEvent (..))
import Load (arrivalRateAt, tryBurstEffectAt)
import Mempool (Mempool (..), admitToMempool, emptyMempool, removeFromMempool, setMempoolTxIds)
import Pricing (PriceUpdate (..), Prices (..), initialPrices, quotedFee, updatePrices)
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
    }

step :: SimM (Seq SimEvent)
step = do
  txs <- actorStep
  submittedEvents <- traverse admitTxSubmission txs
  blockEvents <- blockStep
  recordBlockEvents blockEvents
  priceEvents <-
    if any isBlockProduced blockEvents
      then priceStep
      else pure mempty
  advanceSlot
  pure (join (Seq.fromList submittedEvents) <> blockEvents <> priceEvents)

calculateFee :: Tx -> SimM Lovelace
calculateFee tx = do
  prices <- gets _simPrices
  pure (quotedFee prices tx)

admitTxSubmission :: TxSubmission -> SimM (Seq SimEvent)
admitTxSubmission (TxSubmission actorId tx) = do
  simSt <- get
  requiredFee <- calculateFee tx
  mempool <- gets _simMempool
  simTxs <- gets _simTxs
  mempoolBytesCap <- asks simConfigMempoolBytesCap
  case validateAdmission mempoolBytesCap mempool tx requiredFee of
    AdmissionRejected reasons ->
      pure $ singleton (TxSubmitted simSt._simSlot actorId tx) |> TxRejected simSt._simSlot tx.txId reasons
    AdmissionAccepted () -> do
      let newMempool = admitToMempool mempool tx
          newSimTxs = Map.insert tx.txId tx simTxs
      modify' \st -> st{_simMempool = newMempool, _simTxs = newSimTxs}
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
  catMaybes <$> replicateM n do
    actor <- pickActor actors
    txSample <- drawTxSample
    pure (TxSubmission actor._actorId <$> generateTransaction f slot actor prices latencyEstimate curves txSample burstEffect)

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

sampleArrivalCount :: Double -> SimM Int
sampleArrivalCount rate = do
  let whole = floor rate
      frac = rate - fromIntegral whole
  extra <- roll frac
  pure (whole + if extra then 1 else 0)

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
      modify' \st -> st{_simPendingEb = Nothing}
      if certEb
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
    let ebTxIdSet =
          maybe mempty _ebTxs (Map.lookup pending.pendingEbId ebs)
        ebTxIds = Set.toList ebTxIdSet
        (validEbTxIds, staleFeeEvents) =
          feeValidTxIds slot prices simTxs ebTxIds
        ebTxs = selectedTxBodies simTxs (Seq.fromList validEbTxIds)
        block = CertifyingBlock pending.pendingEbId
        mempool' = removeFromMempool simTxs ebTxIdSet mempool
        summary =
          RankingBlockProduced
            (mkRankingBlockSummary block 0 0 0 0 [])
        certifiedEbSummary =
          EndorserBlockCertified
            (mkEndorserBlockSummary pending.pendingEbId ebTxBytesCap ebExUnitsCap rbPriorityTxBytesCap rbExUnitsCap ebTxs)
        events =
          Seq.fromList [BlockProduced slot summary]
            >< staleFeeEvents
            >< Seq.fromList
              ( BlockProduced slot certifiedEbSummary
                  : fmap (\txId -> TxIncluded slot txId (IncludedInEb pending.pendingEbId)) validEbTxIds
              )
    appendRankingBlock slot block
    modify' \st -> st{_simMempool = mempool'}
    pure (mempool', events)

  producePraosBlock :: Design -> SlotNo -> Int -> Int -> Map TxId Tx -> Mempool -> SimM (Mempool, Seq SimEvent)
  producePraosBlock design slot rbTxBytesCap rbExUnitsCap simTxs mempool = do
    prices <- gets _simPrices
    let (feeCheckedMempool, evictionEvents) =
          evictStaleFees slot prices simTxs mempool
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
            : fmap (\txId -> TxIncluded slot txId IncludedInRb) selectedTxIds
    appendRankingBlock slot block
    modify' \st -> st{_simMempool = mempool'}
    pure (mempool', evictionEvents >< Seq.fromList events)

  selectRankingBlockTxs ::
    Design ->
    Int ->
    Int ->
    Map TxId Tx ->
    Seq TxId ->
    (Seq TxId, Seq TxId, (Int, Int))
  selectRankingBlockTxs design rbTxBytesCap rbExUnitsCap simTxs mempool =
    case design.designReservationPolicy of
      PriorityReservationRb reservationBytes ->
        selectPriorityByBlockCapacity
          (min rbTxBytesCap reservationBytes)
          rbExUnitsCap
          simTxs
          mempool
      NoReservation ->
        selectByBlockCapacity rbTxBytesCap rbExUnitsCap simTxs mempool

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
    let (feeCheckedMempool, evictionEvents) =
          evictStaleFees slot prices simTxs mempool
        (selectedTxs, _remainingMempool, (_usedBytes, _usedExUnits)) =
          selectByBlockCapacity ebTxBytesCap ebExUnitsCap simTxs feeCheckedMempool.mempoolTxIds
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

  evictStaleFees :: SlotNo -> Prices -> Map TxId Tx -> Mempool -> (Mempool, Seq SimEvent)
  evictStaleFees slot prices simTxs mempool =
    (setMempoolTxIds simTxs keptTxIds, evictions)
   where
    (keptTxIds, evictions) =
      foldl' checkTx (mempty, mempty) mempool.mempoolTxIds

    checkTx (kept, events) txId =
      case Map.lookup txId simTxs of
        Nothing ->
          (kept |> txId, events)
        Just tx
          | txFeeStillValid prices tx ->
              (kept |> txId, events)
          | otherwise ->
              (kept, events |> staleFeeEviction slot prices txId tx)

  feeValidTxIds :: SlotNo -> Prices -> Map TxId Tx -> [TxId] -> ([TxId], Seq SimEvent)
  feeValidTxIds slot prices simTxs =
    foldr checkTx ([], mempty)
   where
    checkTx txId (validTxIds, events) =
      case Map.lookup txId simTxs of
        Nothing ->
          (validTxIds, events)
        Just tx
          | txFeeStillValid prices tx ->
              (txId : validTxIds, events)
          | otherwise ->
              (validTxIds, staleFeeEviction slot prices txId tx <| events)

  txFeeStillValid :: Prices -> Tx -> Bool
  txFeeStillValid prices tx =
    tx.txBody._txFee >= quotedFee prices tx

  staleFeeEviction :: SlotNo -> Prices -> TxId -> Tx -> SimEvent
  staleFeeEviction slot prices txId tx =
    TxEvicted slot txId (FeeTooLowAtSelection tx.txBody._txFee (quotedFee prices tx))

  nextEbId :: Map EbId EndorserBlock -> EbId
  nextEbId ebs =
    case Map.lookupMax ebs of
      Nothing -> EbId 0
      Just (EbId n, _) -> EbId (n + 1)
