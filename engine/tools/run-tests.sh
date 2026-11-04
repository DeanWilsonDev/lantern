#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
nox_command="${NOX:-nox}"

cd "$engine_directory"
test_suites=()
for suite_directory in tests/*/; do
    test_suites+=("${suite_directory%/}")
done
"$nox_command" test "$@" "${test_suites[@]}"
