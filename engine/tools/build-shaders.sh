#!/usr/bin/env bash
set -euo pipefail

engine_directory="$(cd "$(dirname "$0")/.." && pwd)"
shader_directory="$engine_directory/shaders"
compiled_directory="$shader_directory/compiled"
mkdir -p "$compiled_directory"

for shader_name in lit lines overlay; do
    for shader_stage in vert frag; do
        source_file="$shader_directory/$shader_name.$shader_stage"
        spirv_file="$compiled_directory/$shader_name.$shader_stage.spv"
        msl_file="$compiled_directory/$shader_name.$shader_stage.msl"
        glslangValidator -V "$source_file" -o "$spirv_file" > /dev/null
        spirv-cross "$spirv_file" --msl --msl-version 20100 --msl-decoration-binding --output "$msl_file"
    done
done
echo "wrote shaders/compiled/"
