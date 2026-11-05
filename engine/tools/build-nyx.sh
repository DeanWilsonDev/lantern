#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
repository_directory="$(cd "$engine_directory/.." && pwd)"
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
    echo "usage: build-nyx.sh [nyx-scripting-language checkout], or set NYX_SOURCE_DIR; it needs include/nyx/nyx.h" >&2
    exit 2
fi

build_directory="$engine_directory/generated/nyx-build"
library_directory="$engine_directory/generated/nyx"
generator_arguments=()
if command -v ninja > /dev/null; then
    generator_arguments=(-G Ninja)
fi

cmake -S "$nyx_source_directory" -B "$build_directory" "${generator_arguments[@]}" -DCMAKE_BUILD_TYPE=Release \
    -DNYX_BUILD_CAPI=ON -DNYX_BUILD_TESTS=OFF -DNYX_BUILD_TOOLS=OFF -DNYX_BUILD_EXAMPLES=OFF -DNYX_BUILD_LSP_LIBRARY=OFF
cmake --build "$build_directory" --target nyx-capi --parallel

mkdir -p "$library_directory"
for library_path in "$build_directory"/libnyx-capi.so "$build_directory"/libnyx-capi.dylib; do
    if [[ -f "$library_path" ]]; then
        ln -sf "$library_path" "$library_directory/$(basename "$library_path")"
    fi
done
echo "built nyx-capi into generated/nyx"
