module Transaction where

import Data.Bits (shiftR, xor, (.&.))
import Data.Char (ord)
import Data.Set (Set)
import Data.Set qualified as Set
import Data.Word (Word64, Word8)
import Types (Duration (..), Lovelace (..), SlotNo (..), Urgency (..), diffSlots)

data Script = Script
  { _scriptSize :: Int -- Bytes
  , _scriptExUnits :: Int
  }

newtype TxId = TxId Int deriving (Eq, Ord, Show)

data Tx = Tx
  { txId :: TxId
  , txBody :: TxBody
  , txSubmitted :: SlotNo
  , txValue :: Lovelace
  , txUrgency :: Urgency
  , txLane :: Lane
  }

data TxBody = TxBody
  { _txSize :: Int -- Bytes
  , _txScript :: Script
  , _txDependsOn :: Set TxId
  , _txFee :: Lovelace
  }

data TxSample = TxSample
  { sampleTxSizeP :: Double
  , sampleScriptSizeP :: Double
  , sampleExUnitsP :: Double
  , sampleTxValueP :: Double
  , sampleUrgencyP :: Double
  }

data Lane = Priority | Standard deriving stock (Eq, Ord, Show)

data RejectReason
  = FeeTooLow Lovelace Lovelace -- submitted, required
  | MempoolFull Int Int Int -- current mempool bytes, tx bytes, cap bytes
  deriving stock (Eq, Show)

data EvictionReason = EvictionReason

retainedValue :: Duration -> Tx -> Lovelace
retainedValue duration tx =
  retainedValueFor duration tx.txUrgency tx.txValue

retainedValueFor :: Duration -> Urgency -> Lovelace -> Lovelace
retainedValueFor duration urgency =
  scaleRetainedValue (retentionRatio duration urgency)

lostValue :: Duration -> Tx -> Lovelace
lostValue duration tx =
  subtractLovelace tx.txValue (retainedValue duration tx)

valueAt :: SlotNo -> Tx -> Lovelace
valueAt slot tx =
  retainedValue (diffSlots slot tx.txSubmitted) tx

retentionRatio :: Duration -> Urgency -> Double
retentionRatio duration urgency =
  case urgency of
    Linear rate ->
      max 0 (1 - decayRate rate * durationSlots duration)
    Exponential rate ->
      exp (negate (decayRate rate * durationSlots duration))

durationSlots :: Duration -> Double
durationSlots (Duration slots) =
  fromIntegral (max 0 slots)

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
  encodeAscii "TxBody:v3"
    <> encodeInt _txSize
    <> encodeScript _txScript
    <> encodeList encodeTxId (Set.toAscList _txDependsOn)
    <> encodeLovelace _txFee

encodeScript :: Script -> [Word8]
encodeScript Script{..} =
  encodeAscii "Script:v1"
    <> encodeInt _scriptSize
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

encodeAscii :: String -> [Word8]
encodeAscii s =
  encodeInt (length s) <> fmap (fromIntegral . ord) s

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
