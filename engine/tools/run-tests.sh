#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
nox_command="${NOX:-nox}"

cd "$engine_directory"
if [[ "$(uname -s)" == Linux && -z "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
    export SDL_VIDEO_DRIVER="${SDL_VIDEO_DRIVER:-offscreen}"
fi
export LANTERN_SHADER_DIRECTORY="$engine_directory/shaders/compiled"
export LIBRARY_PATH="$engine_directory/generated/mist${LIBRARY_PATH:+:$LIBRARY_PATH}"
test_suites=()
for suite_directory in tests/*/; do
    test_suites+=("${suite_directory%/}")
done
"$nox_command" test "$@" "${test_suites[@]}"
