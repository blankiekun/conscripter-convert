#!/usr/bin/env python3
"""PostToolUse hook: tell a subagent past 100k tokens of context to hand off.

The cap (user, 2026-10-19; raised 2026-10-23): no agent past 120k context. At 100k the agent
commits/pushes its WIP, writes HANDOFF.md in its worktree, reports and
stops; the lead starts a fresh agent from that file. The main session
(the lead) isn't checked. An agent planned over the cap may go on: its
prompt says so and gives the budget.
"""
import json
import os
import sys

WARN, CAP = 100_000, 120_000


def context(path):
    """Input tokens of the transcript's last assistant turn (its context)."""
    with open(path, "rb") as f:
        f.seek(max(0, os.path.getsize(path) - 300_000))
        lines = f.read().decode("utf-8", "ignore").splitlines()[1:]
    for line in reversed(lines):
        try:
            u = json.loads(line).get("message", {}).get("usage")
        except (ValueError, AttributeError):
            continue
        if isinstance(u, dict):
            return sum(u.get(k, 0) for k in ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"))
    return 0


def main(hook):
    agent = hook.get("agent_id")
    path = hook.get("agent_transcript_path")
    if agent and not path and hook.get("transcript_path"):
        path = os.path.join(hook["transcript_path"][: -len(".jsonl")], "subagents", f"agent-{agent}.jsonl")
    if not path and "/subagents/" in hook.get("transcript_path", ""):
        path = hook["transcript_path"]
    if not path or not os.path.isfile(path):
        return None
    n = context(path)
    if n < WARN:
        return None
    what = "PAST THE 120k CAP" if n >= CAP else "near the 120k cap"
    return (f"Context is {n // 1000}k tokens, {what}. Unless your prompt plans this task over the cap: "
            "commit and push your WIP now, write HANDOFF.md in your worktree (done, in progress, running "
            "jobs, next), report to the lead and stop. Start long runs with nohup/setsid so they outlive you.")


if __name__ == "__main__":
    try:
        msg = main(json.load(sys.stdin))
    except (OSError, ValueError):
        sys.exit(0)  # never block a tool over this check
    if msg:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": msg}}))
