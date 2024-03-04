use crate::math::to_3d;
use crate::ship::Ship;
use crate::ship_renderer::ShipRenderer;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::{vec3, Mat4, Vec3, Vec4};
use octa_force::gui::{GuiId, InWorldGui, InWorldGuiTransform};
use octa_force::imgui::Key::V;
use octa_force::imgui::{Condition, Ui};
use octa_force::log;
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::Extent2D;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSet, DescriptorSetLayout,
    GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, PipelineLayout,
    WriteDescriptorSet, WriteDescriptorSetKind,
};
use std::mem;
use std::mem::{align_of, size_of};
use std::time::Duration;

#[derive(PartialEq)]
pub enum DebugMode {
    OFF,
    WFC,
    WFCSkip,
}

const DEBUG_MODE_CHANGE_SPEED: Duration = Duration::from_millis(100);

pub struct DebugController {
    pub mode: DebugMode,
    pub line_renderer: DebugLineRenderer,
    pub text_renderer: DebugTextRenderer,

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

pub struct DebugTextRenderer {
    pub gui_id: GuiId,
    pub texts: Vec<DebugText>,
    pub render_texts: Vec<DebugText>,
}

#[derive(Debug, Clone)]
pub struct DebugText {
    pub lines: Vec<String>,
    pub pos: Vec3,
}

impl DebugController {
    pub fn new(line_renderer: DebugLineRenderer, text_renderer: DebugTextRenderer) -> Result<Self> {
        Ok(DebugController {
            mode: DebugMode::OFF,
            line_renderer,
            text_renderer,
            last_mode_change: Duration::ZERO,
        })
    }

    pub fn update(
        &mut self,
        controls: &Controls,
        total_time: Duration,
        debug_gui: &mut InWorldGui,
    ) -> Result<()> {
        if controls.f2 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::WFC {
                DebugMode::WFC
            } else {
                DebugMode::OFF
            }
        }
        if controls.f3 && (self.last_mode_change + DEBUG_MODE_CHANGE_SPEED) < total_time {
            self.last_mode_change = total_time;

            self.mode = if self.mode != DebugMode::WFCSkip {
                DebugMode::WFCSkip
            } else {
                DebugMode::OFF
            }
        }

        if self.mode != DebugMode::OFF {
            self.text_renderer.push_texts(debug_gui)?;
            self.line_renderer.push_lines()?;
        } else {
            self.line_renderer.vertecies_count = 0;
        }

        Ok(())
    }

    pub fn add_lines(&mut self, lines: Vec<DebugLine>) {
        for line in lines.into_iter() {
            self.line_renderer
                .vertecies
                .push(LineVertex::new(line.a, line.color));
            self.line_renderer
                .vertecies
                .push(LineVertex::new(line.b, line.color));
        }
    }

    pub fn add_cube(&mut self, min: Vec3, max: Vec3, color: Vec4) {
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

        self.add_lines(lines);
    }

    pub fn add_text(&mut self, lines: Vec<String>, pos: Vec3) {
        self.text_renderer.texts.push(DebugText { lines, pos });
    }

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        image_index: usize,
        camera: &Camera,
        extent: Extent2D,
        debug_gui: &mut InWorldGui,
    ) -> Result<()> {
        if self.mode == DebugMode::OFF {
            return Ok(());
        }

        self.text_renderer
            .render(buffer, camera, extent, debug_gui)?;
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
        renderer: &ShipRenderer,
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

impl DebugTextRenderer {
    pub fn new(gui_id: GuiId) -> Self {
        Self {
            gui_id,
            texts: Vec::new(),
            render_texts: Vec::new(),
        }
    }

    fn push_texts(&mut self, debug_gui: &mut InWorldGui) -> Result<()> {
        if self.texts.is_empty() {
            return Ok(());
        }

        self.render_texts.clear();
        self.render_texts.append(&mut self.texts);
        self.texts.clear();

        let mut transforms = Vec::new();
        for text in self.render_texts.iter() {
            let mut t = InWorldGuiTransform::default();
            t.pos = text.pos;

            transforms.push(t);
        }
        debug_gui.set_transfrom(&transforms);

        Ok(())
    }

    fn render(
        &mut self,
        buffer: &CommandBuffer,
        camera: &Camera,
        extent: Extent2D,
        debug_gui: &mut InWorldGui,
    ) -> Result<()> {
        if self.render_texts.is_empty() {
            return Ok(());
        }

        debug_gui.draw(buffer, extent, camera, |ui: &Ui| {
            for (i, text) in self.render_texts.iter().enumerate() {
                let y = 10.0 + text.lines.len() as f32 * 20.0;

                ui.window(format!("{i}"))
                    .position([i as f32, 0.0], Condition::Always)
                    .size([120.0, y], Condition::Always)
                    .resizable(false)
                    .movable(false)
                    .no_decoration()
                    .no_nav()
                    .no_inputs()
                    .build(|| {
                        for s in text.lines.iter() {
                            ui.text(s);
                        }
                    });
            }
            Ok(())
        })?;

        Ok(())
    }
}
