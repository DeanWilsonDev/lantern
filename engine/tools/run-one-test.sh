#!/usr/bin/env bash
set -euo pipefail

test_name="${1:?usage: run-one-test.sh TEST-NAME}"
engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="${LANTERN_PREFIX:-$HOME/.local}/bin:$PATH"
export LANTERN_SHADER_DIRECTORY="$engine_directory/shaders/compiled"
if [[ "$(uname -s)" == Linux && -z "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
    export SDL_VIDEO_DRIVER="${SDL_VIDEO_DRIVER:-offscreen}"
fi

cd "$engine_directory"
test_main_file="$(find tests -path "*/$test_name/main.nox" | head -n 1)"
[[ -n "$test_main_file" ]] || { echo "no test program named $test_name under engine/tests" >&2; exit 1; }
test_directory="$(dirname "$test_main_file")"
built_program="$(mktemp -d)/$test_name"
(cd "$test_directory" && nox build --project . -o "$built_program" >/dev/null)
"$built_program"
