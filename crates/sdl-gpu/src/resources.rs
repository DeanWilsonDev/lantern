//! GPU resources: buffers, textures, samplers and the staging (upload) path.
//!
//! Static data (meshes, textures) is uploaded once through a temporary transfer buffer in its
//! own command buffer. Per-frame data (debug lines, overlay) goes through `DynamicBuffer`, which
//! re-uses one GPU buffer and one transfer buffer and asks SDL to *cycle* them so writing this
//! frame's data never stalls on the GPU still reading last frame's.

use lantern_core::math::Aabb;
use lantern_renderer_3d::texture::{TextureFilter, TextureWrap};
use lantern_renderer_3d::{MeshData, TextureData};
use sdl3_sys::everything as sdl;

use crate::GpuError;

pub(crate) struct GpuMesh {
    pub vertex_buffer: *mut sdl::SDL_GPUBuffer,
    pub index_buffer: *mut sdl::SDL_GPUBuffer,
    pub index_count: u32,
    pub bounds: Aabb,
}

impl GpuMesh {
    pub fn release(self, device: *mut sdl::SDL_GPUDevice) {
        // SAFETY: both buffers were created on `device` and are released once (we consume self).
        unsafe {
            sdl::SDL_ReleaseGPUBuffer(device, self.vertex_buffer);
            sdl::SDL_ReleaseGPUBuffer(device, self.index_buffer);
        }
    }
}

pub(crate) struct GpuTexture {
    pub texture: *mut sdl::SDL_GPUTexture,
    /// Borrowed from `Samplers`; not released with the texture.
    pub sampler: *mut sdl::SDL_GPUSampler,
}

impl GpuTexture {
    pub fn release(&self, device: *mut sdl::SDL_GPUDevice) {
        // SAFETY: created on `device`; callers release each texture once.
        unsafe { sdl::SDL_ReleaseGPUTexture(device, self.texture) };
    }
}

/// The four sampler combinations the renderer needs, created once.
pub(crate) struct Samplers {
    linear_repeat: *mut sdl::SDL_GPUSampler,
    linear_clamp: *mut sdl::SDL_GPUSampler,
    nearest_repeat: *mut sdl::SDL_GPUSampler,
    nearest_clamp: *mut sdl::SDL_GPUSampler,
}

impl Samplers {
    pub fn create(device: *mut sdl::SDL_GPUDevice) -> Result<Samplers, GpuError> {
        let make = |filter: TextureFilter, wrap: TextureWrap| -> Result<*mut sdl::SDL_GPUSampler, GpuError> {
            let (sdl_filter, mipmap_mode) = match filter {
                TextureFilter::Linear => (sdl::SDL_GPU_FILTER_LINEAR, sdl::SDL_GPU_SAMPLERMIPMAPMODE_LINEAR),
                TextureFilter::Nearest => (sdl::SDL_GPU_FILTER_NEAREST, sdl::SDL_GPU_SAMPLERMIPMAPMODE_NEAREST),
            };
            let address_mode = match wrap {
                TextureWrap::Repeat => sdl::SDL_GPU_SAMPLERADDRESSMODE_REPEAT,
                TextureWrap::Clamp => sdl::SDL_GPU_SAMPLERADDRESSMODE_CLAMP_TO_EDGE,
            };
            let info = sdl::SDL_GPUSamplerCreateInfo {
                min_filter: sdl_filter,
                mag_filter: sdl_filter,
                mipmap_mode,
                address_mode_u: address_mode,
                address_mode_v: address_mode,
                address_mode_w: address_mode,
                max_lod: 1000.0,
                max_anisotropy: 4.0,
                enable_anisotropy: filter == TextureFilter::Linear,
                ..Default::default()
            };
            // SAFETY: live device; `info` is a local.
            let sampler = unsafe { sdl::SDL_CreateGPUSampler(device, &info) };
            if sampler.is_null() { Err(GpuError::sdl("SDL_CreateGPUSampler")) } else { Ok(sampler) }
        };
        Ok(Samplers {
            linear_repeat: make(TextureFilter::Linear, TextureWrap::Repeat)?,
            linear_clamp: make(TextureFilter::Linear, TextureWrap::Clamp)?,
            nearest_repeat: make(TextureFilter::Nearest, TextureWrap::Repeat)?,
            nearest_clamp: make(TextureFilter::Nearest, TextureWrap::Clamp)?,
        })
    }

    pub fn for_texture(&self, texture: &TextureData) -> *mut sdl::SDL_GPUSampler {
        match (texture.filter, texture.wrap) {
            (TextureFilter::Linear, TextureWrap::Repeat) => self.linear_repeat,
            (TextureFilter::Linear, TextureWrap::Clamp) => self.linear_clamp,
            (TextureFilter::Nearest, TextureWrap::Repeat) => self.nearest_repeat,
            (TextureFilter::Nearest, TextureWrap::Clamp) => self.nearest_clamp,
        }
    }

