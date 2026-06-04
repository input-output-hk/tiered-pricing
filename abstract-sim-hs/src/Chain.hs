module Chain where

import Block (RankingBlock)
import Data.Sequence (Seq)
import Types (SlotNo)

data Chain = Chain
  { _chainBlocks :: Seq (SlotNo, RankingBlock)
  , _chainTip :: Maybe SlotNo
  }

emptyChain :: Chain
emptyChain = Chain mempty Nothing