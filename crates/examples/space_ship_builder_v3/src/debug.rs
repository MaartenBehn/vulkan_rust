use crate::renderer::Renderer;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::{vec3, vec4, IVec2, IVec3, Mat4, UVec2, Vec3, Vec4};
use octa_force::imgui::internal::ImVector;
use octa_force::imgui::sys::ImGuiConfigFlags;
use octa_force::imgui::{
    BackendFlags, Condition, ConfigFlags, DrawData, DrawVert, FontConfig, FontSource,
    PlatformMonitor, PlatformViewportBackend, RendererViewportBackend, Viewport,
};
use octa_force::imgui_rs_vulkan_renderer::{DynamicRendering, Options};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::Extent2D;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    Buffer, CommandBuffer, CommandPool, Context, DescriptorPool, DescriptorSet,
    DescriptorSetLayout, GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo,
    PipelineLayout, WriteDescriptorSet, WriteDescriptorSetKind,
};
use octa_force::{imgui, imgui_rs_vulkan_renderer, log};
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::{align_of, size_of};
use std::time::Duration;

#[derive(PartialEq)]
pub enum DebugMode {
    OFF,
    WFC,
}

const DEBUG_MODE_CHANGE_SPEED: Duration = Duration::from_millis(100);

pub struct DebugController {
    pub mode: DebugMode,
    pub line_renderer: DebugLineRenderer,
    pub text_renderer: Vec<DebugTextRenderer>,

    last_mode_change: Duration,
}

pub struct DebugLineRenderer {
    pub vertecies: Vec<LineVertex>,
    pub vertecies_count: u32,

    pub max_lines: u32,
    pub vertex_buffer: Buffer,

    pub descriptor_pool: DescriptorPool,
    pub descriptor_layout: DescriptorSetLayout,
    pub descriptor_sets: Vec<DescriptorSet>,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: GraphicsPipeline,
}

#[derive(Debug, Clone, Copy)]
pub struct DebugLine {
    pub a: Vec3,
    pub b: Vec3,
    pub color: Vec4,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct LineVertex {
    pub pos: Vec3,
    pub color: [u8; 4],
}

impl DebugController {
    pub fn new(line_renderer: DebugLineRenderer, text_renderer: DebugTextRenderer) -> Result<Self> {
        Ok(DebugController {
            mode: DebugMode::OFF,
            line_renderer,
            text_renderer: vec![text_renderer],
            last_mode_change: Duration::ZERO,
        })
    }

    pub fn update(
        &mut self,
        controls: &Controls,
        delta_time: Duration,
        total_time: Duration,
    ) -> Result<()> {
        self.text_renderer[0].update(delta_time);

        if controls.f2 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::WFC {
                DebugMode::WFC
            } else {
                DebugMode::OFF
            }
        }

        if self.mode != DebugMode::OFF {
            self.line_renderer.push_lines()?;
        } else {
            self.line_renderer.vertecies_count = 0;
        }

        Ok(())
    }

    pub fn add_lines(&mut self, lines: Vec<DebugLine>) -> Result<()> {
        for line in lines.into_iter() {
            self.line_renderer
                .vertecies
                .push(LineVertex::new(line.a - vec3(0.5, 0.5, 0.5), line.color));
            self.line_renderer
                .vertecies
                .push(LineVertex::new(line.b - vec3(0.5, 0.5, 0.5), line.color));
        }

        Ok(())
    }

