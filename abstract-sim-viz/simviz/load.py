import math
from simviz.stats import mean


def bucket_width(slot_count, target_buckets=300):
    return max(1, math.ceil(slot_count / target_buckets)) if slot_count > 0 else 1


def load_buckets(submissions_per_slot, inclusions_per_slot, slot_count, width):
    buckets = []
    for start in range(0, slot_count, width):
        end = min(start + width, slot_count)
        subs = sum(submissions_per_slot.get(s, 0) for s in range(start, end))
        incs = sum(inclusions_per_slot.get(s, 0) for s in range(start, end))
        buckets.append({"slot": start, "submissions": subs, "inclusions": incs})
    return buckets
