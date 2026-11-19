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

rm -rf "$output_directory"
mkdir -p "$output_directory/bin" "$output_directory/shaders" "$output_directory/game"
cp "$built_program" "$output_directory/bin/lantern"
cp -R "$engine_directory/shaders/compiled" "$output_directory/shaders/compiled"
cp -R "$game_directory/." "$output_directory/game/"

packaged_game="$output_directory/game"
rm -rf "$packaged_game/assets/silhouettes" "$packaged_game/assets/editor"
find "$packaged_game" \( -name '*.calamus' -o -name '*.graph.json' -o -name '.editor-play*' \) -delete

cat > "$output_directory/play.sh" <<'SCRIPT'
#!/usr/bin/env bash
cd "$(dirname "$0")"
exec bin/lantern game "$@"
SCRIPT
chmod +x "$output_directory/play.sh"

if find "$output_directory" -name '*.calamus' | grep -q .; then
    echo "a Calamus file reached the package" >&2
    exit 1
fi
echo "packaged $game_directory into $output_directory"
