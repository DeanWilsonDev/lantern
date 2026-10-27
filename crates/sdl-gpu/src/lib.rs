//! Lantern's SDL GPU backend (Umbra's `IRenderer` + Raylib renderer facade).
//!
//! `GpuRenderer` owns the GPU device, the pipelines and every GPU resource. The game talks to it
//! with `lantern-renderer-3d` types only: it uploads `MeshData` / `TextureData` and gets
//! generational handles back, and each frame it hands over a `RenderWorld`. No SDL type crosses
//! this crate's public API.
//!
//! Resource lifetime: `SDL_Release*` defers the actual destruction until the GPU has finished
//! every command buffer that uses the resource, so releasing a mesh mid-game is safe without our
//! own fences. On drop the renderer waits for the GPU to go idle, releases everything, gives the
//! window back and destroys the device.
//!
//! Every `unsafe` block below is an SDL call. The shared invariants are: `self.device` is live
//! from `new` until `drop`; every SDL object we store was created on that device and is released
//! exactly once; and pointers passed to SDL point at locals that outlive the call.

mod pipelines;
mod resources;
mod shaders;

use std::ffi::{CStr, CString};
use std::fmt;

use glam::{Mat4, Vec3, Vec4};
use lantern_core::color::Color;
use lantern_core::handle::Pool;
use lantern_core::math::Aabb;
use lantern_renderer_3d::render_world::{LineVertex, OverlayVertex};
use lantern_renderer_3d::texture::{TextureFilter, TextureWrap};
use lantern_renderer_3d::{MeshData, MeshHandle, RenderWorld, TextureData, TextureHandle};
use lantern_sdl_facade::SdlWindow;
use sdl3_sys::everything as sdl;

use pipelines::Pipelines;
use resources::{DynamicBuffer, GpuMesh, GpuTexture, Samplers};
use shaders::ShaderFormat;

#[derive(Debug, Clone)]
pub struct GpuError {
    pub message: String,
}

impl GpuError {
    pub(crate) fn new(message: impl Into<String>) -> GpuError {
        GpuError { message: message.into() }
    }

    /// A failed SDL call, with `SDL_GetError()` appended.
    pub(crate) fn sdl(context: &str) -> GpuError {
        // SAFETY: SDL_GetError returns a valid NUL-terminated string owned by SDL; we copy it.
        let detail = unsafe { CStr::from_ptr(sdl::SDL_GetError()) }.to_string_lossy();
        GpuError::new(format!("{context}: {detail}"))
    }
}

impl fmt::Display for GpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GpuError {}

/// Development settings for the device.
#[derive(Clone, Debug)]
pub struct GpuOptions {
    /// Enables SDL GPU debug mode and the backend's validation layers when installed.
    pub debug: bool,
    /// Force a driver (`"vulkan"`, `"metal"`); `None` lets SDL choose.
    pub driver: Option<String>,
    /// Wait for vertical blank. Turning it off is for measuring frame time headroom.
    pub vsync: bool,
}

impl GpuOptions {
    /// Debug mode follows the build profile; `LANTERN_GPU_DEBUG=0|1`, `LANTERN_GPU_DRIVER` and
    /// `LANTERN_VSYNC=0|1` override.
    pub fn from_environment() -> GpuOptions {
        let flag = |name: &str, default: bool| match std::env::var(name).as_deref() {
            Ok("0") | Ok("false") => false,
            Ok("1") | Ok("true") => true,
            _ => default,
        };
        GpuOptions {
            debug: flag("LANTERN_GPU_DEBUG", cfg!(debug_assertions)),
            driver: std::env::var("LANTERN_GPU_DRIVER").ok().filter(|name| !name.is_empty()),
            vsync: flag("LANTERN_VSYNC", true),
        }
    }
}

/// What happened in one `render` call, for the HUD.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameReport {
    pub presented: bool,
    pub draws_submitted: u32,
    pub draws_culled: u32,
    pub triangles: u64,
    pub line_count: u32,
}

struct DepthTarget {
    texture: *mut sdl::SDL_GPUTexture,
    width: u32,
    height: u32,
}

/// Render targets reused by `render_offscreen` until the size changes.
struct OffscreenTarget {
    color: *mut sdl::SDL_GPUTexture,
    depth: *mut sdl::SDL_GPUTexture,
    download: *mut sdl::SDL_GPUTransferBuffer,
    width: u32,
    height: u32,
}

impl OffscreenTarget {
    fn release(self, device: *mut sdl::SDL_GPUDevice) {
        // SAFETY: all three were created on `device` and are released once, here.
        unsafe {
            sdl::SDL_ReleaseGPUTexture(device, self.color);
            sdl::SDL_ReleaseGPUTexture(device, self.depth);
            sdl::SDL_ReleaseGPUTransferBuffer(device, self.download);
        }
    }
}

pub struct GpuRenderer {
    device: *mut sdl::SDL_GPUDevice,
    /// The window presented to, or `None` for an off-screen renderer (the world editor's
    /// viewport), which only draws with `render_offscreen` and `capture`.
    window: Option<SdlWindow>,
    offscreen: Option<OffscreenTarget>,
    /// An off-screen renderer initialised SDL's video subsystem itself and quits it on drop.
    owns_video_subsystem: bool,
    pipelines: Option<Pipelines>,
    samplers: Samplers,
    meshes: Pool<GpuMesh>,
    textures: Pool<GpuTexture>,
    white_texture: GpuTexture,
    line_buffer: DynamicBuffer,
    overlay_buffer: DynamicBuffer,
    depth: Option<DepthTarget>,
    color_format: sdl::SDL_GPUTextureFormat,
    depth_format: sdl::SDL_GPUTextureFormat,
    /// True when the swapchain is not sRGB, so shaders must gamma-encode their output.
    encode_srgb_in_shader: bool,
    driver_name: String,
    debug: bool,
    visible_draws: Vec<(u32, u32, usize)>,
}

