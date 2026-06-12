module Curve (
  Curve (..),
  CurvePoint (..),
  Curves (..),
  TxSizeCurve (..),
  ScriptSizeCurve (..),
  ExUnitsCurve (..),
  TxValueCurve (..),
  sampleCurve,
  curvesDefault,
  txSizeCurveDefault,
  scriptSizeCurveDefault,
  exUnitsCurveDefault,
  txValueCurveDefault,
) where

import Data.Aeson (FromJSON (..), Value (..), withObject, (.:), (.:?))
import Data.List.NonEmpty (NonEmpty (..), nonEmpty)
import Data.Text qualified as T

data Curve
  = Constant Double
  | PiecewiseLinear (NonEmpty CurvePoint)
  deriving stock (Eq, Show)

-- | A bare number is a constant curve; otherwise a tagged object.
instance FromJSON Curve where
  parseJSON value@(Number _) =
    Constant <$> parseJSON value
  parseJSON value =
    flip (withObject "Curve") value \obj -> do
      tag <- obj .: "type"
      case tag :: String of
        "constant" -> Constant <$> obj .: "value"
        "piecewise-linear" -> do
          points <- obj .: "points"
          case nonEmpty points of
            Just nonEmptyPoints -> pure (PiecewiseLinear nonEmptyPoints)
            Nothing -> fail "piecewise-linear curve requires at least one point"
        _ -> fail ("unknown curve type: " <> tag)

data CurvePoint = CurvePoint
  { curveP :: Double -- quantile, in [0,1]
  , curveValue :: Double -- sampled value at that quantile
  }
  deriving stock (Eq, Show)

instance FromJSON CurvePoint where
  parseJSON =
    withObject "CurvePoint" \obj ->
      CurvePoint
        <$> obj .: "p"
        <*> obj .: "value"

sampleCurve :: Curve -> Double -> Double
sampleCurve (Constant v) _ = v
sampleCurve (PiecewiseLinear points) val =
  samplePoints points (clamp 0 1 val)

samplePoints :: NonEmpty CurvePoint -> Double -> Double
samplePoints (p :| []) _ = p.curveValue
samplePoints (lo :| hi : rest) val
  | val <= lo.curveP = lo.curveValue
  | val <= hi.curveP = interpolate lo hi val
  | otherwise = samplePoints (hi :| rest) val

interpolate :: CurvePoint -> CurvePoint -> Double -> Double
interpolate lo hi val =
  let t = (val - lo.curveP) / (hi.curveP - lo.curveP)
   in lo.curveValue + t * (hi.curveValue - lo.curveValue)

clamp :: Double -> Double -> Double -> Double
clamp lo hi = min hi . max lo

newtype TxSizeCurve = TxSizeCurve Curve
  deriving stock (Eq, Show)

newtype ScriptSizeCurve = ScriptSizeCurve Curve
  deriving stock (Eq, Show)

newtype ExUnitsCurve = ExUnitsCurve Curve
  deriving stock (Eq, Show)

newtype TxValueCurve = TxValueCurve Curve
  deriving stock (Eq, Show)

data Curves = Curves
  { curveTxSize :: TxSizeCurve
  , curveScriptSize :: ScriptSizeCurve
  , curveExUnits :: ExUnitsCurve
  , curveTxValue :: TxValueCurve
  }
  deriving stock (Eq, Show)

-- | @"default"@ (bare or tagged) selects 'curvesDefault'; otherwise an
-- object with the four sampling curves.
instance FromJSON Curves where
  parseJSON (String preset)
    | preset == "default" = pure curvesDefault
    | otherwise = fail ("unknown curves preset: " <> T.unpack preset)
  parseJSON value =
    flip (withObject "Curves") value \obj -> do
      tag <- obj .:? "type"
      case tag of
        Nothing ->
          Curves
            <$> (TxSizeCurve <$> obj .: "txSize")
            <*> (ScriptSizeCurve <$> obj .: "scriptSize")
            <*> (ExUnitsCurve <$> obj .: "exUnits")
            <*> (TxValueCurve <$> obj .: "txValue")
        Just ("default" :: String) ->
          pure curvesDefault
        Just unknownTag ->
          fail ("unknown curves type: " <> unknownTag)

