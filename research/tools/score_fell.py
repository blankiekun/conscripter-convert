#!/usr/bin/env python3
"""Print each `score` channel (and the total, as_original) of a metrics.json
that fell by more than 0.01 against its snapshot; exit 1 if any did.
tests/snapshot.sh runs it as its score ratchet.
Usage: python3 -I research/tools/score_fell.py <snapshot> <new> <game>
       python3 -I research/tools/score_fell.py --self-test"""
import json, sys


def scores(totals):
    s = totals.get("score") or {}
    return {"total": s.get("total"), "as_original": s.get("as_original"), **s.get("channels", {})}


def fell(old, new):
    """(channel, old, new) for each channel of `new` below `old` by over 0.01."""
    o, n = scores(old), scores(new)
    return [(k, o[k], v) for k, v in n.items() if v is not None and o.get(k) is not None and v < o[k] - 0.01]


def self_test():
    old = {"score": {"total": 99.5, "as_original": 99.0, "channels": {"text": 98.0, "motion": 97.0}}}
    new = {"score": {"total": 99.495, "as_original": 99.2, "channels": {"text": 97.9, "motion": 97.0, "movie": 50.0}}}
    assert fell(old, new) == [("text", 98.0, 97.9)], fell(old, new)
    assert fell({}, new) == []  # an old snapshot without a score
    print("score_fell: ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        sys.exit(self_test())
    old, new = (json.load(open(f))["totals"] for f in sys.argv[1:3])
    for k, o, n in fell(old, new):
        print("%s: score %s %.2f -> %.2f" % (sys.argv[3], k, o, n))
    sys.exit(1 if fell(old, new) else 0)