impl GpuRenderer {
    pub fn new(window: &SdlWindow, options: &GpuOptions) -> Result<GpuRenderer, GpuError> {
        let driver = options.driver.as_deref().map(|name| CString::new(name).expect("driver names have no NUL"));
        let driver_pointer = driver.as_ref().map_or(std::ptr::null(), |name| name.as_ptr());
        // We ship SPIR-V and MSL (decision D-004), so SDL picks Vulkan or Metal.
        let formats = sdl::SDL_GPU_SHADERFORMAT_SPIRV | sdl::SDL_GPU_SHADERFORMAT_MSL;
        // SAFETY: SDL video is initialised (we hold a window); `driver_pointer` is NULL or a
        // NUL-terminated string that outlives the call.
        let device = unsafe { sdl::SDL_CreateGPUDevice(formats, options.debug, driver_pointer) };
        if device.is_null() {
            return Err(GpuError::sdl("SDL_CreateGPUDevice"));
        }
        // From here on, `device` must be destroyed on every error path; `DeviceGuard` does that
        // until ownership moves into the renderer.
        let guard = DeviceGuard(device);

        // SAFETY: live device; returns a static string.
        let driver_name = unsafe { CStr::from_ptr(sdl::SDL_GetGPUDeviceDriver(device)) }.to_string_lossy().into_owned();
        // SAFETY: live device.
        let supported = unsafe { sdl::SDL_GetGPUShaderFormats(device) };
        let shader_format = if (supported & sdl::SDL_GPU_SHADERFORMAT_SPIRV) == sdl::SDL_GPU_SHADERFORMAT_SPIRV {
            ShaderFormat::Spirv
        } else {
            ShaderFormat::Msl
        };

        // SAFETY: live device and window; the window is not claimed by another device.
        if !unsafe { sdl::SDL_ClaimWindowForGPUDevice(device, window.raw()) } {
            return Err(GpuError::sdl("SDL_ClaimWindowForGPUDevice"));
        }
        let encode_srgb_in_shader = configure_swapchain(device, window.raw(), options.vsync);
        // SAFETY: live device, claimed window.
        let color_format = unsafe { sdl::SDL_GetGPUSwapchainTextureFormat(device, window.raw()) };
        let depth_format = choose_depth_format(device);
        log::info!(
            target: "renderer",
            "GPU driver: {driver_name} ({shader_format:?} shaders), debug {}, swapchain format {}, depth format {}, {} output",
            if options.debug { "on" } else { "off" },
            color_format.0,
            depth_format.0,
            if encode_srgb_in_shader { "shader-encoded sRGB" } else { "hardware sRGB" }
        );

        let pipelines = match pipelines::create_pipelines(device, shader_format, color_format, depth_format) {
            Ok(pipelines) => pipelines,
            Err(error) => {
                // SAFETY: we claimed the window above; give it back before the guard destroys the device.
                unsafe { sdl::SDL_ReleaseWindowFromGPUDevice(device, window.raw()) };
                return Err(error);
            }
        };
        let samplers = Samplers::create(device)?;
        let white_texture = resources::upload_texture(device, &samplers, &TextureData::solid(Color::WHITE))?;
        std::mem::forget(guard);

        Ok(GpuRenderer {
            device,
            window: Some(window.clone()),
            offscreen: None,
            owns_video_subsystem: false,
            pipelines: Some(pipelines),
            samplers,
            meshes: Pool::new(),
            textures: Pool::new(),
            white_texture,
            line_buffer: DynamicBuffer::new(sdl::SDL_GPU_BUFFERUSAGE_VERTEX),
            overlay_buffer: DynamicBuffer::new(sdl::SDL_GPU_BUFFERUSAGE_VERTEX),
            depth: None,
            color_format,
            depth_format,
            encode_srgb_in_shader,
            driver_name,
            debug: options.debug,
            visible_draws: Vec::new(),
        })
    }

    /// A renderer with no window, for drawing into memory: the world editor renders its
    /// viewport this way and hands the pixels to its UI toolkit. Works whether or not someone
    /// else (the editor's UI) has initialised SDL; SDL counts subsystem references.
    pub fn new_offscreen(options: &GpuOptions) -> Result<GpuRenderer, GpuError> {
        // SAFETY: SDL_InitSubSystem is reference-counted and may be called at any time from the
        // main thread; the Vulkan backend needs video initialised to load its library.
        if !unsafe { sdl::SDL_InitSubSystem(sdl::SDL_INIT_VIDEO) } {
            return Err(GpuError::sdl("SDL_InitSubSystem(VIDEO)"));
        }
        let result = GpuRenderer::create_offscreen(options);
        if result.is_err() {
            // SAFETY: balances the successful SDL_InitSubSystem above.
            unsafe { sdl::SDL_QuitSubSystem(sdl::SDL_INIT_VIDEO) };
        }
        result
    }

