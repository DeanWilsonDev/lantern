#!/usr/bin/env bash
set -euo pipefail

lantern_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
workspace_directory="${LANTERN_WORKSPACE:-$(dirname "$lantern_directory")}"
install_prefix="${LANTERN_PREFIX:-$HOME/.local}"
mist_cache_directory="${MIST_CACHE_DIR:-$HOME/.mist/cache}"
output_directory="${1:-$PWD}"
archive_name="lantern-cloud-toolchain.tar.gz"

for required_path in "$install_prefix/bin/nox" "$install_prefix/bin/nox-lsp" "$install_prefix/bin/mist" \
    "$mist_cache_directory/builds" "$mist_cache_directory/sources"; do
    [[ -e "$required_path" ]] || { echo "cloud-toolchain-pack: $required_path is missing; run engine/tools/cloud-bootstrap.sh first" >&2; exit 1; }
done

staging_directory="$(mktemp -d)"
trap 'rm -rf "$staging_directory"' EXIT

nox_commit="$(git -C "$workspace_directory/nox-systems-language" rev-parse HEAD)"
mist_commit="$(git -C "$workspace_directory/mist-package-manager" rev-parse HEAD)"
mist_lock_hash="$(sha256sum "$lantern_directory/mist.lock" | cut -d' ' -f1)"
{
    echo "nox_commit=$nox_commit"
    echo "mist_commit=$mist_commit"
    echo "mist_lock_hash=$mist_lock_hash"
    echo "home_directory=$HOME"
    echo "lantern_directory=$lantern_directory"
    echo "install_prefix=$install_prefix"
    echo "mist_cache_directory=$mist_cache_directory"
    echo "built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$staging_directory/toolchain-manifest.txt"

relative_to_root() { echo "${1#/}"; }

mkdir -p "$output_directory"
tar -C / -czf "$output_directory/$archive_name" \
    --transform "s,^$(relative_to_root "$staging_directory")/,," \
    "$(relative_to_root "$staging_directory")/toolchain-manifest.txt" \
    "$(relative_to_root "$install_prefix")/bin/nox" \
    "$(relative_to_root "$install_prefix")/bin/nox-lsp" \
    "$(relative_to_root "$install_prefix")/bin/mist" \
    "$(relative_to_root "$mist_cache_directory")/builds" \
    "$(relative_to_root "$mist_cache_directory")/sources"
(cd "$output_directory" && sha256sum "$archive_name" > "$archive_name.sha256")

echo "cloud-toolchain-pack: wrote $output_directory/$archive_name ($(du -h "$output_directory/$archive_name" | cut -f1)) and $archive_name.sha256"
cat "$staging_directory/toolchain-manifest.txt"
