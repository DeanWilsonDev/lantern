#!/usr/bin/env bash
# Runs the screenshot comparison tests (engine/tests/screenshots) on the software Vulkan driver, Mesa's lavapipe.
#
#   engine/tools/run-screenshot-tests.sh            compare every scene with its reference
#   engine/tools/run-screenshot-tests.sh --update   record the scenes as the new references
#   engine/tools/run-screenshot-tests.sh --require  fail instead of skipping when lavapipe isn't installed
#
# The references are tied to lavapipe because a real GPU, and Metal, round colours differently. This script points
# Vulkan at lavapipe alone, so the same pictures come out on a machine that has a GPU too. A failed comparison writes
# NAME.actual.png and NAME.diff.png into engine/build/screenshot-failures. See docs/screenshot-tests.md.
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
nox_command="${NOX:-nox}"
update_references=0
require_software_renderer=0
for argument in "$@"; do
    case "$argument" in
        --) ;;
        --update) update_references=1 ;;
        --require) require_software_renderer=1 ;;
        *) echo "usage: run-screenshot-tests.sh [--update] [--require]" >&2; exit 2 ;;
    esac
done

software_driver_file=""
for candidate in /usr/share/vulkan/icd.d/lvp_icd*.json /etc/vulkan/icd.d/lvp_icd*.json /usr/local/share/vulkan/icd.d/lvp_icd*.json; do
    if [[ -f "$candidate" ]]; then
        software_driver_file="$candidate"
        break
    fi
done
if [[ -z "$software_driver_file" ]]; then
    echo "screenshot tests: skipped, lavapipe isn't installed (apt-get install mesa-vulkan-drivers)"
    if [[ "$require_software_renderer" == 1 ]]; then
        exit 1
    fi
    exit 0
fi

cd "$engine_directory"
export VK_DRIVER_FILES="$software_driver_file"
export VK_ICD_FILENAMES="$software_driver_file"
export LANTERN_GPU_DRIVER=vulkan
export LANTERN_REQUIRE_GPU=1
export LANTERN_SCREENSHOT_SOFTWARE=1
export LANTERN_SCREENSHOT_REFERENCES="$engine_directory/tests/screenshots/references"
export LANTERN_SCREENSHOT_OUTPUT="$engine_directory/build/screenshot-failures"
if [[ "$(uname -s)" == Linux && -z "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
    export SDL_VIDEO_DRIVER="${SDL_VIDEO_DRIVER:-offscreen}"
fi
export LANTERN_SHADER_DIRECTORY="$engine_directory/shaders/compiled"
if [[ "$update_references" == 1 ]]; then
    export LANTERN_UPDATE_SCREENSHOTS=1
else
    rm -rf "$LANTERN_SCREENSHOT_OUTPUT"
fi

"$nox_command" test tests/screenshots

if [[ "$update_references" == 1 ]]; then
    echo "screenshot tests: references recorded in tests/screenshots/references; look at them before committing"
fi