    fn create_offscreen(options: &GpuOptions) -> Result<GpuRenderer, GpuError> {
        let driver = options.driver.as_deref().map(|name| CString::new(name).expect("driver names have no NUL"));
        let driver_pointer = driver.as_ref().map_or(std::ptr::null(), |name| name.as_ptr());
        let formats = sdl::SDL_GPU_SHADERFORMAT_SPIRV | sdl::SDL_GPU_SHADERFORMAT_MSL;
        // SAFETY: SDL video is initialised; `driver_pointer` is NULL or outlives the call.
        let device = unsafe { sdl::SDL_CreateGPUDevice(formats, options.debug, driver_pointer) };
        if device.is_null() {
            return Err(GpuError::sdl("SDL_CreateGPUDevice"));
        }
        let guard = DeviceGuard(device);
        // SAFETY: live device; returns a static string.
        let driver_name = unsafe { CStr::from_ptr(sdl::SDL_GetGPUDeviceDriver(device)) }.to_string_lossy().into_owned();
        // SAFETY: live device.
        let supported = unsafe { sdl::SDL_GetGPUShaderFormats(device) };
        let shader_format = if (supported & sdl::SDL_GPU_SHADERFORMAT_SPIRV) == sdl::SDL_GPU_SHADERFORMAT_SPIRV {
            ShaderFormat::Spirv
        } else {
            ShaderFormat::Msl
        };
        // An sRGB target lets the hardware encode, so the pixels read back are ready to show.
        // SAFETY: live device; a pure query.
        let srgb_supported = unsafe {
            sdl::SDL_GPUTextureSupportsFormat(
                device,
                sdl::SDL_GPU_TEXTUREFORMAT_R8G8B8A8_UNORM_SRGB,
                sdl::SDL_GPU_TEXTURETYPE_2D,
                sdl::SDL_GPU_TEXTUREUSAGE_COLOR_TARGET,
            )
        };
        let color_format = if srgb_supported {
            sdl::SDL_GPU_TEXTUREFORMAT_R8G8B8A8_UNORM_SRGB
        } else {
            sdl::SDL_GPU_TEXTUREFORMAT_R8G8B8A8_UNORM
        };
        let encode_srgb_in_shader = !srgb_supported;
        let depth_format = choose_depth_format(device);
        log::info!(
            target: "renderer",
            "off-screen GPU driver: {driver_name} ({shader_format:?} shaders), colour format {}, depth format {}",
            color_format.0,
            depth_format.0
        );
        let pipelines = pipelines::create_pipelines(device, shader_format, color_format, depth_format)?;
        let samplers = Samplers::create(device)?;
        let white_texture = resources::upload_texture(device, &samplers, &TextureData::solid(Color::WHITE))?;
        std::mem::forget(guard);
        Ok(GpuRenderer {
            device,
            window: None,
            offscreen: None,
            owns_video_subsystem: true,
            pipelines: Some(pipelines),
            samplers,
            meshes: Pool::new(),
            textures: Pool::new(),
            white_texture,
            line_buffer: DynamicBuffer::new(sdl::SDL_GPU_BUFFERUSAGE_VERTEX),
            overlay_buffer: DynamicBuffer::new(sdl::SDL_GPU_BUFFERUSAGE_VERTEX),
            depth: None,
            color_format,
            depth_format,
            encode_srgb_in_shader,
            driver_name,
            debug: options.debug,
            visible_draws: Vec::new(),
        })
    }

    pub fn driver_name(&self) -> &str {
        &self.driver_name
    }

    pub fn debug_enabled(&self) -> bool {
        self.debug
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }

    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Uploads a mesh. The returned handle stays valid until `remove_mesh`.
    pub fn create_mesh(&mut self, mesh: &MeshData) -> Result<MeshHandle, GpuError> {
        mesh.validate().map_err(GpuError::new)?;
        let gpu_mesh = resources::upload_mesh(self.device, mesh)?;
        Ok(self.meshes.insert(gpu_mesh).cast())
    }

    pub fn remove_mesh(&mut self, handle: MeshHandle) {
        if let Some(mesh) = self.meshes.remove(handle.cast()) {
            mesh.release(self.device);
        }
    }

    pub fn create_texture(&mut self, texture: &TextureData) -> Result<TextureHandle, GpuError> {
        let gpu_texture = resources::upload_texture(self.device, &self.samplers, texture)?;
        Ok(self.textures.insert(gpu_texture).cast())
    }

    pub fn remove_texture(&mut self, handle: TextureHandle) {
        if let Some(texture) = self.textures.remove(handle.cast()) {
            texture.release(self.device);
        }
    }

