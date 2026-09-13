#!/usr/bin/env bash
# Capture documentation screenshots for book/src/screenshots.md
#
# Themes come from the operator's normal tool configs (Helix, Yazi, Lazygit,
# Zellij, …). This script only forces truecolor + a Nerd Font so colors and
# icons render correctly under Xvfb/xterm.
set -u

REPO=/home/vincent/Codes/sat-helix-ide
ASSETS="$REPO/book/src/assets"
# Prefer a freshly built binary so new features (e.g. __mindmap) are available.
HXIDE="${HXIDE:-$REPO/target/release/sat-hx-ide}"
if [[ ! -x "$HXIDE" ]]; then
  HXIDE=$(command -v sat-hx-ide || true)
fi
if [[ -z "$HXIDE" ]]; then
  echo "sat-hx-ide binary not found; build with: cargo build --release" >&2
  exit 1
fi
SESSION=docshots
RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/sat-helix-ide/$SESSION"
CONFIG="$RUNTIME/config.kdl"
LAYOUT="$RUNTIME/layout.kdl"
DISPLAY_NUM=99
export DISPLAY=":${DISPLAY_NUM}"
export ZELLIJ_SESSION_NAME="$SESSION"
# Critical: plain TERM=xterm is only 8 colors and makes Helix blues nearly invisible.
export TERM=xterm-direct
export COLORTERM=truecolor
XVFB_PID=""
XTERM_PID=""

log() { printf '[capture] %s\n' "$*"; }

cleanup() {
  local code=$?
  zellij kill-session "$SESSION" >/dev/null 2>&1 || true
  zellij delete-session "$SESSION" --force >/dev/null 2>&1 || true
  if [[ -n "${XTERM_PID}" ]]; then kill "$XTERM_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${XVFB_PID}" ]]; then kill "$XVFB_PID" >/dev/null 2>&1 || true; fi
  exit "$code"
}
trap cleanup EXIT

SRC_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/sat-helix-ide/sat-helix-ide"
if [[ ! -f "$SRC_RUNTIME/config.kdl" ]]; then
  log "missing $SRC_RUNTIME/config.kdl — run sat-hx-ide init once first"
  exit 1
fi

mkdir -p "$RUNTIME" "$ASSETS"
cp "$SRC_RUNTIME/config.kdl" "$CONFIG"

# Use the operator's Helix config/theme; only pin cwd + pane names for capture.
HX_BIN=$(command -v hx || echo /snap/bin/hx)
cat >"$LAYOUT" <<EOF
layout {
    tab name="code" focus=true {
        pane split_direction="horizontal" {
            pane name="editor" command="$HX_BIN" cwd="$REPO"
            pane name="terminal" size="20%" cwd="$REPO"
        }
    }
}
EOF

# Stop any previous docshots session without pkill -f patterns that match this script.
zellij kill-session "$SESSION" >/dev/null 2>&1 || true
zellij delete-session "$SESSION" --force >/dev/null 2>&1 || true

# Prefer a free display if :99 is taken
if [[ -e /tmp/.X11-unix/X${DISPLAY_NUM} ]]; then
  DISPLAY_NUM=109
  export DISPLAY=":${DISPLAY_NUM}"
fi

log "starting Xvfb on $DISPLAY"
Xvfb "$DISPLAY" -screen 0 1600x1000x24 >/tmp/xvfb-docshots.log 2>&1 &
XVFB_PID=$!
sleep 1
if ! kill -0 "$XVFB_PID" 2>/dev/null; then
  log "Xvfb failed to start"; cat /tmp/xvfb-docshots.log; exit 1
fi

log "starting xterm + zellij (truecolor; user themes)"
# Yazi (and many TUI icons) need a Nerd Font; DejaVu shows tofu placeholders.
# -tn xterm-direct: advertise truecolor terminfo to Helix/Zellij.
xterm -geometry 170x50 \
  -tn xterm-direct \
  -fa 'JetBrainsMono Nerd Font Mono' -fs 9 \
  -title "sat-hx-ide-docshots" \
  -e "export TERM=xterm-direct COLORTERM=truecolor; cd '$REPO' && exec zellij --session '$SESSION' --config '$CONFIG' --new-session-with-layout '$LAYOUT'" &
XTERM_PID=$!
sleep 5

WID=""
for _ in $(seq 1 20); do
  # Zellij rewrites the window title; match the xterm process instead.
  WID=$(xdotool search --pid "$XTERM_PID" 2>/dev/null | tail -n1 || true)
  if [[ -z "$WID" ]]; then
    WID=$(xdotool search --class XTerm 2>/dev/null | tail -n1 || true)
  fi
  if [[ -n "$WID" ]]; then break; fi
  sleep 0.4
done
log "WID=$WID"
if [[ -z "$WID" ]]; then
  log "no xterm window found"
  cat /tmp/xvfb-docshots.log || true
  exit 1
fi

