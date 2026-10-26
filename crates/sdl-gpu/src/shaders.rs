//! Compiled shaders (from `build.rs`) and their conversion into SDL GPU shader objects.

use std::ffi::CString;

use sdl3_sys::everything as sdl;

use crate::GpuError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShaderStageKind {
    Vertex,
    Fragment,
}

/// One entry point, compiled for every backend we ship.
pub(crate) struct CompiledShader {
    pub name: &'static str,
    pub spirv: &'static [u8],
    pub msl: &'static str,
    pub entry_point: &'static str,
    pub msl_entry_point: &'static str,
    pub stage: ShaderStageKind,
    pub sampler_count: u32,
    pub uniform_buffer_count: u32,
}

include!(concat!(env!("OUT_DIR"), "/shader_manifest.rs"));

/// Which compiled form the device consumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShaderFormat {
    Spirv,
    Msl,
}

/// An SDL shader that is released on drop. Shaders are only needed until their pipelines exist.
pub(crate) struct ShaderObject {
    device: *mut sdl::SDL_GPUDevice,
    pub raw: *mut sdl::SDL_GPUShader,
}

impl Drop for ShaderObject {
    fn drop(&mut self) {
        // SAFETY: `raw` was created on `device`, which outlives this temporary (it is only used
        // while building pipelines inside `GpuRenderer::new`).
        unsafe { sdl::SDL_ReleaseGPUShader(self.device, self.raw) };
    }
}

pub(crate) fn create_shader(
    device: *mut sdl::SDL_GPUDevice,
    shader: &CompiledShader,
    format: ShaderFormat,
) -> Result<ShaderObject, GpuError> {
    // SPIR-V must be 4-byte aligned for Vulkan; `include_bytes!` only guarantees 1, so copy into
    // a `Vec<u32>`. MSL must be NUL-terminated text.
    let spirv_words: Vec<u32>;
    let msl_text: CString;
    let (code, code_size, entry_point, sdl_format) = match format {
        ShaderFormat::Spirv => {
            spirv_words = shader
                .spirv
                .chunks_exact(4)
                .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                .collect();
            (spirv_words.as_ptr().cast::<u8>(), shader.spirv.len(), shader.entry_point, sdl::SDL_GPU_SHADERFORMAT_SPIRV)
        }
        ShaderFormat::Msl => {
            msl_text =
                CString::new(shader.msl).map_err(|_| GpuError::new(format!("{}: MSL contains NUL", shader.name)))?;
            (msl_text.as_ptr().cast::<u8>(), shader.msl.len(), shader.msl_entry_point, sdl::SDL_GPU_SHADERFORMAT_MSL)
        }
    };
    let entry_point = CString::new(entry_point).expect("entry point names have no NUL");
    let info = sdl::SDL_GPUShaderCreateInfo {
        code_size,
        code,
        entrypoint: entry_point.as_ptr(),
        format: sdl_format,
        stage: match shader.stage {
            ShaderStageKind::Vertex => sdl::SDL_GPU_SHADERSTAGE_VERTEX,
            ShaderStageKind::Fragment => sdl::SDL_GPU_SHADERSTAGE_FRAGMENT,
        },
        num_samplers: shader.sampler_count,
        num_storage_textures: 0,
        num_storage_buffers: 0,
        num_uniform_buffers: shader.uniform_buffer_count,
        ..Default::default()
    };
    // SAFETY: `device` is live; `code` points to `code_size` readable bytes that outlive the call
    // (`spirv_words` / `msl_text` / the static data); `entry_point` is NUL-terminated.
    let raw = unsafe { sdl::SDL_CreateGPUShader(device, &info) };
    if raw.is_null() {
        return Err(GpuError::sdl(&format!("SDL_CreateGPUShader({})", shader.name)));
    }
    Ok(ShaderObject { device, raw })
}