    /// Draws `world` to the window and presents it.
    pub fn render(&mut self, world: &RenderWorld) -> Result<FrameReport, GpuError> {
        let Some(window) = self.window.clone() else {
            return Err(GpuError::new("render needs a window; this renderer is off-screen"));
        };
        // SAFETY: live device.
        let command_buffer = unsafe { sdl::SDL_AcquireGPUCommandBuffer(self.device) };
        if command_buffer.is_null() {
            return Err(GpuError::sdl("SDL_AcquireGPUCommandBuffer"));
        }
        self.upload_dynamic_geometry(command_buffer, world)?;

        let mut swapchain_texture = std::ptr::null_mut();
        let (mut width, mut height) = (0u32, 0u32);
        // SAFETY: live command buffer and claimed window; out-pointers are locals.
        let acquired = unsafe {
            sdl::SDL_WaitAndAcquireGPUSwapchainTexture(
                command_buffer,
                window.raw(),
                &mut swapchain_texture,
                &mut width,
                &mut height,
            )
        };
        if !acquired {
            let error = GpuError::sdl("SDL_WaitAndAcquireGPUSwapchainTexture");
            // SAFETY: a command buffer must be submitted or cancelled; nothing was recorded that
            // needs presenting.
            unsafe { sdl::SDL_CancelGPUCommandBuffer(command_buffer) };
            return Err(error);
        }
        let mut report = FrameReport::default();
        if swapchain_texture.is_null() {
            // Minimised or occluded: nothing to draw into this frame. Submit the uploads anyway.
            // SAFETY: live command buffer, submitted once.
            unsafe { sdl::SDL_SubmitGPUCommandBuffer(command_buffer) };
            return Ok(report);
        }
        self.ensure_depth_target(width, height)?;
        let depth_texture = self.depth.as_ref().expect("ensured above").texture;
        self.record_frame(command_buffer, swapchain_texture, depth_texture, width, height, world, &mut report);
        // SAFETY: live command buffer, submitted once.
        if !unsafe { sdl::SDL_SubmitGPUCommandBuffer(command_buffer) } {
            return Err(GpuError::sdl("SDL_SubmitGPUCommandBuffer"));
        }
        report.presented = true;
        Ok(report)
    }

