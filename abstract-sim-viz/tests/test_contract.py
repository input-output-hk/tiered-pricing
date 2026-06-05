from simviz.ingest import Accumulator
from simviz.contract import urgency_classes


def _submitted(tx_id, rate, tag="Exponential"):
    return {"tag": "TxSubmitted", "slot": 0, "actorId": 0,
            "tx": {"id": tx_id, "lane": "Standard", "submitted": 0, "value": 1,
                   "urgency": {"tag": tag, "rate": rate},
                   "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                            "dependsOn": [], "fee": 1}}}


def test_urgency_classes_ordered_low_to_high_rate():
    acc = Accumulator()
    acc.ingest(_submitted(1, 6.0e-3))
    acc.ingest(_submitted(2, 5.0e-4))
    acc.ingest(_submitted(3, 5.0e-4))   # duplicate class
    classes = urgency_classes(acc)
    assert [c["rate"] for c in classes] == [5.0e-4, 6.0e-3]
    assert classes[0]["id"] == "Exponential:0.0005"
    assert classes[0]["label"] == "Exp λ=0.0005"
    assert classes[0]["tag"] == "Exponential"
