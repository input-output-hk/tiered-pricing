{- | Block-resource quantities. Bytes and ex-units stop being bare 'Int's the
compiler cannot tell apart: swapping a byte cap for an ex-unit cap, or
mixing an RB cap into an EB call site, is a type error instead of a silent
simulation bias.
-}
module Resource (
  Bytes (..),
  ExUnits (..),
  Resources (..),
  fitsWithin,
  scaleResources,
) where

newtype Bytes = Bytes {unBytes :: Int}
  deriving stock (Eq, Ord, Show)

instance Semigroup Bytes where
  Bytes a <> Bytes b = Bytes (a + b)

instance Monoid Bytes where
  mempty = Bytes 0

newtype ExUnits = ExUnits {unExUnits :: Int}
  deriving stock (Eq, Ord, Show)

instance Semigroup ExUnits where
  ExUnits a <> ExUnits b = ExUnits (a + b)

instance Monoid ExUnits where
  mempty = ExUnits 0

-- | What a tx costs a block, or what a block offers: both dimensions at once.
data Resources = Resources
  { resBytes :: Bytes
  , resExUnits :: ExUnits
  }
  deriving stock (Eq, Show)

-- | Componentwise accumulation.
instance Semigroup Resources where
  a <> b =
    Resources
      { resBytes = a.resBytes <> b.resBytes
      , resExUnits = a.resExUnits <> b.resExUnits
      }

instance Monoid Resources where
  mempty = Resources mempty mempty

-- | Pointwise: within budget on every dimension.
fitsWithin :: Resources -> Resources -> Bool
fitsWithin used capacity =
  used.resBytes <= capacity.resBytes
    && used.resExUnits <= capacity.resExUnits

-- | A clamped share of a capacity, floored per component.
scaleResources :: Double -> Resources -> Resources
scaleResources share (Resources (Bytes bytes) (ExUnits exUnits)) =
  Resources (Bytes (scale bytes)) (ExUnits (scale exUnits))
 where
  scale c = floor (max 0 (min 1 share) * fromIntegral c)