    /// Renders `world` at `width` x `height` and copies the result into `pixels` as RGBA8,
    /// top row first (sRGB-encoded, ready to display). The render targets are kept and reused
    /// while the size stays the same. Waits for the GPU, so it costs a frame of latency
    /// at most; fine for an editor viewport.
    pub fn render_offscreen(
        &mut self,
        world: &RenderWorld,
        width: u32,
        height: u32,
        pixels: &mut Vec<u8>,
    ) -> Result<FrameReport, GpuError> {
        let (width, height) = (width.max(1), height.max(1));
        if self.offscreen.as_ref().is_none_or(|target| target.width != width || target.height != height) {
            if let Some(old) = self.offscreen.take() {
                // SAFETY: live device; nothing may still use the old targets once idle.
                unsafe { sdl::SDL_WaitForGPUIdle(self.device) };
                old.release(self.device);
            }
            let color = resources::create_render_texture(self.device, self.color_format, width, height, false)?;
            let depth = resources::create_render_texture(self.device, self.depth_format, width, height, true)?;
            let download = resources::create_transfer_buffer(self.device, width * height * 4, false)?;
            self.offscreen = Some(OffscreenTarget { color, depth, download, width, height });
        }
        let target = self.offscreen.as_ref().expect("created above");
        let (color, depth, download) = (target.color, target.depth, target.download);
        let byte_count = (width * height * 4) as usize;

        // SAFETY: live device.
        let command_buffer = unsafe { sdl::SDL_AcquireGPUCommandBuffer(self.device) };
        if command_buffer.is_null() {
            return Err(GpuError::sdl("SDL_AcquireGPUCommandBuffer"));
        }
        self.upload_dynamic_geometry(command_buffer, world)?;
        let mut report = FrameReport::default();
        self.record_frame(command_buffer, color, depth, width, height, world, &mut report);
        let source = sdl::SDL_GPUTextureRegion { texture: color, w: width, h: height, d: 1, ..Default::default() };
        let destination = sdl::SDL_GPUTextureTransferInfo { transfer_buffer: download, ..Default::default() };
        // SAFETY: live command buffer; the copy pass is begun and ended here; `color` and
        // `download` are sized for `width * height` 4-byte texels.
        let fence = unsafe {
            let copy_pass = sdl::SDL_BeginGPUCopyPass(command_buffer);
            sdl::SDL_DownloadFromGPUTexture(copy_pass, &source, &destination);
            sdl::SDL_EndGPUCopyPass(copy_pass);
            sdl::SDL_SubmitGPUCommandBufferAndAcquireFence(command_buffer)
        };
        if fence.is_null() {
            return Err(GpuError::sdl("SDL_SubmitGPUCommandBufferAndAcquireFence"));
        }
        pixels.resize(byte_count, 0);
        // SAFETY: the fence belongs to this device; after waiting, the download buffer holds the
        // pixels and may be mapped; the mapped range is `byte_count` bytes.
        unsafe {
            sdl::SDL_WaitForGPUFences(self.device, true, &fence, 1);
            sdl::SDL_ReleaseGPUFence(self.device, fence);
            let mapped = sdl::SDL_MapGPUTransferBuffer(self.device, download, false).cast::<u8>();
            if mapped.is_null() {
                return Err(GpuError::sdl("SDL_MapGPUTransferBuffer"));
            }
            std::ptr::copy_nonoverlapping(mapped, pixels.as_mut_ptr(), byte_count);
            sdl::SDL_UnmapGPUTransferBuffer(self.device, download);
        }
        if is_bgra(self.color_format) {
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        report.presented = true;
        Ok(report)
    }

    /// Renders `world` into an off-screen image and reads it back as RGBA8 (top row first).
    /// Used by tests and the `--capture` option to check output without looking at a window.
    pub fn capture(&mut self, world: &RenderWorld, width: u32, height: u32) -> Result<TextureData, GpuError> {
        let color = resources::create_render_texture(self.device, self.color_format, width, height, false)?;
        let depth = resources::create_render_texture(self.device, self.depth_format, width, height, true)?;
        let byte_count = width * height * 4;
        let download = resources::create_transfer_buffer(self.device, byte_count, false)?;

        // SAFETY: live device.
        let command_buffer = unsafe { sdl::SDL_AcquireGPUCommandBuffer(self.device) };
        if command_buffer.is_null() {
            return Err(GpuError::sdl("SDL_AcquireGPUCommandBuffer"));
        }
        self.upload_dynamic_geometry(command_buffer, world)?;
        let mut report = FrameReport::default();
        self.record_frame(command_buffer, color, depth, width, height, world, &mut report);
        let source = sdl::SDL_GPUTextureRegion { texture: color, w: width, h: height, d: 1, ..Default::default() };
        let destination = sdl::SDL_GPUTextureTransferInfo { transfer_buffer: download, ..Default::default() };
        // SAFETY: live command buffer; the copy pass is begun and ended here; `color` and
        // `download` are sized for `width * height` 4-byte texels.
        let fence = unsafe {
            let copy_pass = sdl::SDL_BeginGPUCopyPass(command_buffer);
            sdl::SDL_DownloadFromGPUTexture(copy_pass, &source, &destination);
            sdl::SDL_EndGPUCopyPass(copy_pass);
            sdl::SDL_SubmitGPUCommandBufferAndAcquireFence(command_buffer)
        };
        if fence.is_null() {
            return Err(GpuError::sdl("SDL_SubmitGPUCommandBufferAndAcquireFence"));
        }
        let mut rgba8 = vec![0u8; byte_count as usize];
        // SAFETY: the fence belongs to this device; after waiting, the download buffer holds the
        // pixels and may be mapped; the mapped range is `byte_count` bytes.
        unsafe {
            sdl::SDL_WaitForGPUFences(self.device, true, &fence, 1);
            sdl::SDL_ReleaseGPUFence(self.device, fence);
            let mapped = sdl::SDL_MapGPUTransferBuffer(self.device, download, false).cast::<u8>();
            if mapped.is_null() {
                return Err(GpuError::sdl("SDL_MapGPUTransferBuffer"));
            }
            std::ptr::copy_nonoverlapping(mapped, rgba8.as_mut_ptr(), rgba8.len());
            sdl::SDL_UnmapGPUTransferBuffer(self.device, download);
            sdl::SDL_ReleaseGPUTransferBuffer(self.device, download);
            sdl::SDL_ReleaseGPUTexture(self.device, color);
            sdl::SDL_ReleaseGPUTexture(self.device, depth);
        }
        if is_bgra(self.color_format) {
            for pixel in rgba8.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
        }
        Ok(TextureData { width, height, rgba8, filter: TextureFilter::Nearest, wrap: TextureWrap::Clamp })
    }

    fn upload_dynamic_geometry(
        &mut self,
        command_buffer: *mut sdl::SDL_GPUCommandBuffer,
        world: &RenderWorld,
    ) -> Result<(), GpuError> {
        let line_bytes = as_bytes_lines(&world.lines);
        let overlay_bytes = as_bytes_overlay(&world.overlay.vertices);
        if line_bytes.is_empty() && overlay_bytes.is_empty() {
            return Ok(());
        }
        self.line_buffer.reserve(self.device, line_bytes.len() as u32)?;
        self.overlay_buffer.reserve(self.device, overlay_bytes.len() as u32)?;
        // SAFETY: live command buffer; the copy pass is begun and ended here.
        let copy_pass = unsafe { sdl::SDL_BeginGPUCopyPass(command_buffer) };
        self.line_buffer.upload(self.device, copy_pass, &line_bytes);
        self.overlay_buffer.upload(self.device, copy_pass, &overlay_bytes);
        // SAFETY: ends the pass begun above.
        unsafe { sdl::SDL_EndGPUCopyPass(copy_pass) };
        Ok(())
    }

    fn ensure_depth_target(&mut self, width: u32, height: u32) -> Result<(), GpuError> {
        if let Some(depth) = &self.depth {
            if depth.width == width && depth.height == height {
                return Ok(());
            }
        }
        if let Some(old) = self.depth.take() {
            // SAFETY: created on this device; SDL defers destruction until in-flight frames finish.
            unsafe { sdl::SDL_ReleaseGPUTexture(self.device, old.texture) };
        }
        let texture = resources::create_render_texture(self.device, self.depth_format, width, height, true)?;
        log::debug!(target: "renderer", "depth buffer recreated at {width}x{height}");
        self.depth = Some(DepthTarget { texture, width, height });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn record_frame(
        &mut self,
        command_buffer: *mut sdl::SDL_GPUCommandBuffer,
        color_texture: *mut sdl::SDL_GPUTexture,
        depth_texture: *mut sdl::SDL_GPUTexture,
        width: u32,
        height: u32,
        world: &RenderWorld,
        report: &mut FrameReport,
    ) {
        let clear = if self.encode_srgb_in_shader { encode_srgb(world.clear_color) } else { world.clear_color };
        let color_target = sdl::SDL_GPUColorTargetInfo {
            texture: color_texture,
            clear_color: sdl::SDL_FColor { r: clear.r, g: clear.g, b: clear.b, a: 1.0 },
            load_op: sdl::SDL_GPU_LOADOP_CLEAR,
            store_op: sdl::SDL_GPU_STOREOP_STORE,
            ..Default::default()
        };
        let depth_target = sdl::SDL_GPUDepthStencilTargetInfo {
            texture: depth_texture,
            clear_depth: 1.0,
            load_op: sdl::SDL_GPU_LOADOP_CLEAR,
            store_op: sdl::SDL_GPU_STOREOP_DONT_CARE,
            stencil_load_op: sdl::SDL_GPU_LOADOP_DONT_CARE,
            stencil_store_op: sdl::SDL_GPU_STOREOP_DONT_CARE,
            cycle: true,
            ..Default::default()
        };
        // SAFETY: live command buffer and targets created on this device; ended below.
        let pass = unsafe { sdl::SDL_BeginGPURenderPass(command_buffer, &color_target, 1, &depth_target) };
        let pipelines = self.pipelines.as_ref().expect("pipelines live until drop");
        let encode_flag = if self.encode_srgb_in_shader { 1.0 } else { 0.0 };
        let aspect = width as f32 / height.max(1) as f32;
        let view_projection = world.camera.view_projection(aspect);
        let camera_position = world.camera.position.extend(1.0);

        // Opaque meshes, culled against the frustum and sorted by texture then mesh so state
        // changes are rare.
        let frustum = Frustum::from_view_projection(&view_projection);
        self.visible_draws.clear();
        for (index, draw) in world.draws.iter().enumerate() {
            let Some(mesh) = self.meshes.get(draw.mesh.cast()) else {
                log::warn!(target: "renderer", "draw item {index} uses a released mesh {:?}", draw.mesh);
                continue;
            };
            if !frustum.intersects(&mesh.bounds.transformed(&draw.transform)) {
                report.draws_culled += 1;
                continue;
            }
            let texture_key = draw.texture.map_or(u32::MAX, |texture| texture.index());
            self.visible_draws.push((texture_key, draw.mesh.index(), index));
        }
        self.visible_draws.sort_unstable();

        let frame_uniforms = mat4_and_vec4(&view_projection, camera_position);
        let direction_to_light = (-world.light.direction).normalize_or(Vec3::Y).extend(0.0);
        let light_uniforms: [f32; 24] = concat_vec4s(&[
            direction_to_light,
            color_vec4(world.light.color),
            color_vec4(world.light.ambient),
            color_vec4(world.clear_color),
            Vec4::new(world.fog_start, world.fog_end, 0.0, encode_flag),
            camera_position,
        ]);
        // SAFETY (whole block): `pass` is the live render pass begun above; pipelines, buffers and
        // samplers were created on this device; uniform pointers reference locals whose byte
        // sizes are passed alongside.
        unsafe {
            sdl::SDL_BindGPUGraphicsPipeline(pass, pipelines.lit);
            push_vertex_uniform(command_buffer, 0, &frame_uniforms);
            push_fragment_uniform(command_buffer, 0, &light_uniforms);
            let mut bound_mesh = u32::MAX;
            let mut bound_texture: Option<u32> = None;
            for &(texture_key, mesh_index, draw_index) in &self.visible_draws {
                let draw = &world.draws[draw_index];
                let mesh = self.meshes.get(draw.mesh.cast()).expect("checked while culling");
                if mesh_index != bound_mesh {
                    let vertex_binding = sdl::SDL_GPUBufferBinding { buffer: mesh.vertex_buffer, offset: 0 };
                    let index_binding = sdl::SDL_GPUBufferBinding { buffer: mesh.index_buffer, offset: 0 };
                    sdl::SDL_BindGPUVertexBuffers(pass, 0, &vertex_binding, 1);
                    sdl::SDL_BindGPUIndexBuffer(pass, &index_binding, sdl::SDL_GPU_INDEXELEMENTSIZE_32BIT);
                    bound_mesh = mesh_index;
                }
                if bound_texture != Some(texture_key) {
                    let texture =
                        draw.texture.and_then(|handle| self.textures.get(handle.cast())).unwrap_or(&self.white_texture);
                    let binding =
                        sdl::SDL_GPUTextureSamplerBinding { texture: texture.texture, sampler: texture.sampler };
                    sdl::SDL_BindGPUFragmentSamplers(pass, 0, &binding, 1);
                    bound_texture = Some(texture_key);
                }
                let normal_matrix = Mat4::from_mat3(glam::Mat3::from_mat4(draw.transform).inverse().transpose());
                let object_uniforms: [f32; 32] = concat_mat4s(&draw.transform, &normal_matrix);
                push_vertex_uniform(command_buffer, 1, &object_uniforms);
                push_fragment_uniform(command_buffer, 1, &color_vec4(draw.color).to_array());
                sdl::SDL_DrawGPUIndexedPrimitives(pass, mesh.index_count, 1, 0, 0, 0);
                report.draws_submitted += 1;
                report.triangles += u64::from(mesh.index_count / 3);
            }

            if !world.lines.is_empty() {
                sdl::SDL_BindGPUGraphicsPipeline(pass, pipelines.lines);
                push_vertex_uniform(
                    command_buffer,
                    0,
                    &mat4_and_vec4(&view_projection, Vec4::new(0.0, 0.0, 0.0, encode_flag)),
                );
                let binding = sdl::SDL_GPUBufferBinding { buffer: self.line_buffer.buffer, offset: 0 };
                sdl::SDL_BindGPUVertexBuffers(pass, 0, &binding, 1);
                sdl::SDL_DrawGPUPrimitives(pass, world.lines.len() as u32, 1, 0, 0);
                report.line_count = world.lines.len() as u32 / 2;
            }

            if !world.overlay.vertices.is_empty() {
                let atlas = world
                    .overlay_texture
                    .and_then(|handle| self.textures.get(handle.cast()))
                    .unwrap_or(&self.white_texture);
                sdl::SDL_BindGPUGraphicsPipeline(pass, pipelines.overlay);
                push_vertex_uniform(command_buffer, 0, &[width as f32, height as f32, 0.0, encode_flag]);
                let sampler_binding =
                    sdl::SDL_GPUTextureSamplerBinding { texture: atlas.texture, sampler: atlas.sampler };
                sdl::SDL_BindGPUFragmentSamplers(pass, 0, &sampler_binding, 1);
                let binding = sdl::SDL_GPUBufferBinding { buffer: self.overlay_buffer.buffer, offset: 0 };
                sdl::SDL_BindGPUVertexBuffers(pass, 0, &binding, 1);
                sdl::SDL_DrawGPUPrimitives(pass, world.overlay.vertices.len() as u32, 1, 0, 0);
            }
            sdl::SDL_EndGPURenderPass(pass);
        }
    }
}

impl Drop for GpuRenderer {
    fn drop(&mut self) {
        // SAFETY: SDL_WasInit only reads SDL's subsystem state.
        let video_still_initialised = unsafe { sdl::SDL_WasInit(sdl::SDL_INIT_VIDEO) } != 0;
        if !video_still_initialised {
            // Another SDL user in the process (the world editor's Penumbra host) called
            // SDL_Quit, which unloaded the Vulkan library the device's destructor would call
            // into. The process is exiting, so the device is left to the driver.
            log::warn!(
                target: "renderer",
                "SDL video was shut down before the GPU device; the device is not destroyed"
            );
            return;
        }
        // SAFETY: live device. Waiting first means nothing below is still in use by the GPU.
        unsafe { sdl::SDL_WaitForGPUIdle(self.device) };
        for mesh in self.meshes.drain() {
            mesh.release(self.device);
        }
        for texture in self.textures.drain() {
            texture.release(self.device);
        }
        self.white_texture.release(self.device);
        self.line_buffer.release(self.device);
        self.overlay_buffer.release(self.device);
        if let Some(depth) = self.depth.take() {
            // SAFETY: created on this device, released once.
            unsafe { sdl::SDL_ReleaseGPUTexture(self.device, depth.texture) };
        }
        if let Some(target) = self.offscreen.take() {
            target.release(self.device);
        }
        self.samplers.release(self.device);
        self.pipelines = None;
        // SAFETY: a window was claimed in `new` and is still alive (we hold an `SdlWindow`);
        // the device is destroyed last, once nothing created on it remains.
        unsafe {
            if let Some(window) = &self.window {
                sdl::SDL_ReleaseWindowFromGPUDevice(self.device, window.raw());
            }
            sdl::SDL_DestroyGPUDevice(self.device);
            if self.owns_video_subsystem {
                sdl::SDL_QuitSubSystem(sdl::SDL_INIT_VIDEO);
            }
        }
        log::info!(target: "renderer", "GPU device destroyed");
    }
}

/// Destroys a device if `GpuRenderer::new` fails part-way.
struct DeviceGuard(*mut sdl::SDL_GPUDevice);

impl Drop for DeviceGuard {
    fn drop(&mut self) {
        // SAFETY: the device was created in `new` and nothing else owns it yet.
        unsafe { sdl::SDL_DestroyGPUDevice(self.0) };
    }
}

/// Prefers an sRGB swapchain (hardware encoding of linear shader output). Returns `true` when
/// that is unavailable and shaders must encode instead.
fn configure_swapchain(device: *mut sdl::SDL_GPUDevice, window: *mut sdl::SDL_Window, vsync: bool) -> bool {
    // SAFETY (whole function): live device and a window claimed by it.
    unsafe {
        let srgb = sdl::SDL_WindowSupportsGPUSwapchainComposition(
            device,
            window,
            sdl::SDL_GPU_SWAPCHAINCOMPOSITION_SDR_LINEAR,
        );
        let composition =
            if srgb { sdl::SDL_GPU_SWAPCHAINCOMPOSITION_SDR_LINEAR } else { sdl::SDL_GPU_SWAPCHAINCOMPOSITION_SDR };
        let present_mode = if vsync {
            sdl::SDL_GPU_PRESENTMODE_VSYNC
        } else if sdl::SDL_WindowSupportsGPUPresentMode(device, window, sdl::SDL_GPU_PRESENTMODE_MAILBOX) {
            sdl::SDL_GPU_PRESENTMODE_MAILBOX
        } else if sdl::SDL_WindowSupportsGPUPresentMode(device, window, sdl::SDL_GPU_PRESENTMODE_IMMEDIATE) {
            sdl::SDL_GPU_PRESENTMODE_IMMEDIATE
        } else {
            sdl::SDL_GPU_PRESENTMODE_VSYNC
        };
        if !sdl::SDL_SetGPUSwapchainParameters(device, window, composition, present_mode) {
            log::warn!(target: "renderer", "{}", GpuError::sdl("SDL_SetGPUSwapchainParameters"));
            return true;
        }
        !srgb
    }
}

fn choose_depth_format(device: *mut sdl::SDL_GPUDevice) -> sdl::SDL_GPUTextureFormat {
    for format in [
        sdl::SDL_GPU_TEXTUREFORMAT_D32_FLOAT,
        sdl::SDL_GPU_TEXTUREFORMAT_D24_UNORM,
        sdl::SDL_GPU_TEXTUREFORMAT_D16_UNORM,
    ] {
        // SAFETY: live device; a pure query.
        let supported = unsafe {
            sdl::SDL_GPUTextureSupportsFormat(
                device,
                format,
                sdl::SDL_GPU_TEXTURETYPE_2D,
                sdl::SDL_GPU_TEXTUREUSAGE_DEPTH_STENCIL_TARGET,
            )
        };
        if supported {
            return format;
        }
    }
    // D16 is required by the SDL GPU spec on every backend.
    sdl::SDL_GPU_TEXTUREFORMAT_D16_UNORM
}

fn is_bgra(format: sdl::SDL_GPUTextureFormat) -> bool {
    format == sdl::SDL_GPU_TEXTUREFORMAT_B8G8R8A8_UNORM || format == sdl::SDL_GPU_TEXTUREFORMAT_B8G8R8A8_UNORM_SRGB
}

fn encode_srgb(color: Color) -> Color {
    let encode = |linear: f32| {
        let linear = linear.clamp(0.0, 1.0);
        if linear <= 0.003_130_8 { linear * 12.92 } else { 1.055 * linear.powf(1.0 / 2.4) - 0.055 }
    };
    Color::rgba(encode(color.r), encode(color.g), encode(color.b), color.a)
}

fn color_vec4(color: Color) -> Vec4 {
    Vec4::from_array(color.to_array())
}

fn concat_vec4s<const N: usize>(vectors: &[Vec4]) -> [f32; N] {
    let mut out = [0.0; N];
    for (index, vector) in vectors.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&vector.to_array());
    }
    out
}

