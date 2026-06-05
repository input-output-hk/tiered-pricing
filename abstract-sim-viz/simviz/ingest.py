import json
from collections import defaultdict


def iter_events(path):
    """Stream a JSONL trace, yielding each line's inner `event` object in order."""
    with open(path, "r") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            yield json.loads(line)["event"]


class Accumulator:
    """Single-pass accumulator over a SimEvent trace. last-write-wins by txId,
    mirroring the simulator's Map.insert semantics."""

    def __init__(self):
        self.submitted_at = {}              # txId -> submit slot
        self.tx_meta = {}                   # txId -> {"tag", "rate", "lane"}
        self.included_at = {}               # txId -> inclusion slot
        self.price_changes = defaultdict(list)     # lane -> [PriceUpdated event]
        self.submissions_per_slot = defaultdict(int)
        self.inclusions_per_slot = defaultdict(int)
        self.rb_count = 0                   # ranking blocks produced (total)
        self.rb_tx_count = 0                # RBs carrying txs directly (PraosBlock)
        self.rb_cert_count = 0              # RBs carrying an EB certificate (CertifyingBlock)
        self.max_slot = 0
        self.total_events = 0

    def ingest(self, event):
        self.total_events += 1
        tag = event["tag"]
        slot = event.get("slot", 0)
        if slot > self.max_slot:
            self.max_slot = slot
        if tag == "TxSubmitted":
            tx = event["tx"]
            tx_id = tx["id"]
            self.submitted_at[tx_id] = tx["submitted"]
            self.tx_meta[tx_id] = {
                "tag": tx["urgency"]["tag"],
                "rate": tx["urgency"]["rate"],
                "lane": tx["lane"],
            }
            self.submissions_per_slot[tx["submitted"]] += 1
        elif tag == "TxIncluded":
            self.included_at[event["txId"]] = slot
            self.inclusions_per_slot[slot] += 1
        elif tag == "PriceUpdated":
            self.price_changes[event["lane"]].append(event)
        elif tag == "BlockProduced":
            summary = event.get("summary", {})
            if summary.get("tag") == "RankingBlockProduced":
                # ranking blocks set the chain's slots-per-block cadence; each carries
                # either txs directly (PraosBlock) or an EB certificate (CertifyingBlock)
                self.rb_count += 1
                block_tag = summary.get("summary", {}).get("block", {}).get("tag")
                if block_tag == "PraosBlock":
                    self.rb_tx_count += 1
                elif block_tag == "CertifyingBlock":
                    self.rb_cert_count += 1
        # all other tags ignored (this iteration)

    @property
    def slot_count(self):
        return self.max_slot + 1
