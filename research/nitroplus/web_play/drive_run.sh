#!/bin/bash
# drive_run.sh <game> <name> <timeout> <query> STEP...: web_run + drive.py
cd "$(dirname "$0")/../../.."
G=$1; N=$2; T=$3; Q=$4; shift 4
PORT=$((9300 + RANDOM % 600))
O=out/play/$N; rm -rf $O; mkdir -p $O
CDP_PORT=$PORT CHROME=$PWD/research/nitroplus/web_play/chrome-wrap.sh timeout -k 5 $((T+30)) python3 -I engine/tools/web_run.py engine/build_web $G -o $O/trace.jsonl --timeout $T --query "$Q" > $O/run.log 2>&1 &
W=$!
timeout $T python3 research/nitroplus/web_play/drive.py $O --port $PORT "$@" 2> $O/drive.err | tee $O/drive.log
kill $W 2>/dev/null; wait $W; echo "web_run exit $?" >> $O/run.log