fn mat4_and_vec4(matrix: &Mat4, vector: Vec4) -> [f32; 20] {
    let mut out = [0.0; 20];
    out[..16].copy_from_slice(&matrix.to_cols_array());
    out[16..].copy_from_slice(&vector.to_array());
    out
}

fn concat_mat4s(first: &Mat4, second: &Mat4) -> [f32; 32] {
    let mut out = [0.0; 32];
    out[..16].copy_from_slice(&first.to_cols_array());
    out[16..].copy_from_slice(&second.to_cols_array());
    out
}

/// # Safety
/// `command_buffer` must be a live command buffer.
unsafe fn push_vertex_uniform(command_buffer: *mut sdl::SDL_GPUCommandBuffer, slot: u32, data: &[f32]) {
    // SAFETY: caller guarantees the command buffer; `data` is a valid slice of the given size.
    unsafe {
        sdl::SDL_PushGPUVertexUniformData(
            command_buffer,
            slot,
            data.as_ptr().cast(),
            std::mem::size_of_val(data) as u32,
        )
    };
}

/// # Safety
/// `command_buffer` must be a live command buffer.
unsafe fn push_fragment_uniform(command_buffer: *mut sdl::SDL_GPUCommandBuffer, slot: u32, data: &[f32]) {
    // SAFETY: as above.
    unsafe {
        sdl::SDL_PushGPUFragmentUniformData(
            command_buffer,
            slot,
            data.as_ptr().cast(),
            std::mem::size_of_val(data) as u32,
        )
    };
}

