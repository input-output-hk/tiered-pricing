module LoadProfile (
  LoadProfile (..),
  loadLoadProfile,
) where

import Data.Aeson (FromJSON (..), Value, eitherDecode, withObject, (.:), (.:?))
import Data.ByteString.Lazy qualified as BL
import Load (ArrivalProcess)

-- | A named, reusable workload that can override every variant in a sweep.
data LoadProfile = LoadProfile
  { loadProfileName :: String
  , loadProfileDescription :: Maybe String
  , loadProfileProcess :: ArrivalProcess
  , loadProfileValue :: Value
  -- ^ Original JSON value inserted into each effective simulation config.
  }
  deriving stock (Eq, Show)

instance FromJSON LoadProfile where
  parseJSON =
    withObject "LoadProfile" \obj -> do
      name <- obj .: "name"
      if null name
        then fail "load profile name must be non-empty"
        else do
          description <- obj .:? "description"
          loadValue <- obj .: "load"
          process <- parseJSON loadValue
          pure (LoadProfile name description process loadValue)

loadLoadProfile :: FilePath -> IO LoadProfile
loadLoadProfile path = do
  bytes <- BL.readFile path
  case eitherDecode bytes of
    Left err -> fail ("cannot parse load profile " <> path <> ": " <> err)
    Right profile -> pure profile
