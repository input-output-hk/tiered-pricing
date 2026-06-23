from simviz.ingest import iter_events, Accumulator


def test_iter_events_yields_inner_event_objects(tmp_path):
    f = tmp_path / "trace.jsonl"
    f.write_text(
        '{"event":{"tag":"TxAdmitted","slot":0,"txId":1},"eventNo":0}\n'
        '\n'  # blank line is skipped
        '{"event":{"tag":"TxAdmitted","slot":1,"txId":2},"eventNo":1}\n'
    )
    events = list(iter_events(str(f)))
    assert events == [
        {"tag": "TxAdmitted", "slot": 0, "txId": 1},
        {"tag": "TxAdmitted", "slot": 1, "txId": 2},
    ]


def _submitted(tx_id, slot, lane, rate, tag="Exponential"):
    return {
        "tag": "TxSubmitted", "slot": slot, "actorId": 0,
        "tx": {
            "id": tx_id, "lane": lane, "submitted": slot, "value": 100,
            "urgency": {"tag": tag, "rate": rate},
            "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                     "dependsOn": [], "fee": 1},
        },
    }


def _included(tx_id, slot):
    return {"tag": "TxIncluded", "slot": slot, "txId": tx_id,
            "inclusionPoint": {"tag": "IncludedInRb"}}


def _price(lane, slot, old, new, util):
    return {"tag": "PriceUpdated", "slot": slot, "lane": lane,
            "oldCoeff": old, "newCoeff": new, "utilisation": util}


def test_accumulator_records_state():
    acc = Accumulator()
    for e in [
        _submitted(1, 0, "Standard", 5.0e-4),
        _included(1, 2),
        _price("Priority", 1, 16, 10, 0.4),
    ]:
        acc.ingest(e)
    assert acc.submitted_at == {1: 0}
    assert acc.included_at == {1: 2}
    assert acc.included_route[1] == "IncludedInRb"
    assert acc.tx_meta[1] == {"tag": "Exponential", "rate": 5.0e-4, "lane": "Standard"}
    assert acc.submissions_per_slot[0] == 1
    assert acc.inclusions_per_slot[2] == 1
    assert acc.price_changes["Priority"][0]["newCoeff"] == 10
    assert acc.slot_count == 3          # max slot 2 -> +1
    assert acc.total_events == 3


def _rb(slot, block_tag):
    return {"tag": "BlockProduced", "slot": slot,
            "summary": {"tag": "RankingBlockProduced", "summary": {"block": {"tag": block_tag}}}}


def test_accumulator_counts_rb_content():
    acc = Accumulator()
    acc.ingest(_rb(10, "PraosBlock"))       # carries txs
    acc.ingest(_rb(20, "CertifyingBlock"))  # certifies an EB
    acc.ingest(_rb(30, "PraosBlock"))
    assert acc.rb_count == 3
    assert acc.rb_tx_count == 2
    assert acc.rb_cert_count == 1
    assert acc.rb_series == [
        {"slot": 10, "kind": "txs", "fill": None},   # no capacity in fixture -> fill None
        {"slot": 20, "kind": "cert", "fill": None},
        {"slot": 30, "kind": "txs", "fill": None},
    ]


def test_rb_fullness_is_binding_utilisation():
    acc = Accumulator()
    acc.ingest({"tag": "BlockProduced", "slot": 5, "summary": {
        "tag": "RankingBlockProduced", "summary": {
            "block": {"tag": "PraosBlock"},
            "usedBytes": 45, "capacityBytes": 90,        # 0.5 by bytes
            "usedExUnits": 20, "capacityExUnits": 100}}})  # 0.2 by ex-units
    assert acc.rb_series[0]["fill"] == 0.5               # binding = max(0.5, 0.2)


def test_cert_block_shaded_by_certified_eb_fullness():
    acc = Accumulator()
    # EB id 7 announced 30% full (bytes), 0% by ex-units -> binding 0.3
    acc.ingest({"tag": "BlockProduced", "slot": 5, "summary": {
        "tag": "EndorserBlockAnnounced", "summary": {
            "id": 7, "usedBytes": 30, "capacityBytes": 100,
            "usedExUnits": 0, "capacityExUnits": 100}}})
    # a later RB certifies EB 7
    acc.ingest({"tag": "BlockProduced", "slot": 18, "summary": {
        "tag": "RankingBlockProduced", "summary": {"block": {"tag": "CertifyingBlock", "ebId": 7}}}})
    assert acc.eb_fullness[7] == 0.3
    assert acc.rb_series[0] == {"slot": 18, "kind": "cert", "fill": 0.3}


