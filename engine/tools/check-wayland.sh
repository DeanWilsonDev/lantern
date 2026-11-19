#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
cd "$engine_directory"

for needed in weston; do
    command -v "$needed" > /dev/null || { echo "check-wayland: '$needed' is not installed (apt install weston mesa-vulkan-drivers)" >&2; exit 2; }
done

(cd "$engine_directory" && nox build --release --project . > /dev/null)

runtime_directory="$(mktemp -d)"
chmod 700 "$runtime_directory"
socket_name="lantern-check-wayland"
log_directory="build"
mkdir -p "$log_directory"

XDG_RUNTIME_DIR="$runtime_directory" weston --backend=headless --socket="$socket_name" --no-config --width=1280 --height=720 \
    > "$log_directory/check-wayland-compositor.log" 2>&1 &
compositor_process=$!

stop_compositor() {
    kill "$compositor_process" 2> /dev/null || true
    rm -rf "$runtime_directory"
}
trap stop_compositor EXIT

for _ in $(seq 1 50); do
    [[ -S "$runtime_directory/$socket_name" ]] && break
    sleep 0.1
done
[[ -S "$runtime_directory/$socket_name" ]] || { echo "check-wayland: the compositor did not start; see $log_directory/check-wayland-compositor.log" >&2; exit 1; }

run_on_wayland() {
    env -u DISPLAY XDG_RUNTIME_DIR="$runtime_directory" WAYLAND_DISPLAY="$socket_name" SDL_VIDEO_DRIVER=wayland \
        LANTERN_SHADER_DIRECTORY="$engine_directory/shaders/compiled" "$@"
}

check() {
    local log_path="$1" expected="$2"
    grep -q "$expected" "$log_path" || { echo "check-wayland: missing '$expected' in $log_path" >&2; exit 1; }
}

platformer_log="$log_directory/check-wayland-platformer.log"
run_on_wayland env LANTERN_AUTOPLAY=1 LANTERN_MAX_FRAMES=60 LANTERN_VSYNC=0 timeout 300 ./build/lantern > "$platformer_log" 2>&1
check "$platformer_log" "video driver: wayland"
check "$platformer_log" "GPU driver: vulkan"
check "$platformer_log" "frame loop finished after 60 frames"
echo "check-wayland: the platformer ran 60 frames on SDL's Wayland driver"

window_log="$log_directory/check-wayland-window.log"
run_on_wayland env LANTERN_WINDOW_SCRIPT=1 LANTERN_MAX_FRAMES=600 LANTERN_VSYNC=0 timeout 120 ./build/lantern --stub > "$window_log" 2>&1
check "$window_log" "video driver: wayland"
check "$window_log" "resized: 800x450 points"
check "$window_log" "\[stub\] minimised"
check "$window_log" "\[stub\] quit requested"
check "$window_log" "(0 while minimised)"
echo "check-wayland: the window resized, minimised and quit (Wayland has no way to restore a window the client minimised, so there is no restore check)"
