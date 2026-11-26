#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
repository_root="$(cd "$engine_directory/.." && pwd)"
game_directory="${1:-$repository_root/games/platformer}"
output_directory="${2:-$engine_directory/build/package}"

[[ -f "$game_directory/game.json" ]] || { echo "no game.json in $game_directory" >&2; exit 1; }
game_directory="$(cd "$game_directory" && pwd)"

(cd "$engine_directory" && nox build --release --project . >/dev/null)
built_program="$engine_directory/build/lantern"
[[ -x "$built_program" ]] || { echo "the build left no program at $built_program" >&2; exit 1; }

(cd "$engine_directory" && nox run --release --project tools/build-graphs -- "$game_directory" >/dev/null && nox run --release --project tools/build-materials -- "$game_directory" >/dev/null)

staging_directory="$(mktemp -d)"
trap 'rm -rf "$staging_directory"' EXIT
cp -R "$game_directory/." "$staging_directory/"
rm -rf "$staging_directory/assets/silhouettes" "$staging_directory/assets/editor" "$staging_directory/build" "$staging_directory/mist_packages"
rm -f "$staging_directory/lantern.project.json" "$staging_directory/mist.json" "$staging_directory/mist.local.json" "$staging_directory/README.md"
find "$staging_directory" \( -name '*.calamus' -o -name '*.graph.json' -o -name '.editor-play*' -o -path '*/game-assets/materials/*.frag' \) -delete
if find "$staging_directory" -name '*.calamus' | grep -q .; then
    echo "a Calamus file reached the package" >&2
    exit 1
fi

(cd "$engine_directory" && nox build --release --project tools/pack-game >/dev/null)
packer="$engine_directory/tools/pack-game/build/pack-game"
[[ -x "$packer" ]] || packer="$engine_directory/build/pack-game"
[[ -x "$packer" ]] || { echo "the build left no pack-game program" >&2; exit 1; }

rm -rf "$output_directory"
mkdir -p "$output_directory"
"$packer" "$staging_directory" "$engine_directory/shaders/compiled" "$built_program" "$output_directory/lantern"
chmod +x "$output_directory/lantern"
echo "packaged $game_directory into $output_directory/lantern"