txSizeCurveDefault :: TxSizeCurve
txSizeCurveDefault = TxSizeCurve recentMainnetTxSize

scriptSizeCurveDefault :: ScriptSizeCurve
scriptSizeCurveDefault = ScriptSizeCurve recentMainnetScriptSize

exUnitsCurveDefault :: ExUnitsCurve
exUnitsCurveDefault = ExUnitsCurve recentMainnetExUnits

txValueCurveDefault :: TxValueCurve
txValueCurveDefault = TxValueCurve recentMainnetInclusionValue

curvesDefault :: Curves
curvesDefault =
  Curves
    { curveTxSize = txSizeCurveDefault
    , curveScriptSize = scriptSizeCurveDefault
    , curveExUnits = exUnitsCurveDefault
    , curveTxValue = txValueCurveDefault
    }

recentMainnetTxSize :: Curve
recentMainnetTxSize =
  PiecewiseLinear $
    CurvePoint 0.00 190
      :| [ CurvePoint 0.10 271
         , CurvePoint 0.25 372
         , CurvePoint 0.50 742
         , CurvePoint 0.75 1_306
         , CurvePoint 0.90 2_004
         , CurvePoint 0.95 4_260
         , CurvePoint 0.99 9_569
         , CurvePoint 0.995 9_993
         , CurvePoint 1.00 11_835
         ]

recentMainnetScriptSize :: Curve
recentMainnetScriptSize =
  PiecewiseLinear $
    CurvePoint 0.00 0
      :| [ CurvePoint 0.66 0
         , CurvePoint 0.75 4_024
         , CurvePoint 0.90 19_702
         , CurvePoint 0.95 22_263
         , CurvePoint 0.99 29_553
         , CurvePoint 0.995 31_806
         , CurvePoint 1.00 71_954
         ]

recentMainnetExUnits :: Curve
recentMainnetExUnits =
  PiecewiseLinear $
    CurvePoint 0.00 0
      :| [ CurvePoint 0.66 0
         , CurvePoint 0.75 850_968
         , CurvePoint 0.90 2_002_248
         , CurvePoint 0.95 2_656_184
         , CurvePoint 0.99 6_289_607
         , CurvePoint 0.995 9_154_460
         , CurvePoint 1.00 18_917_287
         ]

{- | Prompt-inclusion value, not transferred ADA value.

The first version used transaction output value here, which made ordinary txs
look as if delay destroyed thousands of ADA. That is the wrong economic proxy:
most output value is change or transferred principal, not private delay value.

This curve is anchored to actual fee quantiles from a Koios sample of 1,469 txs
across 120 recent Conway mainnet blocks on 2026-06-04. Current flat fees are a
censored willingness-to-pay signal, so the default uses a small multiple of the
observed fee as a fee-budget proxy for private prompt-inclusion value.
-}
recentMainnetInclusionValue :: Curve
recentMainnetInclusionValue =
  PiecewiseLinear $
    feeBudgetPoint 0.00 164_049
      :| [ feeBudgetPoint 0.10 169_756
         , feeBudgetPoint 0.25 176_985
         , feeBudgetPoint 0.50 208_665
         , feeBudgetPoint 0.75 481_231
         , feeBudgetPoint 0.90 671_110
         , feeBudgetPoint 0.95 772_184
         , feeBudgetPoint 0.99 1_000_000
         , feeBudgetPoint 1.00 2_124_717
         ]

feeBudgetPoint :: Double -> Double -> CurvePoint
feeBudgetPoint p observedFee =
  CurvePoint p (feeBudgetValueMultiple * observedFee)

feeBudgetValueMultiple :: Double
feeBudgetValueMultiple = 5
