//! The three graphics pipelines: lit meshes, debug lines and the 2D overlay.

use lantern_renderer_3d::Vertex;
use lantern_renderer_3d::render_world::{LineVertex, OverlayVertex};
use sdl3_sys::everything as sdl;

use crate::GpuError;
use crate::shaders::{self, CompiledShader, ShaderFormat};

pub(crate) struct Pipelines {
    device: *mut sdl::SDL_GPUDevice,
    pub lit: *mut sdl::SDL_GPUGraphicsPipeline,
    pub lines: *mut sdl::SDL_GPUGraphicsPipeline,
    pub overlay: *mut sdl::SDL_GPUGraphicsPipeline,
}

impl Drop for Pipelines {
    fn drop(&mut self) {
        for pipeline in [self.lit, self.lines, self.overlay] {
            if !pipeline.is_null() {
                // SAFETY: each pipeline was created on `device` and is released once; the owning
                // `GpuRenderer` drops `Pipelines` before destroying the device.
                unsafe { sdl::SDL_ReleaseGPUGraphicsPipeline(self.device, pipeline) };
            }
        }
    }
}

struct PipelineDescription<'a> {
    vertex: &'a CompiledShader,
    fragment: &'a CompiledShader,
    stride: u32,
    attributes: &'a [(sdl::SDL_GPUVertexElementFormat, u32)],
    primitive: sdl::SDL_GPUPrimitiveType,
    cull_back_faces: bool,
    depth_test: bool,
    depth_write: bool,
    alpha_blend: bool,
}

pub(crate) fn create_pipelines(
    device: *mut sdl::SDL_GPUDevice,
    format: ShaderFormat,
    color_format: sdl::SDL_GPUTextureFormat,
    depth_format: sdl::SDL_GPUTextureFormat,
) -> Result<Pipelines, GpuError> {
    let mut pipelines =
        Pipelines { device, lit: std::ptr::null_mut(), lines: std::ptr::null_mut(), overlay: std::ptr::null_mut() };
    pipelines.lit = create(
        device,
        format,
        color_format,
        depth_format,
        &PipelineDescription {
            vertex: &shaders::LIT_VERTEX,
            fragment: &shaders::LIT_FRAGMENT,
            stride: Vertex::STRIDE,
            attributes: &[
                (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT3, 0),
                (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT3, 12),
                (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT2, 24),
                (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT4, 32),
            ],
            primitive: sdl::SDL_GPU_PRIMITIVETYPE_TRIANGLELIST,
            cull_back_faces: true,
            depth_test: true,
            depth_write: true,
            alpha_blend: false,
        },
    )?;
    pipelines.lines = create(
        device,
        format,
        color_format,
        depth_format,
        &PipelineDescription {
            vertex: &shaders::LINES_VERTEX,
            fragment: &shaders::LINES_FRAGMENT,
            stride: std::mem::size_of::<LineVertex>() as u32,
            attributes: &[(sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT3, 0), (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT4, 12)],
            primitive: sdl::SDL_GPU_PRIMITIVETYPE_LINELIST,
            cull_back_faces: false,
            depth_test: true,
            depth_write: false,
            alpha_blend: true,
        },
    )?;
    pipelines.overlay = create(
        device,
        format,
        color_format,
        depth_format,
        &PipelineDescription {
            vertex: &shaders::OVERLAY_VERTEX,
            fragment: &shaders::OVERLAY_FRAGMENT,
            stride: std::mem::size_of::<OverlayVertex>() as u32,
            attributes: &[
                (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT2, 0),
                (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT2, 8),
                (sdl::SDL_GPU_VERTEXELEMENTFORMAT_FLOAT4, 16),
            ],
            primitive: sdl::SDL_GPU_PRIMITIVETYPE_TRIANGLELIST,
            cull_back_faces: false,
            depth_test: false,
            depth_write: false,
            alpha_blend: true,
        },
    )?;
    Ok(pipelines)
}

