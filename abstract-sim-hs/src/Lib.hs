module Lib
    ( someFunc
    ) where

import Run qualified

someFunc :: IO ()
someFunc = Run.run
