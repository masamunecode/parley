#!/bin/zsh
# ./snap.sh [seconds] [app flags...]  -> builds (debug), launches the app unfocused, screenshots its window, prints the PNG path
cd "$(dirname "$0")"
SEC=4; if [[ "$1" =~ '^[0-9]+(\.[0-9]+)?$' ]]; then SEC=$1; shift; fi
OUT=$(cargo build --message-format=json-render-diagnostics 2>.snaps-build.log) || { tail -60 .snaps-build.log; echo "BUILD FAILED"; exit 1; }
BIN=$(print -r -- "$OUT" | python3 -I -c 'import sys,json
e=None
for l in sys.stdin:
  try: m=json.loads(l)
  except: continue
  if m.get("reason")=="compiler-artifact" and m.get("executable"): e=m["executable"]
print(e or "")')
rm -f .snaps-build.log
[[ -z "$BIN" ]] && { echo "no executable built"; exit 1; }
mkdir -p .snaps; PNG="$PWD/.snaps/$(date +%H%M%S).png"
SNAP=1 "$BIN" "$@" > .snaps/app.log 2>&1 & PID=$!
sleep $SEC
if ! kill -0 $PID 2>/dev/null; then echo "APP EXITED"; tail -40 .snaps/app.log; exit 1; fi
RSS=$(ps -o rss= -p $PID | tr -d ' ')
~/masamune/tools/wincap/wincap shotpid $PID "$PNG" >/dev/null || echo "no window found"
kill $PID 2>/dev/null; wait $PID 2>/dev/null
echo "$PNG"; echo "rss ${RSS} KB"; tail -15 .snaps/app.log