    pub fn release(&self, device: *mut sdl::SDL_GPUDevice) {
        for sampler in [self.linear_repeat, self.linear_clamp, self.nearest_repeat, self.nearest_clamp] {
            // SAFETY: created on `device`, released once from `GpuRenderer::drop`.
            unsafe { sdl::SDL_ReleaseGPUSampler(device, sampler) };
        }
    }
}

pub(crate) fn create_buffer(
    device: *mut sdl::SDL_GPUDevice,
    usage: sdl::SDL_GPUBufferUsageFlags,
    size: u32,
) -> Result<*mut sdl::SDL_GPUBuffer, GpuError> {
    let info = sdl::SDL_GPUBufferCreateInfo { usage, size, ..Default::default() };
    // SAFETY: live device; `info` is a local.
    let buffer = unsafe { sdl::SDL_CreateGPUBuffer(device, &info) };
    if buffer.is_null() { Err(GpuError::sdl("SDL_CreateGPUBuffer")) } else { Ok(buffer) }
}

pub(crate) fn create_transfer_buffer(
    device: *mut sdl::SDL_GPUDevice,
    size: u32,
    upload: bool,
) -> Result<*mut sdl::SDL_GPUTransferBuffer, GpuError> {
    let usage =
        if upload { sdl::SDL_GPU_TRANSFERBUFFERUSAGE_UPLOAD } else { sdl::SDL_GPU_TRANSFERBUFFERUSAGE_DOWNLOAD };
    let info = sdl::SDL_GPUTransferBufferCreateInfo { usage, size, ..Default::default() };
    // SAFETY: live device; `info` is a local.
    let buffer = unsafe { sdl::SDL_CreateGPUTransferBuffer(device, &info) };
    if buffer.is_null() { Err(GpuError::sdl("SDL_CreateGPUTransferBuffer")) } else { Ok(buffer) }
}

/// Copies `chunks` back to back into a new upload transfer buffer.
fn staging_buffer(
    device: *mut sdl::SDL_GPUDevice,
    chunks: &[&[u8]],
) -> Result<*mut sdl::SDL_GPUTransferBuffer, GpuError> {
    let total: usize = chunks.iter().map(|chunk| chunk.len()).sum();
    let transfer = create_transfer_buffer(device, total as u32, true)?;
    // SAFETY: `transfer` is a fresh upload buffer of `total` bytes; we write exactly `total`
    // bytes into the mapping and unmap before anything reads it.
    unsafe {
        let mapped = sdl::SDL_MapGPUTransferBuffer(device, transfer, false).cast::<u8>();
        if mapped.is_null() {
            sdl::SDL_ReleaseGPUTransferBuffer(device, transfer);
            return Err(GpuError::sdl("SDL_MapGPUTransferBuffer"));
        }
        let mut offset = 0;
        for chunk in chunks {
            std::ptr::copy_nonoverlapping(chunk.as_ptr(), mapped.add(offset), chunk.len());
            offset += chunk.len();
        }
        sdl::SDL_UnmapGPUTransferBuffer(device, transfer);
    }
    Ok(transfer)
}

