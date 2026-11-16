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

nox_directory="$(checkout_directory_for nox-systems-language proto)"
cmake -S "$nox_directory" -B "$nox_directory/build" -G Ninja -DCMAKE_BUILD_TYPE=Release -DNOX_UNITY_BUILD=ON
cmake --build "$nox_directory/build" --target nox nox-lsp --parallel "$parallel_jobs"
cmake --install "$nox_directory/build" --component nox --prefix "$install_prefix"

mist_directory="$(checkout_directory_for mist-package-manager main)"
cmake -S "$mist_directory" -B "$mist_directory/build" -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build "$mist_directory/build" --target mist --parallel "$parallel_jobs"
cmake --install "$mist_directory/build" --component mist --prefix "$install_prefix"

cd "$lantern_directory"
mist install

echo "cloud-bootstrap: nox and mist are in $install_prefix/bin; put it on PATH, then run 'mist run test' from $lantern_directory"
