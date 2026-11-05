#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
repository_directory="$(cd "$engine_directory/.." && pwd)"
projects_directory="$(cd "$repository_directory/.." && pwd)"
nyx_source_directory="${NYX_SOURCE_DIR:-$projects_directory/nyx-scripting-language}"
chaos_source_directory="${CHAOS_SOURCE_DIR:-$projects_directory/chaos-ui-framework}"
cosmos_source_directory="${COSMOS_SOURCE_DIR:-$projects_directory/cosmos-styling-language}"
penumbra_bridge_source_directory="${PENUMBRA_BRIDGE_SOURCE_DIR:-$projects_directory/penumbra-bridge}"

for required_file in "$nyx_source_directory/include/nyx/nyx.h" "$chaos_source_directory/include/chaos/ChaosNyxDriver.h" "$cosmos_source_directory/include/cosmos/Resolver.h" "$penumbra_bridge_source_directory/capi/include/penumbra_bridge.h"; do
    if [[ ! -f "$required_file" ]]; then
        echo "build-native.sh needs nyx-scripting-language, chaos-ui-framework and cosmos-styling-language on proto and penumbra-bridge on main beside lantern" >&2
        echo "(or NYX_SOURCE_DIR, CHAOS_SOURCE_DIR, COSMOS_SOURCE_DIR and PENUMBRA_BRIDGE_SOURCE_DIR), with their submodules; missing $required_file" >&2
        exit 2
    fi
done

build_directory="$engine_directory/generated/native-build"
library_directory="$engine_directory/generated/nyx"
generator_arguments=()
if command -v ninja > /dev/null; then
    generator_arguments=(-G Ninja)
fi

cmake -S "$engine_directory/native" -B "$build_directory" "${generator_arguments[@]}" -DCMAKE_BUILD_TYPE=Release \
    -DLANTERN_NYX_SOURCE_DIR="$nyx_source_directory" -DLANTERN_CHAOS_SOURCE_DIR="$chaos_source_directory" \
    -DLANTERN_COSMOS_SOURCE_DIR="$cosmos_source_directory" -DLANTERN_PENUMBRA_BRIDGE_SOURCE_DIR="$penumbra_bridge_source_directory"
cmake --build "$build_directory" --target lantern-native --parallel

mkdir -p "$library_directory"
rm -f "$library_directory"/libnyx-capi.*
for library_path in "$build_directory"/liblantern-native.so "$build_directory"/liblantern-native.dylib; do
    if [[ -f "$library_path" ]]; then
        ln -sf "$library_path" "$library_directory/$(basename "$library_path")"
    fi
done
echo "built lantern-native (Nyx's C API, the Chaos UI bridge and penumbra-bridge's C API) into generated/nyx"
