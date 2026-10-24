#!/bin/bash
# chrome-wrap.sh: the browser web_run.py uses (CHROME=): muted, ja-JP, with a CDP port for webshots.py
exec /usr/bin/chromium --mute-audio --lang=ja-JP --remote-debugging-port=${CDP_PORT:-9341} --window-size=1280,760 "$@"
