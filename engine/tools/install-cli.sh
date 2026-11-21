#!/usr/bin/env bash
set -euo pipefail

tools_directory="$(cd "$(dirname "$0")" && pwd)"
destination="${1:-$HOME/.local/bin}"
mkdir -p "$destination"
ln -sf "$tools_directory/lantern" "$destination/lantern"
echo "linked $destination/lantern -> $tools_directory/lantern"
case ":$PATH:" in
*":$destination:"*) ;;
*) echo "add $destination to your PATH to run 'lantern' from anywhere" ;;
esac