pub(crate) fn upload_mesh(device: *mut sdl::SDL_GPUDevice, mesh: &MeshData) -> Result<GpuMesh, GpuError> {
    let vertex_bytes = mesh.vertex_bytes();
    let index_bytes = mesh.index_bytes();
    let vertex_buffer = create_buffer(device, sdl::SDL_GPU_BUFFERUSAGE_VERTEX, vertex_bytes.len() as u32)?;
    let index_buffer = match create_buffer(device, sdl::SDL_GPU_BUFFERUSAGE_INDEX, index_bytes.len() as u32) {
        Ok(buffer) => buffer,
        Err(error) => {
            // SAFETY: created just above on this device.
            unsafe { sdl::SDL_ReleaseGPUBuffer(device, vertex_buffer) };
            return Err(error);
        }
    };
    let gpu_mesh =
        GpuMesh { vertex_buffer, index_buffer, index_count: mesh.indices.len() as u32, bounds: mesh.bounds() };
    let transfer = match staging_buffer(device, &[&vertex_bytes, &index_bytes]) {
        Ok(transfer) => transfer,
        Err(error) => {
            gpu_mesh.release(device);
            return Err(error);
        }
    };
    // SAFETY: live device; the copy pass is begun and ended on a fresh command buffer that is
    // submitted once; regions lie within the buffers created above. Releasing the transfer buffer
    // straight after submission is allowed: SDL keeps it until the copy has executed.
    unsafe {
        let command_buffer = sdl::SDL_AcquireGPUCommandBuffer(device);
        if command_buffer.is_null() {
            sdl::SDL_ReleaseGPUTransferBuffer(device, transfer);
            gpu_mesh.release(device);
            return Err(GpuError::sdl("SDL_AcquireGPUCommandBuffer"));
        }
        let copy_pass = sdl::SDL_BeginGPUCopyPass(command_buffer);
        sdl::SDL_UploadToGPUBuffer(
            copy_pass,
            &sdl::SDL_GPUTransferBufferLocation { transfer_buffer: transfer, offset: 0 },
            &sdl::SDL_GPUBufferRegion { buffer: vertex_buffer, offset: 0, size: vertex_bytes.len() as u32 },
            false,
        );
        sdl::SDL_UploadToGPUBuffer(
            copy_pass,
            &sdl::SDL_GPUTransferBufferLocation { transfer_buffer: transfer, offset: vertex_bytes.len() as u32 },
            &sdl::SDL_GPUBufferRegion { buffer: index_buffer, offset: 0, size: index_bytes.len() as u32 },
            false,
        );
        sdl::SDL_EndGPUCopyPass(copy_pass);
        let submitted = sdl::SDL_SubmitGPUCommandBuffer(command_buffer);
        sdl::SDL_ReleaseGPUTransferBuffer(device, transfer);
        if !submitted {
            gpu_mesh.release(device);
            return Err(GpuError::sdl("SDL_SubmitGPUCommandBuffer"));
        }
    }
    Ok(gpu_mesh)
}

