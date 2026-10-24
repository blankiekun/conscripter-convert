#!/bin/bash
# play.sh <scratch> <name> <seconds> [query]: web run plus a screenshot every EVERY (20) s into out/play/<name>/
cd /home/saya/Projects/conscripter-convert-muramasa-play
S=$1; N=$2; T=$3; Q=${4:-auto-play=0&auto-choice=random&seed=3}
PORT=$((9300 + RANDOM % 600))
mkdir -p out/play/$N
CDP_PORT=$PORT CHROME=$PWD/research/nitroplus/web_play/chrome-wrap.sh timeout -k 5 $((T+30)) python3 -I engine/tools/web_run.py engine/build_web $S -o out/play/$N/trace.jsonl --timeout $T --query "$Q" > out/play/$N/run.log 2>&1 &
W=$!
python3 research/nitroplus/web_play/webshots.py out/play/$N/shots --port $PORT --every ${EVERY:-20} --for $T 2> out/play/$N/shots.log
wait $W; echo "web_run exit $?" >> out/play/$N/run.log
