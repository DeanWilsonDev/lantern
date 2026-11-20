#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
nox_command="${NOX:-nox}"

cd "$engine_directory"
if [[ "$(uname -s)" == Linux && -z "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
    export SDL_VIDEO_DRIVER="${SDL_VIDEO_DRIVER:-offscreen}"
fi
export LANTERN_SHADER_DIRECTORY="$engine_directory/shaders/compiled"
test_suites=()
for suite_directory in tests/*/; do
    [[ "$suite_directory" == tests/screenshots/ ]] && continue
    test_suites+=("${suite_directory%/}")
done
"$nox_command" test "$@" "${test_suites[@]}"

if [[ $# -eq 0 ]]; then
    "$engine_directory/tools/run-screenshot-tests.sh"
    "$nox_command" run --release --project tools/generate-assets -- --check "$engine_directory/../games/platformer/assets"
fi

editor_project="$engine_directory/../apps/world-editor/nox"
if [[ -d "$editor_project" && $# -eq 0 ]]; then
    editor_check_output="$(mktemp)"
    trap 'rm -f "$editor_check_output"' EXIT
    "$nox_command" build --release --project "$engine_directory" > /dev/null
    "$nox_command" build --release --project "$editor_project" -o "$editor_check_output" > /dev/null
    echo "the world editor builds"
fi
