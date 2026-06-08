import json
from collections import defaultdict


def _ratio(used, cap):
    return (used / cap) if (cap and cap > 0) else None


def _rb_fullness(inner):
    """Ranking-block fullness in [0,1]: the binding of bytes / ex-units utilisation.
    None when there is no capacity (e.g. a certifying block carries no txs)."""
    ratios = [r for r in (
        _ratio(inner.get("usedBytes", 0), inner.get("capacityBytes", 0)),
        _ratio(inner.get("usedExUnits", 0), inner.get("capacityExUnits", 0)),
    ) if r is not None]
    return max(0.0, min(1.0, max(ratios))) if ratios else None


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
        self.included_route = {}            # txId -> "IncludedInRb" | "IncludedInEb"
        self.price_changes = defaultdict(list)     # lane -> [PriceUpdated event]
        self.submissions_per_slot = defaultdict(int)
        self.inclusions_per_slot = defaultdict(int)
        self.rb_count = 0                   # ranking blocks produced (total)
        self.rb_tx_count = 0                # RBs carrying txs directly (PraosBlock)
        self.rb_cert_count = 0              # RBs carrying an EB certificate (CertifyingBlock)
        self.rb_series = []                 # [{slot, kind, fill}] per RB, in order (over-time strip)
        self.eb_fullness = {}               # EB id -> fullness, to shade certifying blocks
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
            self.included_route[event["txId"]] = event.get("inclusionPoint", {}).get("tag")
            self.inclusions_per_slot[slot] += 1
        elif tag == "PriceUpdated":
            self.price_changes[event["lane"]].append(event)
        elif tag == "BlockProduced":
            summary = event.get("summary", {})
            stag = summary.get("tag")
            inner = summary.get("summary", {})
            if stag == "EndorserBlockAnnounced":
                # record each EB's fullness so a certifying RB can be shaded by the EB it certifies
                eb_id = inner.get("id")
                if eb_id is not None:
                    self.eb_fullness[eb_id] = _rb_fullness(inner)
            elif stag == "RankingBlockProduced":
                # ranking blocks set the chain's slots-per-block cadence; each carries
                # either txs directly (PraosBlock) or an EB certificate (CertifyingBlock)
                self.rb_count += 1
                block = inner.get("block", {})
                block_tag = block.get("tag")
                if block_tag == "PraosBlock":
                    self.rb_tx_count += 1
                    self.rb_series.append({"slot": slot, "kind": "txs", "fill": _rb_fullness(inner)})
                elif block_tag == "CertifyingBlock":
                    # cert blocks carry no txs; shade by the fullness of the EB they certify
                    self.rb_cert_count += 1
                    self.rb_series.append(
                        {"slot": slot, "kind": "cert", "fill": self.eb_fullness.get(block.get("ebId"))})
        # all other tags ignored (this iteration)

    @property
    def slot_count(self):
        return self.max_slot + 1