    pub fn add_cube(&mut self, min: Vec3, max: Vec3, color: Vec4) -> Result<()> {
        let mut lines = Vec::new();
        lines.push(DebugLine::new(
            vec3(min.x, min.y, min.z),
            vec3(max.x, min.y, min.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(min.x, max.y, min.z),
            vec3(max.x, max.y, min.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(min.x, min.y, max.z),
            vec3(max.x, min.y, max.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(min.x, max.y, max.z),
            vec3(max.x, max.y, max.z),
            color,
        ));

        lines.push(DebugLine::new(
            vec3(min.x, min.y, min.z),
            vec3(min.x, max.y, min.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(max.x, min.y, min.z),
            vec3(max.x, max.y, min.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(min.x, min.y, max.z),
            vec3(min.x, max.y, max.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(max.x, min.y, max.z),
            vec3(max.x, max.y, max.z),
            color,
        ));

        lines.push(DebugLine::new(
            vec3(min.x, min.y, min.z),
            vec3(min.x, min.y, max.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(max.x, min.y, min.z),
            vec3(max.x, min.y, max.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(min.x, max.y, min.z),
            vec3(min.x, max.y, max.z),
            color,
        ));
        lines.push(DebugLine::new(
            vec3(max.x, max.y, min.z),
            vec3(max.x, max.y, max.z),
            color,
        ));

        self.add_lines(lines)?;
        Ok(())
    }

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        image_index: usize,
        camera: &Camera,
        extent: Extent2D,
    ) -> Result<()> {
        self.text_renderer[0].render(buffer, camera, extent)?;

        if self.mode == DebugMode::OFF {
            return Ok(());
        }

        self.line_renderer.render(buffer, image_index);

        Ok(())
    }
}

impl DebugLineRenderer {
    pub fn new(
        max_lines: u32,
        context: &Context,
        images_len: u32,
        color_attachment_format: vk::Format,
        depth_attachment_format: vk::Format,
        renderer: &Renderer,
    ) -> Result<Self> {
        let vertex_buffer = context.create_buffer(
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (max_lines * 2 * size_of::<LineVertex>() as u32) as _,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            images_len * 1,
            &[vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: images_len,
            }],
        )?;

        let descriptor_layout =
            context.create_descriptor_set_layout(&[vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::ALL_GRAPHICS,
                ..Default::default()
            }])?;

        let mut descriptor_sets = Vec::new();
        for _ in 0..images_len {
            let render_descriptor_set = descriptor_pool.allocate_set(&descriptor_layout)?;

            render_descriptor_set.update(&[WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::UniformBuffer {
                    buffer: &renderer.render_buffer,
                },
            }]);
            descriptor_sets.push(render_descriptor_set);
        }

        let pipeline_layout = context.create_pipeline_layout(&[&descriptor_layout], &[])?;

        let pipeline = context.create_graphics_pipeline::<LineVertex>(
            &pipeline_layout,
            GraphicsPipelineCreateInfo {
                shaders: &[
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../shaders/debug.vert.spv")[..],
                        stage: vk::ShaderStageFlags::VERTEX,
                    },
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../shaders/debug.frag.spv")[..],
                        stage: vk::ShaderStageFlags::FRAGMENT,
                    },
                ],
                primitive_topology: vk::PrimitiveTopology::LINE_LIST,
                extent: None,
                color_attachment_format,
                color_attachment_blend: Some(
                    vk::PipelineColorBlendAttachmentState::builder()
                        .color_write_mask(vk::ColorComponentFlags::RGBA)
                        .blend_enable(true)
                        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                        .color_blend_op(vk::BlendOp::ADD)
                        .src_alpha_blend_factor(vk::BlendFactor::ONE)
                        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                        .alpha_blend_op(vk::BlendOp::ADD)
                        .build(),
                ),
                depth_attachment_format: Some(depth_attachment_format),
                dynamic_states: Some(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]),
            },
        )?;

        Ok(DebugLineRenderer {
            vertecies: Vec::new(),
            vertecies_count: 0,
            max_lines,
            vertex_buffer,
            descriptor_pool,
            descriptor_layout,
            descriptor_sets,
            pipeline_layout,
            pipeline,
        })
    }

    fn push_lines(&mut self) -> Result<()> {
        if self.vertecies.is_empty() {
            return Ok(());
        }

        self.vertex_buffer.copy_data_to_buffer_complex(
            &self.vertecies,
            0,
            align_of::<LineVertex>(),
        )?;
        self.vertecies_count = self.vertecies.len() as u32;
        self.vertecies.clear();

        Ok(())
    }

    fn render(&self, buffer: &CommandBuffer, image_index: usize) {
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.pipeline_layout,
            0,
            &[&self.descriptor_sets[image_index]],
        );

        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.draw(self.vertecies_count);
    }
}

impl DebugLine {
    pub fn new(a: Vec3, b: Vec3, color: Vec4) -> Self {
        DebugLine { a, b, color }
    }
}

impl LineVertex {
    pub fn new(pos: Vec3, color: Vec4) -> LineVertex {
        let color = [
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
            (color.w * 255.0) as u8,
        ];
        LineVertex { pos, color }
    }
}

impl octa_force::vulkan::Vertex for LineVertex {
    fn bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<LineVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: 0,
        }]
    }
}

pub struct DebugTextRenderer {
    renderer: imgui_rs_vulkan_renderer::Renderer,
    imgui: Option<imgui::SuspendedContext>,
    
}

pub struct DebugTextPlatformBackend {}
pub struct DebugTextRenderBackend {}

