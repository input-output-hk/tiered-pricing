module Transaction (
  Script (..),
  TxId (..),
  DemandId (..),
  Provenance (..),
  Demand (..),
  Tx (..),
  TxBody (..),
  TxSample (..),
  Lane (..),
  RejectReason (..),
  EvictionReason (..),
  retainedValue,
  retainedValueFor,
  lostValue,
  valueAt,
  retentionRatio,
  subtractLovelace,
  hash,
) where

import Data.Aeson (ToJSON (..), object, (.=))
import Data.Bits (shiftR, xor, (.&.))
import Data.Set (Set)
import Data.Set qualified as Set
import Data.Word (Word64, Word8)
import GHC.Records (HasField (..))
import Types (BlockDelay (..), Lane (..), Lovelace (..), SlotNo (..), Urgency (..), diffSlots, expectedBlockDelay)

data Script = Script
  { _scriptSize :: Int -- Bytes
  , _scriptExUnits :: Int
  }

instance ToJSON Script where
  toJSON script =
    object
      [ "sizeBytes" .= script._scriptSize
      , "exUnits" .= script._scriptExUnits
      ]

newtype TxId = TxId Int deriving (Eq, Ord, Show)

instance ToJSON TxId where
  toJSON (TxId n) = toJSON n

-- | The identity of a demand unit: the tx number of its first submission.
newtype DemandId = DemandId Int deriving (Eq, Ord, Show)

instance ToJSON DemandId where
  toJSON (DemandId n) = toJSON n

{- | Where a generated tx comes from: a fresh demand unit, or the
resubmission of one whose earlier attempt failed.
-}
data Provenance
  = FreshDemand
  | -- | demand unit, attempt number of this submission (first attempt = 1),
    -- origin submission slot — the value-decay anchor
    ResubmissionOf DemandId Int SlotNo
  deriving stock (Eq, Show)

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

data Tx = Tx
  { txId :: TxId
  , txBody :: TxBody
  , txSubmitted :: SlotNo
  , txDemand :: Demand
  -- ^ the demand unit this attempt serves
  , txLane :: Lane
  , txProvenance :: Provenance
  -- ^ fresh demand or a resubmission; origin facts are views of this
  }

{- Virtual fields: the flattened views every reader had before the demand
payload and provenance were embedded. The first-attempt equations
(origin = own number, attempt = 1, origin slot = submission slot) are now
definitional rather than comment-enforced. -}

instance HasField "txValue" Tx Lovelace where
  getField tx = tx.txDemand.demandValue

instance HasField "txUrgency" Tx Urgency where
  getField tx = tx.txDemand.demandUrgency

instance HasField "txOriginNumber" Tx DemandId where
  getField tx =
    case tx.txProvenance of
      FreshDemand -> DemandId tx.txBody._txNumber
      ResubmissionOf origin _ _ -> origin

instance HasField "txAttempt" Tx Int where
  getField tx =
    case tx.txProvenance of
      FreshDemand -> 1
      ResubmissionOf _ attempt _ -> attempt

instance HasField "txOriginSubmitted" Tx SlotNo where
  getField tx =
    case tx.txProvenance of
      FreshDemand -> tx.txSubmitted
      ResubmissionOf _ _ originSubmitted -> originSubmitted

instance ToJSON Tx where
  toJSON tx =
    object
      [ "id" .= tx.txId
      , "body" .= tx.txBody
      , "submitted" .= tx.txSubmitted
      , "value" .= tx.txValue
      , "urgency" .= tx.txUrgency
      , "lane" .= tx.txLane
      , "originNumber" .= tx.txOriginNumber
      , "attempt" .= tx.txAttempt
      , "originSubmitted" .= tx.txOriginSubmitted
      ]

data TxBody = TxBody
  { _txSize :: Int -- Bytes
  , _txScript :: Script
  , _txDependsOn :: Set TxId
  , _txFee :: Lovelace
  , _txNumber :: Int
  }

instance ToJSON TxBody where
  toJSON body =
    object
      [ "sizeBytes" .= body._txSize
      , "script" .= body._txScript
      , "dependsOn" .= Set.toAscList body._txDependsOn
      , "fee" .= body._txFee
      , "number" .= body._txNumber
      ]

data TxSample = TxSample
  { sampleTxSizeP :: Double
  , sampleScriptSizeP :: Double
  , sampleExUnitsP :: Double
  , sampleTxValueP :: Double
  , sampleUrgencyP :: Double
  }

instance ToJSON TxSample where
  toJSON sample =
    object
      [ "txSizeP" .= sample.sampleTxSizeP
      , "scriptSizeP" .= sample.sampleScriptSizeP
      , "exUnitsP" .= sample.sampleExUnitsP
      , "txValueP" .= sample.sampleTxValueP
      , "urgencyP" .= sample.sampleUrgencyP
      ]

