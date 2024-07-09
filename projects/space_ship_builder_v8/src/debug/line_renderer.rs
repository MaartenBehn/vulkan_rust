use crate::debug::DebugController;
use crate::render::parallax::renderer::ParallaxRenderer;
use octa_force::glam::{vec3, Vec3, Vec4};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSet, DescriptorSetLayout,
    GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, PipelineLayout,
    WriteDescriptorSet, WriteDescriptorSetKind,
};
use std::mem::{align_of, size_of};

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

impl DebugLineRenderer {
    pub fn new(
        max_lines: u32,
        context: &Context,
        images_len: u32,
        color_attachment_format: vk::Format,
        depth_attachment_format: vk::Format,
        renderer: &ParallaxRenderer,
    ) -> octa_force::anyhow::Result<Self> {
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
                        source: &include_bytes!("../../shaders/debug.vert.spv")[..],
                        stage: vk::ShaderStageFlags::VERTEX,
                    },
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../shaders/debug.frag.spv")[..],
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
                depth_attachment_format: depth_attachment_format,
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

    pub(crate) fn push_lines(&mut self) -> octa_force::anyhow::Result<()> {
        self.vertex_buffer.copy_data_to_buffer_complex(
            &self.vertecies,
            0,
            align_of::<LineVertex>(),
        )?;
        self.vertecies_count = self.vertecies.len() as u32;
        self.vertecies.clear();

        Ok(())
    }

    pub(crate) fn render(&self, buffer: &CommandBuffer, image_index: usize) {
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

impl DebugController {
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
