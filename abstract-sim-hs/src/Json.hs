{- | The config-file convention for sum types, written once.

A sum alternative is spelled either as a bare string (nullary alternatives
only) or as an object whose @type@ field carries the tag with the
alternative's fields alongside. Nullary alternatives may also use the
object form; field-bearing alternatives require it.
-}
module Json (
  Alt (..),
  taggedSum,
) where

import Data.Aeson (Object, Value (..), withObject, (.:))
import Data.Aeson.Types (Parser)
import Data.Text qualified as T

data Alt a
  = Nullary a
  | WithFields (Object -> Parser a)

taggedSum :: String -> [(String, Alt a)] -> Value -> Parser a
taggedSum name alts = \case
  String tag ->
    case lookup (T.unpack tag) alts of
      Just (Nullary a) -> pure a
      Just (WithFields _) -> fail (name <> " " <> T.unpack tag <> " requires an object")
      Nothing -> unknown (T.unpack tag)
  value ->
    flip (withObject name) value \obj -> do
      tag <- obj .: "type"
      case lookup tag alts of
        Just (Nullary a) -> pure a
        Just (WithFields parseFields) -> parseFields obj
        Nothing -> unknown tag
 where
  unknown tag =
    fail ("unknown " <> name <> ": " <> tag)
