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


def unit_lane(unit):
    """Lane attribution for a demand unit: the lane that actually served it
    when included, otherwise the last lane it attempted."""
    return unit["servingLane"] or unit["meta"]["lane"]


def unit_fate(acc, unit):
    """Terminal state of a demand unit: included / abandoned / unresolved.

    Lineage traces carry explicit TxAbandoned events for every terminal
    failure, so only those are trusted (a rejected attempt may still have a
    retry queued). Legacy traces (no lineage fields) predate both retries and
    abandonment events, so there a rejected/evicted attempt IS the end of its
    demand unit and abandonment is inferred."""
    if unit["includedAt"] is not None:
        return "included"
    if unit["abandonedAt"] is not None:
        return "abandoned"
    if not acc.has_lineage and (
            unit["lastTxId"] in acc.rejected or unit["lastTxId"] in acc.evicted):
        return "abandoned"
    return "unresolved"


class Accumulator:
    """Single-pass accumulator over a SimEvent trace.

    Two keyings coexist, mirroring the simulator's metrics: per-attempt maps
    keyed by txId (load, flow, price/block series), and demand units keyed by
    origin tx number — the user's underlying intent, however many submission
    attempts it took. Fate, value, latency, and fairness read the units."""

    def __init__(self):
        self.submitted_at = {}              # txId -> submit slot (per attempt)
        self.tx_meta = {}                   # txId -> {"tag", "rate", "lane"}
        self.tx_value = {}                  # txId -> value (lovelace)
        self.tx_actor = {}                  # txId -> actorId
        self.tx_origin = {}                 # txId -> demand-unit origin number
        self.included_at = {}               # txId -> inclusion slot
        self.included_route = {}            # txId -> "IncludedInRb" | "IncludedInEb"
        self.rejected = set()               # txIds rejected at admission
        self.evicted = set()                # txIds evicted at selection (fee too low)
        self.units = {}                     # origin -> demand-unit dict
        self.has_lineage = False            # trace carries originNumber/attempt fields
        self.attempt_count = 0              # total submission attempts
        self.attempts_max = 0               # most attempts by any single unit
        self.price_changes = defaultdict(list)     # lane -> [PriceUpdated event]
        self.submissions_per_slot = defaultdict(int)
        self.inclusions_per_slot = defaultdict(int)
        self.rb_count = 0                   # ranking blocks produced (total)
        self.rb_tx_count = 0                # RBs carrying txs directly (PraosBlock)
        self.rb_cert_count = 0              # RBs carrying an EB certificate (CertifyingBlock)
        self.rb_series = []                 # [{slot, kind, fill}] per RB, in order
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
            self.tx_value[tx_id] = tx.get("value")
            self.tx_actor[tx_id] = event.get("actorId")
            self.submissions_per_slot[tx["submitted"]] += 1
            self._ingest_attempt(event, tx, tx_id)
        elif tag == "TxIncluded":
            tx_id = event["txId"]
            self.included_at[tx_id] = slot
            self.included_route[tx_id] = event.get("inclusionPoint", {}).get("tag")
            self.inclusions_per_slot[slot] += 1
            unit = self.units.get(self.tx_origin.get(tx_id))
            if unit is not None and unit["includedAt"] is None and unit["abandonedAt"] is None:
                unit["includedAt"] = slot
                unit["route"] = self.included_route[tx_id]
                unit["servingLane"] = self.tx_meta.get(tx_id, {}).get("lane")
        elif tag == "TxRejected":
            self.rejected.add(event["txId"])
        elif tag == "TxEvicted":
            self.evicted.add(event["txId"])
        elif tag == "TxAbandoned":
            # the moment a demand unit's remaining value is definitively lost:
            # its actor declined to resubmit, or it ran out of attempts
            unit = self.units.get(event.get("originNumber"))
            if unit is not None and unit["includedAt"] is None and unit["abandonedAt"] is None:
                unit["abandonedAt"] = slot
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

    def _ingest_attempt(self, event, tx, tx_id):
        """Fold one submission attempt into its demand unit. Legacy traces
        without lineage fields degrade to one unit per tx (origin = own id,
        attempt 1), which reproduces the old per-tx semantics exactly."""
        if "originNumber" in tx:
            self.has_lineage = True
        origin = tx.get("originNumber", tx_id)
        attempt = tx.get("attempt", 1)
        self.tx_origin[tx_id] = origin
        self.attempt_count += 1
        if attempt > self.attempts_max:
            self.attempts_max = attempt
        unit = self.units.get(origin)
        if unit is None:
            # events are chronological: the first sighting is the first
            # attempt, which fixes the unit's origin facts
            self.units[origin] = {
                "firstSubmitted": tx.get("originSubmitted", tx["submitted"]),
                "meta": dict(self.tx_meta[tx_id]),
                "value": tx.get("value") or 0,
                "actor": event.get("actorId"),
                "attempts": attempt,
                "lastTxId": tx_id,
                "includedAt": None,
                "route": None,
                "servingLane": None,
                "abandonedAt": None,
            }
        elif attempt >= unit["attempts"]:
            unit["attempts"] = attempt
            unit["meta"]["lane"] = tx["lane"]
            unit["lastTxId"] = tx_id

    @property
    def slot_count(self):
        return self.max_slot + 1
