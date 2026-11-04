#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
mist_command="${MIST:-mist}"
linked_directory="$engine_directory/generated/mist"

install_output="$(cd "$engine_directory" && "$mist_command" install "$@")"
echo "$install_output"

rm -rf "$linked_directory"
mkdir -p "$linked_directory"
while IFS= read -r built_line; do
    built_files="${built_line#*: }"
    IFS=',' read -ra built_paths <<< "$built_files"
    for built_path in "${built_paths[@]}"; do
        built_path="${built_path# }"
        ln -s "$built_path" "$linked_directory/$(basename "$built_path")"
    done
done < <(echo "$install_output" | grep -E '^(Built|Already built) ')
echo "linked Mist's libraries and bindings into generated/mist/"
