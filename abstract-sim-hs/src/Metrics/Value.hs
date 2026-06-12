module Metrics.Value (
  ValueOutcome (..),
  valueOutcome,
) where

import Metrics.Accumulator
import Transaction (retainedValueFor, subtractLovelace)
import Types (BlockDelay (..), Lovelace (..))

{- | Metric (2): retained vs lost demand-unit value. Every unit contributes to
exactly one column: decay-discounted retention if served (decayed from the
unit's *first* submission, so retry wait counts against the design), full
loss if abandoned, or unresolved if still in flight at the end of the run.
Unresolved value is reported rather than folded into lost so the run horizon
does not masquerade as a design failure.
-}
data ValueOutcome = ValueOutcome
  { retainedValue :: Lovelace
  -- ^ decayed value of served units
  , lostValue :: Lovelace
  -- ^ decay losses of served units plus the full value of abandoned units
  , unresolvedValue :: Lovelace
  -- ^ value of units neither served nor abandoned when the run ended
  }
  deriving (Eq, Show)

valueOutcome :: [DemandUnit] -> ValueOutcome
valueOutcome units =
  ValueOutcome
    { retainedValue = sumLovelace [retained | (retained, _, _) <- outcomes]
    , lostValue = sumLovelace [lost | (_, lost, _) <- outcomes]
    , unresolvedValue = sumLovelace [unresolved | (_, _, unresolved) <- outcomes]
    }
 where
  outcomes = fmap unitValueOutcome units

unitValueOutcome :: DemandUnit -> (Lovelace, Lovelace, Lovelace)
unitValueOutcome unit =
  case unit.unitOutcome of
    Just (UnitIncluded _ block _ _) ->
      let delay = BlockDelay (fromIntegral (max 0 (block - unit.unitFirstSubmittedBlock)))
          retained = retainedValueFor delay unit.unitUrgency unit.unitValue
       in (retained, subtractLovelace unit.unitValue retained, Lovelace 0)
    Just (UnitAbandoned _) ->
      (Lovelace 0, unit.unitValue, Lovelace 0)
    Nothing ->
      (Lovelace 0, Lovelace 0, unit.unitValue)