fn create(
    device: *mut sdl::SDL_GPUDevice,
    format: ShaderFormat,
    color_format: sdl::SDL_GPUTextureFormat,
    depth_format: sdl::SDL_GPUTextureFormat,
    description: &PipelineDescription,
) -> Result<*mut sdl::SDL_GPUGraphicsPipeline, GpuError> {
    let vertex_shader = shaders::create_shader(device, description.vertex, format)?;
    let fragment_shader = shaders::create_shader(device, description.fragment, format)?;

    let buffer_descriptions = [sdl::SDL_GPUVertexBufferDescription {
        slot: 0,
        pitch: description.stride,
        input_rate: sdl::SDL_GPU_VERTEXINPUTRATE_VERTEX,
        instance_step_rate: 0,
    }];
    let attributes: Vec<sdl::SDL_GPUVertexAttribute> = description
        .attributes
        .iter()
        .enumerate()
        .map(|(location, &(element_format, offset))| sdl::SDL_GPUVertexAttribute {
            location: location as u32,
            buffer_slot: 0,
            format: element_format,
            offset,
        })
        .collect();
    let blend_state = if description.alpha_blend {
        sdl::SDL_GPUColorTargetBlendState {
            src_color_blendfactor: sdl::SDL_GPU_BLENDFACTOR_SRC_ALPHA,
            dst_color_blendfactor: sdl::SDL_GPU_BLENDFACTOR_ONE_MINUS_SRC_ALPHA,
            color_blend_op: sdl::SDL_GPU_BLENDOP_ADD,
            src_alpha_blendfactor: sdl::SDL_GPU_BLENDFACTOR_ONE,
            dst_alpha_blendfactor: sdl::SDL_GPU_BLENDFACTOR_ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: sdl::SDL_GPU_BLENDOP_ADD,
            enable_blend: true,
            ..Default::default()
        }
    } else {
        sdl::SDL_GPUColorTargetBlendState::default()
    };
    let color_targets = [sdl::SDL_GPUColorTargetDescription { format: color_format, blend_state }];
    let info = sdl::SDL_GPUGraphicsPipelineCreateInfo {
        vertex_shader: vertex_shader.raw,
        fragment_shader: fragment_shader.raw,
        vertex_input_state: sdl::SDL_GPUVertexInputState {
            vertex_buffer_descriptions: buffer_descriptions.as_ptr(),
            num_vertex_buffers: 1,
            vertex_attributes: attributes.as_ptr(),
            num_vertex_attributes: attributes.len() as u32,
        },
        primitive_type: description.primitive,
        rasterizer_state: sdl::SDL_GPURasterizerState {
            fill_mode: sdl::SDL_GPU_FILLMODE_FILL,
            cull_mode: if description.cull_back_faces {
                sdl::SDL_GPU_CULLMODE_BACK
            } else {
                sdl::SDL_GPU_CULLMODE_NONE
            },
            front_face: sdl::SDL_GPU_FRONTFACE_COUNTER_CLOCKWISE,
            ..Default::default()
        },
        depth_stencil_state: sdl::SDL_GPUDepthStencilState {
            compare_op: sdl::SDL_GPU_COMPAREOP_LESS,
            enable_depth_test: description.depth_test,
            enable_depth_write: description.depth_write,
            ..Default::default()
        },
        target_info: sdl::SDL_GPUGraphicsPipelineTargetInfo {
            color_target_descriptions: color_targets.as_ptr(),
            num_color_targets: 1,
            depth_stencil_format: depth_format,
            has_depth_stencil_target: true,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: `device` is live; every pointer in `info` points to locals that outlive the call.
    let pipeline = unsafe { sdl::SDL_CreateGPUGraphicsPipeline(device, &info) };
    if pipeline.is_null() {
        return Err(GpuError::sdl(&format!("SDL_CreateGPUGraphicsPipeline({})", description.vertex.name)));
    }
    // The shader objects drop here; the pipeline keeps what it needs.
    Ok(pipeline)
}
