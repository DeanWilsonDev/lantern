#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
cd "$engine_directory"

display_number="${CHECK_WINDOW_DISPLAY:-:97}"
log_path="build/check-window.log"
mkdir -p build

started_server=""
if [ -z "${DISPLAY:-}" ]; then
    Xvfb "$display_number" -screen 0 1920x1080x24 >/dev/null 2>&1 &
    started_server=$!
    export DISPLAY="$display_number"
    sleep 1
    openbox >/dev/null 2>&1 &
    started_window_manager=$!
    sleep 1
fi

stop_servers() {
    if [ -n "$started_server" ]; then
        kill "$started_window_manager" "$started_server" 2>/dev/null || true
    fi
}
trap stop_servers EXIT

SDL_VIDEODRIVER=x11 LANTERN_WINDOW_SCRIPT=1 LANTERN_MAX_FRAMES=600 timeout 30 ./build/lantern > "$log_path" 2>&1
summary="$(tail -n 1 "$log_path")"
echo "$summary"

check() {
    if ! grep -q "$1" "$log_path"; then
        echo "check-window: missing '$1' in $log_path" >&2
        exit 1
    fi
}
check "resized: 800x450 points"
check "\[stub\] minimised"
check "\[stub\] restored"
check "\[stub\] quit requested"
check "(0 while minimised)"
check "1 minimises, 1 restores"
echo "check-window: the window resized, minimised, restored and quit"
