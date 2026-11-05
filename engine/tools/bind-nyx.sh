#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
repository_directory="$(cd "$engine_directory/.." && pwd)"
nox_command="${NOX:-nox}"
nyx_source_directory="${1:-${NYX_SOURCE_DIR:-}}"

if [[ -z "$nyx_source_directory" ]]; then
    for candidate_directory in "$repository_directory/../nyx-scripting-language" "$repository_directory/../nyx"; do
        if [[ -f "$candidate_directory/include/nyx/nyx.h" ]]; then
            nyx_source_directory="$candidate_directory"
            break
        fi
    done
fi

if [[ -z "$nyx_source_directory" || ! -f "$nyx_source_directory/include/nyx/nyx.h" ]]; then
    echo "usage: bind-nyx.sh [nyx-scripting-language checkout], or set NYX_SOURCE_DIR; it needs include/nyx/nyx.h" >&2
    exit 2
fi

nyx_include_directory="$(cd "$nyx_source_directory/include" && pwd)"
mkdir -p "$engine_directory/generated"
"$nox_command" bind-c "$nyx_include_directory/nyx/nyx.h" "-I$nyx_include_directory" --from "$nyx_include_directory/nyx" \
    --library lantern-native --namespace NyxC -o "$engine_directory/generated/nyx.nox" 2> "$engine_directory/generated/nyx-binding-notes.txt"
echo "wrote generated/nyx.nox; notes in generated/nyx-binding-notes.txt"
chaos_ui_include_directory="$engine_directory/native/include"
"$nox_command" bind-c "$chaos_ui_include_directory/lantern_chaos_ui.h" "-I$chaos_ui_include_directory" "-I$nyx_include_directory" \
    --from "$chaos_ui_include_directory" --library lantern-native --namespace ChaosUiC -o "$engine_directory/generated/chaos-ui.nox" \
    2> "$engine_directory/generated/chaos-ui-binding-notes.txt"
echo "wrote generated/chaos-ui.nox; notes in generated/chaos-ui-binding-notes.txt"