impl DebugTextRenderer {
    pub fn new(
        context: &Context,
        command_pool: &CommandPool,
        format: vk::Format,
        in_flight_frames: usize,
        display_size: UVec2,
    ) -> Result<Self> {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let font_size = 13.0;
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: include_bytes!("../../../../assets/fonts/mplus-1p-regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.75,
                    ..FontConfig::default()
                }),
            },
        ]);
        imgui.io_mut().font_global_scale = 1.0;
        //imgui.io_mut().display_size = [display_size.x as f32, display_size.y as f32];
        imgui.io_mut().display_size = [1.0, 1.0];
        imgui.io_mut().config_flags |= ConfigFlags::DOCKING_ENABLE;
        imgui.io_mut().config_flags |= ConfigFlags::VIEWPORTS_ENABLE;
        imgui.io_mut().backend_flags |= BackendFlags::PLATFORM_HAS_VIEWPORTS;
        imgui.io_mut().backend_flags |= BackendFlags::RENDERER_HAS_VIEWPORTS;
        imgui
            .platform_io_mut()
            .monitors
            .replace_from_slice(&[PlatformMonitor {
                main_pos: [0.0, 0.0],
                main_size: [f32::MAX, f32::MAX],
                work_pos: [0.0, 0.0],
                work_size: [f32::MAX, f32::MAX],
                dpi_scale: 1.0,
            }]);
        imgui.main_viewport_mut().size = [1.0, 1.0];

        let mut platform_backend = DebugTextPlatformBackend {};
        let ptr: *mut DebugTextPlatformBackend = &mut platform_backend;
        let voidptr = ptr as *mut c_void;
        imgui.main_viewport_mut().platform_handle = voidptr;
        imgui.set_platform_backend(platform_backend);
        imgui.set_renderer_backend(DebugTextRenderBackend {});

        let renderer = imgui_rs_vulkan_renderer::Renderer::with_gpu_allocator(
            context.allocator.clone(),
            context.device.inner.clone(),
            context.graphics_queue.inner,
            command_pool.inner,
            DynamicRendering {
                color_attachment_format: format,
                depth_attachment_format: Some(vk::Format::D32_SFLOAT),
            },
            &mut imgui,
            Some(Options {
                in_flight_frames,
                render_3d: true,
                ..Default::default()
            }),
        )?;

        Ok(Self {
            renderer,
            imgui: Some(imgui.suspend()),
        })
    }

    pub fn update(&mut self, delta: Duration) {
        let mut imgui = self.imgui.take().unwrap().activate().unwrap();
        imgui.io_mut().update_delta_time(delta);
        self.imgui = Some(imgui.suspend());
    }

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        camera: &Camera,
        extent: Extent2D,
    ) -> Result<()> {
        let mut imgui = self.imgui.take().unwrap().activate().unwrap();

        let ui = imgui.new_frame();
        ui.window("Test")
            .position([1.0, 0.0], Condition::FirstUseEver)
            .size([300.0, 300.0], Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .build(|| {
                ui.text("Hello World");
            });

        ui.window("Test 2")
            .position([2.0, 0.0], Condition::FirstUseEver)
            .size([300.0, 300.0], Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .build(|| {
                ui.text("Hello World2");
            });

        imgui.render();
        imgui.update_platform_windows();
        imgui.render_platform_windows_default();

        for (i, viewport) in imgui.viewports().enumerate() {
            log::info!("Viewport {i}");

            let transform = Mat4::from_scale(vec3(1.0, -1.0, 1.0) * 0.01)
                * Mat4::from_rotation_x(f32::to_radians(0.0))
                * Mat4::from_translation(vec3(-(i as f32), 0.0, 0.0));

            let mat = camera.projection_matrix() * camera.view_matrix() * transform;

            let draw_data = viewport.draw_data();
            self.renderer
                .cmd_draw(buffer.inner, draw_data, extent, Some(&mat.to_cols_array()))?;
        }
        self.imgui = Some(imgui.suspend());
        Ok(())
    }
}

impl PlatformViewportBackend for DebugTextPlatformBackend {
    fn create_window(&mut self, _: &mut Viewport) {}
    fn destroy_window(&mut self, _: &mut Viewport) {}
    fn show_window(&mut self, _: &mut Viewport) {}
    fn set_window_pos(&mut self, _: &mut Viewport, _: [f32; 2]) {}
    fn get_window_pos(&mut self, _: &mut Viewport) -> [f32; 2] {
        [0.0, 0.0]
    }
    fn set_window_size(&mut self, _: &mut Viewport, _: [f32; 2]) {}
    fn get_window_size(&mut self, _: &mut Viewport) -> [f32; 2] {
        [0.0, 0.0]
    }
    fn set_window_focus(&mut self, _: &mut Viewport) {}
    fn get_window_focus(&mut self, _: &mut Viewport) -> bool {
        false
    }
    fn get_window_minimized(&mut self, _: &mut Viewport) -> bool {
        false
    }
    fn set_window_title(&mut self, _: &mut Viewport, _: &str) {}
    fn set_window_alpha(&mut self, _: &mut Viewport, _: f32) {}
    fn update_window(&mut self, _: &mut Viewport) {}
    fn render_window(&mut self, _: &mut Viewport) {}
    fn swap_buffers(&mut self, _: &mut Viewport) {}
    fn create_vk_surface(&mut self, _: &mut Viewport, _: u64, _: &mut u64) -> i32 {
        0
    }
}

impl RendererViewportBackend for DebugTextRenderBackend {
    fn create_window(&mut self, viewport: &mut Viewport) {}
    fn destroy_window(&mut self, viewport: &mut Viewport) {}
    fn set_window_size(&mut self, viewport: &mut Viewport, size: [f32; 2]) {}
    fn render_window(&mut self, viewport: &mut Viewport) {}
    fn swap_buffers(&mut self, viewport: &mut Viewport) {}
}
