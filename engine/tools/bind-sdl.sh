#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
nox_command="${NOX:-nox}"
sdl_include_directory="${1:-}"

if [[ -z "$sdl_include_directory" ]]; then
    for candidate_directory in /opt/homebrew/include /usr/local/include /usr/include; do
        if [[ -f "$candidate_directory/SDL3/SDL.h" ]]; then
            sdl_include_directory="$candidate_directory"
            break
        fi
    done
fi

if [[ -z "$sdl_include_directory" || ! -f "$sdl_include_directory/SDL3/SDL.h" ]]; then
    echo "usage: bind-sdl.sh [SDL3 include directory], the directory holding SDL3/SDL.h" >&2
    exit 2
fi

mkdir -p "$engine_directory/generated"
"$nox_command" bind-c "$sdl_include_directory/SDL3/SDL.h" "-I$sdl_include_directory" --from "$sdl_include_directory/SDL3" \
    --library SDL3 --namespace Sdl -o "$engine_directory/generated/sdl.nox" 2> "$engine_directory/generated/sdl-binding-notes.txt"
echo "wrote generated/sdl.nox; notes in generated/sdl-binding-notes.txt"
