import json
import math
from datetime import datetime, timezone

from simviz import price as price_mod
from simviz import load as load_mod
from simviz import latency as latency_mod
from simviz.stats import quantile, histogram_bins

DEFAULT_PARAMS = {"shockThreshold": 0.10, "convergenceBandPct": 0.05, "loadChangePct": 0.10}


def _format_rate(rate):
    return f"{rate:g}"


def urgency_label(tag, rate):
    short = {"Exponential": "Exp", "Linear": "Lin"}.get(tag, tag)
    return f"{short} λ={_format_rate(rate)}"


def urgency_classes(acc):
    """Distinct urgency classes present, ordered by rate low -> high."""
    keys = {(m["tag"], m["rate"]) for m in acc.tx_meta.values()}
    classes = []
    for tag, rate in sorted(keys, key=lambda k: k[1]):
        classes.append({
            "id": latency_mod.class_id(tag, rate),
            "tag": tag, "rate": rate,
            "label": urgency_label(tag, rate),
        })
    return classes
