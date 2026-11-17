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

release_asset_identifier() {
    local repository_name="$1" release_tag="$2" asset_name="$3"
    curl -fsS "https://api.github.com/repos/fearlessgamelabs/$repository_name/releases/tags/$release_tag" \
        | python3 -I -c 'import json,sys; print(next(a["id"] for a in json.load(sys.stdin)["assets"] if a["name"]==sys.argv[1]))' "$asset_name"
}

download_release_asset() {
    local repository_name="$1" release_tag="$2" asset_name="$3" destination_path="$4" asset_identifier
    asset_identifier="$(release_asset_identifier "$repository_name" "$release_tag" "$asset_name")" || return 1
    curl -fsSL -H "Accept: application/octet-stream" -o "$destination_path" \
        "https://api.github.com/repos/fearlessgamelabs/$repository_name/releases/assets/$asset_identifier"
}

install_release_tool() {
    local repository_name="$1" release_tag="$2" archive_name="$3" download_directory
    download_directory="$(mktemp -d)"
    if download_release_asset "$repository_name" "$release_tag" "$archive_name" "$download_directory/$archive_name" \
        && download_release_asset "$repository_name" "$release_tag" "$archive_name.sha256" "$download_directory/$archive_name.sha256" \
        && (cd "$download_directory" && sha256sum --check --quiet "$archive_name.sha256") \
        && mkdir -p "$install_prefix" && tar -C "$install_prefix" -xzf "$download_directory/$archive_name"; then
        rm -rf "$download_directory"
        return 0
    fi
    echo "cloud-bootstrap: could not install $archive_name from $repository_name@$release_tag" >&2
    rm -rf "$download_directory"
    return 1
}

install_tools_from_releases() {
    [[ "${LANTERN_BUILD_FROM_SOURCE:-0}" == 1 ]] && return 1
    [[ "$(uname -s)-$(uname -m)" == Linux-x86_64 ]] || return 1
    install_release_tool nox-systems-language "${LANTERN_NOX_RELEASE_TAG:-proto-latest}" nox-linux-x86_64.tar.gz \
        && install_release_tool mist-package-manager "${LANTERN_MIST_RELEASE_TAG:-main-latest}" mist-linux-x86_64.tar.gz \
        && command -v nox-lsp >/dev/null && command -v mist >/dev/null
}

seed_mist_cache_from_bundle() {
    local bundle_repository_name="${LANTERN_BUNDLE_REPOSITORY:-fearless-hq}" bundle_tag="${LANTERN_BUNDLE_TAG:-lantern-cloud-toolchain}"
    local mist_cache_directory="${MIST_CACHE_DIR:-$HOME/.mist/cache}" archive_name=lantern-cloud-toolchain.tar.gz
    local download_directory bundled_cache_directory cache_pattern strip_count
    [[ -d "$mist_cache_directory/sources" ]] && return 0
    download_directory="$(mktemp -d)"
    if download_release_asset "$bundle_repository_name" "$bundle_tag" "$archive_name" "$download_directory/$archive_name" \
        && download_release_asset "$bundle_repository_name" "$bundle_tag" "$archive_name.sha256" "$download_directory/$archive_name.sha256" \
        && (cd "$download_directory" && sha256sum --check --quiet "$archive_name.sha256"); then
        bundled_cache_directory="$(tar -xzOf "$download_directory/$archive_name" toolchain-manifest.txt | sed -n 's/^mist_cache_directory=//p')"
        cache_pattern='*/.mist/cache/sources/*'
        [[ "$bundled_cache_directory" == "$mist_cache_directory" ]] && cache_pattern='*/.mist/cache/*'
        strip_count="$(awk -F/ '{print NF-1}' <<<"$bundled_cache_directory")"
        mkdir -p "$mist_cache_directory"
        tar -xzf "$download_directory/$archive_name" -C "$mist_cache_directory" --strip-components="$strip_count" --wildcards "$cache_pattern" 2>/dev/null \
            || echo "cloud-bootstrap: could not unpack the cached Mist builds; mist install will rebuild them" >&2
    else
        echo "cloud-bootstrap: no cached Mist builds; mist install will fetch and build them" >&2
    fi
    rm -rf "$download_directory"
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

if ! install_tools_from_releases; then
    echo "cloud-bootstrap: building nox and mist from source" >&2
    build_toolchain_from_source
fi

seed_mist_cache_from_bundle

cd "$lantern_directory"
mist install

echo "cloud-bootstrap: nox and mist are in $install_prefix/bin; put it on PATH, then run 'mist run test' from $lantern_directory"