Z() { zellij --session "$SESSION" "$@"; }

for _ in $(seq 1 20); do
  if Z action list-panes --json --all >/tmp/docshots-panes.json 2>/dev/null; then
    break
  fi
  sleep 0.4
done

capture() {
  local name=$1
  sleep 1.2
  import -window "$WID" "$ASSETS/${name}.png"
  ls -lh "$ASSETS/${name}.png"
}

pane_id_by_title() {
  local title=$1
  Z action list-panes --json --all | python3 -c '
import json,sys
title=sys.argv[1]
for p in json.load(sys.stdin):
    if p.get("title")==title and not p.get("is_plugin"):
        print("terminal_"+str(p["id"]))
        break
' "$title"
}

close_by_title() {
  local title=$1 id
  id=$(pane_id_by_title "$title" || true)
  if [[ -n "${id:-}" ]]; then
    Z action close-pane -p "$id" || true
  fi
}

open_floating() {
  local name=$1; shift
  Z action new-pane --close-on-exit --name "$name" --floating \
    --cwd "$REPO" --x 0% --y 0% --width 100% --height 100% -- \
    env \
      "TERM=xterm-direct" \
      "COLORTERM=truecolor" \
      "$@"
}

ED=$(pane_id_by_title editor)
TERM_PANE=$(pane_id_by_title terminal)
log "using binary: $HXIDE ($("$HXIDE" --version 2>/dev/null || echo unknown))"
log "editor=$ED terminal=$TERM_PANE"
if [[ -z "$ED" || -z "$TERM_PANE" ]]; then
  log "missing panes"; cat /tmp/docshots-panes.json || true; exit 1
fi

Z action write-chars -p "$ED" ":open $REPO/src/main.rs"
Z action send-keys -p "$ED" Enter
sleep 1

Z action write-chars -p "$TERM_PANE" 'clear; printf "\$ cargo build\n"; cargo build -q; echo "    Finished \`dev\` profile"'
Z action send-keys -p "$TERM_PANE" Enter
sleep 2
capture initial-workspace

Z action write-chars -p "$TERM_PANE" 'clear; printf "\$ cargo test --locked\n"; echo "    Running unittests src/main.rs"; echo "    test result: ok."'
Z action send-keys -p "$TERM_PANE" Enter
sleep 1
capture terminal-visible

log "terminal-hidden"
"$HXIDE" __terminal toggle || true
sleep 1
capture terminal-hidden
"$HXIDE" __terminal toggle || true
sleep 0.5

log "terminal-zoomed"
"$HXIDE" __terminal zoom || true
sleep 1
Z action write-chars -p "$TERM_PANE" 'clear; printf "\$ cargo build --release\n"; echo "    Compiling sat-helix-ide"; echo "    Finished \`release\` profile"'
Z action send-keys -p "$TERM_PANE" Enter
sleep 1
capture terminal-zoomed
"$HXIDE" __terminal zoom || true
sleep 0.5

log "yazi-fullscreen"
open_floating file-manager yazi "$REPO" >/tmp/yazi-float.out 2>&1 &
sleep 2.5
capture yazi-fullscreen
close_by_title file-manager
sleep 0.5

log "yazi-docked"
Z run --close-on-exit --borderless true --floating --width 1 --height 1 --cwd "$REPO" -- \
  env TERM=xterm-direct COLORTERM=truecolor \
  "$HXIDE" __file-manager open >/tmp/yazi-open.out 2>&1 &
sleep 2
Z run --close-on-exit --borderless true --floating --width 1 --height 1 --cwd "$REPO" -- \
  env TERM=xterm-direct COLORTERM=truecolor \
  "$HXIDE" __file-manager toggle-dock >/tmp/yazi-dock.out 2>&1 &
sleep 2.5
capture yazi-docked

log "development-workflow"
Z action write-chars -p "$ED" ":open $REPO/src/actions.rs"
Z action send-keys -p "$ED" Enter
Z action write-chars -p "$TERM_PANE" 'clear; printf "\$ cargo test --locked\n"; cargo test --locked -q || true'
Z action send-keys -p "$TERM_PANE" Enter
sleep 4
capture development-workflow
close_by_title file-manager
sleep 0.5

log "lazygit-integration"
open_floating git lazygit >/tmp/lazygit.out 2>&1 &
sleep 2.5
capture lazygit-integration
close_by_title git
sleep 0.5

log "review-workflow"
# Default working-tree mode can be empty; show a recent commit range instead.
open_floating review revdiff HEAD~5 HEAD >/tmp/revdiff.out 2>&1 &
sleep 2.5
capture review-workflow
close_by_title review
sleep 0.5

log "mindmap-docked"
"$HXIDE" __mindmap toggle >/tmp/mindmap.out 2>&1 || true
sleep 2.5
capture mindmap-docked
"$HXIDE" __mindmap toggle >/dev/null 2>&1 || true

log "done"
ls -lh "$ASSETS"/*.png