fn mip_level_count(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

pub(crate) fn upload_texture(
    device: *mut sdl::SDL_GPUDevice,
    samplers: &Samplers,
    texture: &TextureData,
) -> Result<GpuTexture, GpuError> {
    if texture.width == 0 || texture.height == 0 || texture.rgba8.len() != (texture.width * texture.height * 4) as usize
    {
        return Err(GpuError::new(format!(
            "texture data is {} bytes for {}x{}",
            texture.rgba8.len(),
            texture.width,
            texture.height
        )));
    }
    let with_mipmaps = texture.filter == TextureFilter::Linear && texture.width.max(texture.height) > 1;
    let levels = if with_mipmaps { mip_level_count(texture.width, texture.height) } else { 1 };
    let mut usage = sdl::SDL_GPU_TEXTUREUSAGE_SAMPLER;
    if with_mipmaps {
        // Mipmap generation renders (blits) into each level.
        usage |= sdl::SDL_GPU_TEXTUREUSAGE_COLOR_TARGET;
    }
    let info = sdl::SDL_GPUTextureCreateInfo {
        r#type: sdl::SDL_GPU_TEXTURETYPE_2D,
        format: sdl::SDL_GPU_TEXTUREFORMAT_R8G8B8A8_UNORM_SRGB,
        usage,
        width: texture.width,
        height: texture.height,
        layer_count_or_depth: 1,
        num_levels: levels,
        sample_count: sdl::SDL_GPU_SAMPLECOUNT_1,
        ..Default::default()
    };
    // SAFETY: live device; `info` is a local.
    let gpu_texture = unsafe { sdl::SDL_CreateGPUTexture(device, &info) };
    if gpu_texture.is_null() {
        return Err(GpuError::sdl("SDL_CreateGPUTexture"));
    }
    let result = GpuTexture { texture: gpu_texture, sampler: samplers.for_texture(texture) };
    let transfer = match staging_buffer(device, &[&texture.rgba8]) {
        Ok(transfer) => transfer,
        Err(error) => {
            result.release(device);
            return Err(error);
        }
    };
    // SAFETY: as in `upload_mesh`; the region covers mip level 0 exactly.
    unsafe {
        let command_buffer = sdl::SDL_AcquireGPUCommandBuffer(device);
        if command_buffer.is_null() {
            sdl::SDL_ReleaseGPUTransferBuffer(device, transfer);
            result.release(device);
            return Err(GpuError::sdl("SDL_AcquireGPUCommandBuffer"));
        }
        let copy_pass = sdl::SDL_BeginGPUCopyPass(command_buffer);
        sdl::SDL_UploadToGPUTexture(
            copy_pass,
            &sdl::SDL_GPUTextureTransferInfo { transfer_buffer: transfer, ..Default::default() },
            &sdl::SDL_GPUTextureRegion {
                texture: gpu_texture,
                w: texture.width,
                h: texture.height,
                d: 1,
                ..Default::default()
            },
            false,
        );
        sdl::SDL_EndGPUCopyPass(copy_pass);
        if with_mipmaps {
            sdl::SDL_GenerateMipmapsForGPUTexture(command_buffer, gpu_texture);
        }
        let submitted = sdl::SDL_SubmitGPUCommandBuffer(command_buffer);
        sdl::SDL_ReleaseGPUTransferBuffer(device, transfer);
        if !submitted {
            result.release(device);
            return Err(GpuError::sdl("SDL_SubmitGPUCommandBuffer"));
        }
    }
    Ok(result)
}

/// A colour or depth render target (used for the depth buffer and for captures).
pub(crate) fn create_render_texture(
    device: *mut sdl::SDL_GPUDevice,
    format: sdl::SDL_GPUTextureFormat,
    width: u32,
    height: u32,
    depth: bool,
) -> Result<*mut sdl::SDL_GPUTexture, GpuError> {
    let usage =
        if depth { sdl::SDL_GPU_TEXTUREUSAGE_DEPTH_STENCIL_TARGET } else { sdl::SDL_GPU_TEXTUREUSAGE_COLOR_TARGET };
    let info = sdl::SDL_GPUTextureCreateInfo {
        r#type: sdl::SDL_GPU_TEXTURETYPE_2D,
        format,
        usage,
        width: width.max(1),
        height: height.max(1),
        layer_count_or_depth: 1,
        num_levels: 1,
        sample_count: sdl::SDL_GPU_SAMPLECOUNT_1,
        ..Default::default()
    };
    // SAFETY: live device; `info` is a local.
    let texture = unsafe { sdl::SDL_CreateGPUTexture(device, &info) };
    if texture.is_null() { Err(GpuError::sdl("SDL_CreateGPUTexture(render target)")) } else { Ok(texture) }
}

/// A GPU buffer rewritten every frame.
pub(crate) struct DynamicBuffer {
    usage: sdl::SDL_GPUBufferUsageFlags,
    pub buffer: *mut sdl::SDL_GPUBuffer,
    transfer: *mut sdl::SDL_GPUTransferBuffer,
    capacity: u32,
}

impl DynamicBuffer {
    pub fn new(usage: sdl::SDL_GPUBufferUsageFlags) -> DynamicBuffer {
        DynamicBuffer { usage, buffer: std::ptr::null_mut(), transfer: std::ptr::null_mut(), capacity: 0 }
    }

    /// Grows (to the next power of two) when `size` does not fit.
    pub fn reserve(&mut self, device: *mut sdl::SDL_GPUDevice, size: u32) -> Result<(), GpuError> {
        if size <= self.capacity {
            return Ok(());
        }
        self.release(device);
        let capacity = size.next_power_of_two().max(4096);
        self.buffer = create_buffer(device, self.usage, capacity)?;
        self.transfer = create_transfer_buffer(device, capacity, true)?;
        self.capacity = capacity;
        Ok(())
    }

    /// Records an upload of `bytes` into `copy_pass`. `reserve` must have been called.
    pub fn upload(&mut self, device: *mut sdl::SDL_GPUDevice, copy_pass: *mut sdl::SDL_GPUCopyPass, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        debug_assert!(bytes.len() as u32 <= self.capacity);
        // SAFETY: the transfer buffer has `capacity >= bytes.len()` bytes. `cycle = true` on both
        // map and upload makes SDL hand us fresh memory if the GPU still uses last frame's copy.
        unsafe {
            let mapped = sdl::SDL_MapGPUTransferBuffer(device, self.transfer, true).cast::<u8>();
            if mapped.is_null() {
                log::error!(target: "renderer", "{}", GpuError::sdl("SDL_MapGPUTransferBuffer"));
                return;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), mapped, bytes.len());
            sdl::SDL_UnmapGPUTransferBuffer(device, self.transfer);
            sdl::SDL_UploadToGPUBuffer(
                copy_pass,
                &sdl::SDL_GPUTransferBufferLocation { transfer_buffer: self.transfer, offset: 0 },
                &sdl::SDL_GPUBufferRegion { buffer: self.buffer, offset: 0, size: bytes.len() as u32 },
                true,
            );
        }
    }

    pub fn release(&mut self, device: *mut sdl::SDL_GPUDevice) {
        // SAFETY: each pointer, when non-null, was created on `device` and is nulled after release.
        unsafe {
            if !self.buffer.is_null() {
                sdl::SDL_ReleaseGPUBuffer(device, self.buffer);
            }
            if !self.transfer.is_null() {
                sdl::SDL_ReleaseGPUTransferBuffer(device, self.transfer);
            }
        }
        self.buffer = std::ptr::null_mut();
        self.transfer = std::ptr::null_mut();
        self.capacity = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_chain_lengths() {
        assert_eq!(mip_level_count(1, 1), 1);
        assert_eq!(mip_level_count(256, 256), 9);
        assert_eq!(mip_level_count(256, 64), 9);
        assert_eq!(mip_level_count(300, 10), 9);
    }
}