def test_accumulator_records_value_rejected_evicted():
    acc = Accumulator()
    acc.ingest(_submitted(1, 0, "Standard", 0.01))   # helper sets value=1
    acc.ingest(_submitted(2, 0, "Priority", 0.01))
    acc.ingest(_submitted(3, 0, "Standard", 0.01))
    acc.ingest({"tag": "TxRejected", "slot": 1, "txId": 2, "reasons": [{"tag": "MempoolFull"}]})
    acc.ingest({"tag": "TxEvicted", "slot": 2, "txId": 3, "reason": {"tag": "FeeTooLowAtSelection"}})
    assert acc.tx_value[1] == 100   # _submitted helper sets value=100
    assert acc.tx_actor[1] == 0     # _submitted helper sets actorId=0
    assert acc.rejected == {2}
    assert acc.evicted == {3}


def test_accumulator_last_wins_on_duplicate_txid():
    acc = Accumulator()
    acc.ingest(_submitted(1, 0, "Standard", 5.0e-4))
    acc.ingest(_submitted(1, 5, "Priority", 6.0e-3))   # resubmission, same id
    acc.ingest(_included(1, 3))
    acc.ingest(_included(1, 9))                         # later inclusion wins
    assert acc.submitted_at[1] == 5
    assert acc.tx_meta[1]["lane"] == "Priority"
    assert acc.included_at[1] == 9


def _attempt(tx_id, slot, lane, rate, origin, attempt, origin_submitted):
    """A lineage-bearing submission: a retry chain attempt of a demand unit."""
    e = _submitted(tx_id, slot, lane, rate)
    e["tx"].update({"originNumber": origin, "attempt": attempt,
                    "originSubmitted": origin_submitted})
    return e


def test_demand_units_chain_retry_attempts():
    from simviz.ingest import unit_fate, unit_lane
    acc = Accumulator()
    for e in [
        # unit 1: rejected on attempt 1, served on attempt 2 (lane switched)
        _attempt(1, 0, "Standard", 0.01, origin=1, attempt=1, origin_submitted=0),
        {"tag": "TxRejected", "slot": 0, "txId": 1, "reasons": [{"tag": "FeeTooLow"}]},
        _attempt(2, 5, "Priority", 0.01, origin=1, attempt=2, origin_submitted=0),
        _included(2, 9),
        # unit 3: abandoned outright
        _attempt(3, 1, "Standard", 0.01, origin=3, attempt=1, origin_submitted=1),
        {"tag": "TxAbandoned", "slot": 4, "originNumber": 3},
        # unit 4: still in flight at the end of the trace
        _attempt(4, 2, "Standard", 0.01, origin=4, attempt=1, origin_submitted=2),
        {"tag": "TxRejected", "slot": 2, "txId": 4, "reasons": [{"tag": "FeeTooLow"}]},
    ]:
        acc.ingest(e)
    assert acc.has_lineage
    assert acc.attempt_count == 4 and acc.attempts_max == 2
    served = acc.units[1]
    assert served["attempts"] == 2
    assert served["firstSubmitted"] == 0       # decay/latency anchor: attempt 1
    assert served["includedAt"] == 9
    assert unit_lane(served) == "Priority"     # the lane that actually served it
    assert unit_fate(acc, served) == "included"
    assert unit_fate(acc, acc.units[3]) == "abandoned"
    # lineage traces trust explicit abandonment only: unit 4's rejection may
    # still have a retry queued, so it is unresolved, not abandoned
    assert unit_fate(acc, acc.units[4]) == "unresolved"


def test_legacy_traces_fall_back_to_per_tx_units():
    from simviz.ingest import unit_fate
    acc = Accumulator()
    acc.ingest(_submitted(1, 0, "Standard", 0.01))
    acc.ingest({"tag": "TxRejected", "slot": 0, "txId": 1, "reasons": [{"tag": "MempoolFull"}]})
    acc.ingest(_submitted(2, 1, "Standard", 0.01))
    acc.ingest(_included(2, 3))
    assert not acc.has_lineage
    # without lineage there are no retries: a rejected attempt IS its unit's end
    assert unit_fate(acc, acc.units[1]) == "abandoned"
    assert unit_fate(acc, acc.units[2]) == "included"
    assert acc.units[2]["firstSubmitted"] == 1
