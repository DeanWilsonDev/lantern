#!/usr/bin/env bash
set -euo pipefail

lantern_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
workspace_directory="${LANTERN_WORKSPACE:-$(dirname "$lantern_directory")}"
install_prefix="${LANTERN_PREFIX:-$HOME/.local}"
parallel_jobs="${LANTERN_JOBS:-$(getconf _NPROCESSORS_ONLN)}"
export CC="${CC:-gcc-14}" CXX="${CXX:-g++-14}"
export GIT_TERMINAL_PROMPT=0
export PATH="$install_prefix/bin:$PATH"

git config --global url."https://github.com/fearlessgamelabs/".insteadOf "https://github.com/fearlessgamedev/"

for required_tool in "$CC" "$CXX" cmake ninja git; do
    command -v "$required_tool" >/dev/null || { echo "cloud-bootstrap: $required_tool is missing" >&2; exit 1; }
done

checkout_directory_for() {
    local repository_name="$1" branch_name="$2"
    local checkout_directory="$workspace_directory/$repository_name"
    if [[ ! -d "$checkout_directory/.git" ]]; then
        git clone --depth 1 -b "$branch_name" "https://github.com/fearlessgamelabs/$repository_name" "$checkout_directory" >&2
    fi
    git -C "$checkout_directory" submodule update --init --recursive >&2
    echo "$checkout_directory"
}

toolchain_repository="${LANTERN_TOOLCHAIN_REPOSITORY:-fearlessgamelabs/fearless-hq}"
toolchain_tag="${LANTERN_TOOLCHAIN_TAG:-lantern-cloud-toolchain}"
toolchain_archive_name="lantern-cloud-toolchain.tar.gz"

download_release_asset() {
    local asset_name="$1" destination_path="$2" asset_identifier
    asset_identifier="$(curl -fsS "https://api.github.com/repos/$toolchain_repository/releases/tags/$toolchain_tag" \
        | python3 -I -c 'import json,sys; print(next(a["id"] for a in json.load(sys.stdin)["assets"] if a["name"]==sys.argv[1]))' "$asset_name")" || return 1
    curl -fsSL -H "Accept: application/octet-stream" -o "$destination_path" \
        "https://api.github.com/repos/$toolchain_repository/releases/assets/$asset_identifier"
}

install_prebuilt_toolchain() {
    [[ "${LANTERN_BUILD_FROM_SOURCE:-0}" == 1 ]] && return 1
    local download_directory archive_path manifest_text
    download_directory="$(mktemp -d)"
    archive_path="${LANTERN_TOOLCHAIN_FILE:-$download_directory/$toolchain_archive_name}"
    if [[ -z "${LANTERN_TOOLCHAIN_FILE:-}" ]]; then
        download_release_asset "$toolchain_archive_name" "$archive_path" \
            && download_release_asset "$toolchain_archive_name.sha256" "$archive_path.sha256" \
            || { echo "cloud-bootstrap: no prebuilt toolchain at $toolchain_repository@$toolchain_tag" >&2; rm -rf "$download_directory"; return 1; }
        (cd "$download_directory" && sha256sum --check --quiet "$toolchain_archive_name.sha256") \
            || { echo "cloud-bootstrap: prebuilt toolchain checksum mismatch" >&2; rm -rf "$download_directory"; return 1; }
    fi
    manifest_text="$(tar -xzOf "$archive_path" toolchain-manifest.txt)"
    for expected_line in "home_directory=$HOME" "lantern_directory=$lantern_directory" "install_prefix=$install_prefix"; do
        grep -qxF "$expected_line" <<<"$manifest_text" \
            || { echo "cloud-bootstrap: prebuilt toolchain was made for a different layout ($expected_line); building from source" >&2; rm -rf "$download_directory"; return 1; }
    done
    tar -C / -xzf "$archive_path" --exclude=toolchain-manifest.txt
    rm -rf "$download_directory"
    warn_when_prebuilt_toolchain_is_stale "$manifest_text"
}

warn_when_prebuilt_toolchain_is_stale() {
    local manifest_text="$1" repository_name branch_name key live_commit built_commit
    for repository_name in nox-systems-language:proto:nox_commit mist-package-manager:main:mist_commit; do
        IFS=: read -r repository_name branch_name key <<<"$repository_name"
        built_commit="$(sed -n "s/^$key=//p" <<<"$manifest_text")"
        live_commit="$(git ls-remote "https://github.com/fearlessgamelabs/$repository_name" "refs/heads/$branch_name" 2>/dev/null | cut -f1)"
        [[ -z "$live_commit" || "$live_commit" == "$built_commit" ]] \
            || echo "cloud-bootstrap: warning: prebuilt $repository_name is $built_commit but $branch_name is $live_commit; rerun with LANTERN_BUILD_FROM_SOURCE=1 if a test needs the newer compiler" >&2
    done
}

build_toolchain_from_source() {
    nox_directory="$(checkout_directory_for nox-systems-language proto)"
    cmake -S "$nox_directory" -B "$nox_directory/build" -G Ninja -DCMAKE_BUILD_TYPE=Release -DNOX_UNITY_BUILD=ON
    cmake --build "$nox_directory/build" --target nox nox-lsp --parallel "$parallel_jobs"
    cmake --install "$nox_directory/build" --component nox --prefix "$install_prefix"

    mist_directory="$(checkout_directory_for mist-package-manager main)"
    cmake -S "$mist_directory" -B "$mist_directory/build" -G Ninja -DCMAKE_BUILD_TYPE=Release
    cmake --build "$mist_directory/build" --target mist --parallel "$parallel_jobs"
    cmake --install "$mist_directory/build" --component mist --prefix "$install_prefix"

}

if ! install_prebuilt_toolchain; then
    build_toolchain_from_source
fi

cd "$lantern_directory"
mist install

echo "cloud-bootstrap: nox and mist are in $install_prefix/bin; put it on PATH, then run 'mist run test' from $lantern_directory"