fn as_bytes_lines(vertices: &[LineVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * 28);
    for vertex in vertices {
        for value in vertex.position.iter().chain(&vertex.color) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

fn as_bytes_overlay(vertices: &[OverlayVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * 32);
    for vertex in vertices {
        for value in vertex.position.iter().chain(&vertex.uv).chain(&vertex.color) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

/// Six planes `(normal, distance)` with normals pointing into the frustum, extracted from a
/// depth-0..1 view-projection matrix (Gribb & Hartmann).
struct Frustum {
    planes: [Vec4; 6],
}

impl Frustum {
    fn from_view_projection(matrix: &Mat4) -> Frustum {
        let [row0, row1, row2, row3] = [0, 1, 2, 3].map(|index| matrix.row(index));
        let planes = [row3 + row0, row3 - row0, row3 + row1, row3 - row1, row2, row3 - row2]
            .map(|plane| plane / plane.truncate().length().max(1e-6));
        Frustum { planes }
    }

    fn intersects(&self, bounds: &Aabb) -> bool {
        self.planes.iter().all(|plane| {
            let normal = plane.truncate();
            // The box corner furthest along the plane normal.
            let farthest = Vec3::select(normal.cmpge(Vec3::ZERO), bounds.max, bounds.min);
            normal.dot(farthest) + plane.w >= 0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lantern_renderer_3d::Camera;

    #[test]
    fn frustum_keeps_what_the_camera_sees_and_drops_what_is_behind_it() {
        let camera = Camera::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        let frustum = Frustum::from_view_projection(&camera.view_projection(1.0));
        let unit = |center: Vec3| Aabb::from_center_half_extents(center, Vec3::splat(0.5));
        assert!(frustum.intersects(&unit(Vec3::ZERO)));
        assert!(!frustum.intersects(&unit(Vec3::new(0.0, 0.0, 10.0))), "behind the camera");
        assert!(!frustum.intersects(&unit(Vec3::new(50.0, 0.0, 0.0))), "far to the side");
        assert!(!frustum.intersects(&unit(Vec3::new(0.0, 0.0, -400.0))), "beyond the far plane");
        assert!(frustum.intersects(&unit(Vec3::new(2.5, 0.0, 0.0))), "partly visible at the edge");
    }
}
