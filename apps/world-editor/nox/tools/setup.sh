#!/usr/bin/env bash
set -euo pipefail

host_directory="$(cd "$(dirname "$0")/.." && pwd)"
repository_directory="$(cd "$host_directory/../../.." && pwd)"
projects_directory="$(cd "$repository_directory/.." && pwd)"
penumbra_nox_directory="${PENUMBRA_NOX_DIR:-$projects_directory/penumbra-ui-engine}"
penumbra_bridge_directory="${PENUMBRA_BRIDGE_SOURCE_DIR:-$projects_directory/penumbra-bridge}"

if [[ ! -f "$penumbra_nox_directory/src/Widgets/Box.nox" || ! -f "$penumbra_nox_directory/sdl/SdlWindow.nox" ]]; then
    echo "setup.sh needs a penumbra-ui-engine checkout on nox-proto beside lantern, or PENUMBRA_NOX_DIR; missing $penumbra_nox_directory/sdl/SdlWindow.nox" >&2
    exit 2
fi
if [[ ! -f "$penumbra_bridge_directory/nox/src/Host.nox" ]]; then
    echo "setup.sh needs a penumbra-bridge checkout on main beside lantern, or PENUMBRA_BRIDGE_SOURCE_DIR; missing $penumbra_bridge_directory/nox/src/Host.nox" >&2
    exit 2
fi

escaped_for_nox() {
    local text="$1"
    text="${text//\\/\\\\}"
    text="${text//\"/\\\"}"
    printf '%s' "$text"
}

mkdir -p "$host_directory/external" "$host_directory/generated"
ln -sfn "$(cd "$penumbra_nox_directory" && pwd)" "$host_directory/external/penumbra-nox"
rm -rf "$host_directory/generated/penumbra-bridge"
mkdir -p "$host_directory/generated/penumbra-bridge"
for bridge_source in "$penumbra_bridge_directory/nox/generated/penumbra-bridge.nox" "$penumbra_bridge_directory"/nox/src/*.nox; do
    sed 's/foreign "penumbra-bridge-capi"/foreign "lantern-native"/' "$bridge_source" \
        > "$host_directory/generated/penumbra-bridge/$(basename "$bridge_source")"
done

world_editor_directory="$(cd "$host_directory/.." && pwd)"
cat > "$host_directory/generated/HostPaths.nox" <<NOX
lantern:::worldeditor:::paths :: namespace {
    UI_DIRECTORY: string : "$(escaped_for_nox "$world_editor_directory/src/ui")";
    FONT_PATH: string : "$(escaped_for_nox "$world_editor_directory/data/fonts/Inter-Regular.ttf")";
    ASSETS_DIRECTORY: string : "$(escaped_for_nox "$repository_directory/assets")";
    SHADER_DIRECTORY: string : "$(escaped_for_nox "$repository_directory/engine/shaders/compiled")";
}
NOX

echo "external/penumbra-nox -> $(readlink "$host_directory/external/penumbra-nox")"
echo "copied penumbra-bridge's Nox side into generated/penumbra-bridge, bound to lantern-native, and wrote generated/HostPaths.nox"
