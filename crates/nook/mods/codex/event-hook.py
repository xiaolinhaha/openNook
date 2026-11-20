#!/usr/bin/env python3
"""Persist only Codex turn status and model for the openNook module."""
import json
import os
import sys
import tempfile
import time
from pathlib import Path


DEST = Path.home() / ".config/opennook/codex-turn-state.json"


def main():
    try:
        event = json.load(sys.stdin)
    except Exception:
        return 0

    name = event.get("hook_event_name")
    if name not in {"UserPromptSubmit", "Stop"}:
        return 0

    state = {"status": "running" if name == "UserPromptSubmit" else "finished", "timestamp": time.time()}
    if name == "Stop":
        try:
            previous = json.loads(DEST.read_text(encoding="utf-8"))
            if isinstance(previous.get("model"), str):
                state["model"] = previous["model"]
        except Exception:
            pass
    model = event.get("model")
    if isinstance(model, str) and model:
        state["model"] = model[:80]

    DEST.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=".turn-state-", dir=DEST.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            json.dump(state, stream, separators=(",", ":"))
            stream.write("\n")
        os.chmod(temp_name, 0o600)
        os.replace(temp_name, DEST)
    finally:
        try:
            os.unlink(temp_name)
        except FileNotFoundError:
            pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