data RejectReason
  = FeeTooLow Lovelace Lovelace -- submitted, required
  | MempoolFull Int Int Int -- current mempool bytes, tx bytes, cap bytes
  deriving stock (Eq, Show)

instance ToJSON RejectReason where
  toJSON = \case
    FeeTooLow submitted required ->
      object
        [ "tag" .= ("FeeTooLow" :: String)
        , "submitted" .= submitted
        , "required" .= required
        ]
    MempoolFull currentBytes txBytes capBytes ->
      object
        [ "tag" .= ("MempoolFull" :: String)
        , "currentBytes" .= currentBytes
        , "txBytes" .= txBytes
        , "capBytes" .= capBytes
        ]

data EvictionReason
  = FeeTooLowAtSelection Lovelace Lovelace -- submitted, required
  deriving stock (Eq, Show)

instance ToJSON EvictionReason where
  toJSON = \case
    FeeTooLowAtSelection submitted required ->
      object
        [ "tag" .= ("FeeTooLowAtSelection" :: String)
        , "submitted" .= submitted
        , "required" .= required
        ]

retainedValue :: BlockDelay -> Tx -> Lovelace
retainedValue delay tx =
  retainedValueFor delay tx.txUrgency tx.txValue

retainedValueFor :: BlockDelay -> Urgency -> Lovelace -> Lovelace
retainedValueFor delay urgency =
  scaleRetainedValue (retentionRatio delay urgency)

lostValue :: BlockDelay -> Tx -> Lovelace
lostValue delay tx =
  subtractLovelace tx.txValue (retainedValue delay tx)

valueAt :: Double -> SlotNo -> Tx -> Lovelace
valueAt f slot tx =
  retainedValue (expectedBlockDelay f (diffSlots slot tx.txSubmitted)) tx

retentionRatio :: BlockDelay -> Urgency -> Double
retentionRatio delay urgency =
  case urgency of
    Linear rate ->
      max 0 (1 - decayRate rate * blockDelay delay)
    Exponential rate ->
      exp (negate (decayRate rate * blockDelay delay))

blockDelay :: BlockDelay -> Double
blockDelay (BlockDelay blocks) =
  max 0 blocks

decayRate :: Double -> Double
decayRate =
  max 0

scaleRetainedValue :: Double -> Lovelace -> Lovelace
scaleRetainedValue ratio value@(Lovelace initialValue)
  | ratio <= 0 = Lovelace 0
  | ratio >= 1 = value
  | otherwise =
      Lovelace
        ( max
            0
            (min initialValue (floor (fromInteger initialValue * ratio)))
        )

subtractLovelace :: Lovelace -> Lovelace -> Lovelace
subtractLovelace (Lovelace a) (Lovelace b) =
  Lovelace (max 0 (a - b))

{- | Cardano-style transaction id: hash the transaction body, not the wrapper.

This is deliberately small for the abstract simulator: Cardano uses a
Blake2b-256 hash of the canonical CBOR transaction body, while this uses a
canonical byte encoding and a stable 64-bit FNV-1a hash.
-}
hash :: TxBody -> TxId
hash body =
  TxId . fromIntegral $
    fnv1a64 (encodeTxBody body) .&. fromIntegral (maxBound :: Int)

encodeTxBody :: TxBody -> [Word8]
encodeTxBody TxBody{..} =
  encodeInt _txSize
    <> encodeScript _txScript
    <> encodeList encodeTxId (Set.toAscList _txDependsOn)
    <> encodeLovelace _txFee
    <> encodeInt _txNumber

encodeScript :: Script -> [Word8]
encodeScript Script{..} =
  encodeInt _scriptSize
    <> encodeInt _scriptExUnits

encodeTxId :: TxId -> [Word8]
encodeTxId (TxId n) = encodeInt n

encodeLovelace :: Lovelace -> [Word8]
encodeLovelace (Lovelace n) = encodeInteger n

encodeList :: (a -> [Word8]) -> [a] -> [Word8]
encodeList encode xs =
  encodeInt (length xs) <> concatMap encode xs

encodeInt :: Int -> [Word8]
encodeInt = encodeInteger . toInteger

encodeInteger :: Integer -> [Word8]
encodeInteger n
  | n < 0 = 1 : encodeWord64 (fromInteger (abs n))
  | otherwise = 0 : encodeWord64 (fromInteger n)

encodeWord64 :: Word64 -> [Word8]
encodeWord64 w =
  fmap byteAt [56, 48 .. 0]
 where
  byteAt shift =
    fromIntegral (w `shiftR` shift)

fnv1a64 :: [Word8] -> Word64
fnv1a64 =
  foldl' step 14_695_981_039_346_656_037
 where
  step h b = (h `xor` fromIntegral b) * 1_099_511_628_211
