#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
shader_directory="$engine_directory/shaders"
compiled_directory="$shader_directory/compiled"
mkdir -p "$compiled_directory"

shader_files=(
    lit.vert lit.frag
    lines.vert lines.frag
    overlay.vert overlay.frag
    shadow.vert shadow.frag
    fullscreen.vert sky.vert sky.frag
    bloom-downsample.frag bloom-upsample.frag composite.frag
)
for shader_file in "${shader_files[@]}"; do
    source_file="$shader_directory/$shader_file"
    spirv_file="$compiled_directory/$shader_file.spv"
    msl_file="$compiled_directory/$shader_file.msl"
    glslangValidator -I"$shader_directory" -V "$source_file" -o "$spirv_file" > /dev/null
    spirv-cross "$spirv_file" --msl --msl-version 20100 --msl-decoration-binding --output "$msl_file"
done
echo "wrote shaders/compiled/"
