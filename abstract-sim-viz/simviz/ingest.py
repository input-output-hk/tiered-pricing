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
