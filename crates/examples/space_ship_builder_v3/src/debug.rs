use crate::builder::Builder;
use crate::math::{get_config, to_3d};
use crate::node::{Node, NodeController};
use crate::renderer::{PushConstant, RenderBuffer, Renderer, Vertex};
use crate::ship::{Ship, SHIP_TYPE_BASE, SHIP_TYPE_BUILD};
use app::anyhow::Result;
use app::glam::{vec3, vec4, BVec3, Vec3, Vec4};
use app::log;
use app::vulkan::ash::vk;
use app::vulkan::ash::vk::{ImageUsageFlags, ShaderStageFlags};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::push_constant::create_push_constant_range;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{
    gpu_allocator, Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSet,
    DescriptorSetLayout, GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo,
    PipelineLayout, WriteDescriptorSet, WriteDescriptorSetKind,
};
use std::mem::{align_of, size_of};

pub enum DebugMode {
    OFF,
    WFC,
}

pub struct DebugRenderer {
    pub vertex_count: u32,
    pub max_lines: u32,
    pub vertex_buffer: Buffer,

    pub descriptor_pool: DescriptorPool,
    pub descriptor_layout: DescriptorSetLayout,
    pub descriptor_sets: Vec<DescriptorSet>,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: GraphicsPipeline,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct LineVertex {
    pub pos: Vec3,
    pub color: [u8; 4],
}

impl DebugRenderer {
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

        Ok(DebugRenderer {
            vertex_count: 0,
            max_lines,
            vertex_buffer,
            descriptor_pool,
            descriptor_layout,
            descriptor_sets,
            pipeline_layout,
            pipeline,
        })
    }

    pub fn set_lines(&mut self) -> Result<()> {
        let mut vetecies = Vec::new();

        let color = vec4(1.0, 0.0, 0.0, 1.0);
        vetecies.push(LineVertex::new(vec3(0.0, 0.0, 0.0), color));
        vetecies.push(LineVertex::new(vec3(5.0, 5.0, 5.0), color));

        self.vertex_buffer
            .copy_data_to_buffer_complex(&vetecies, 0, align_of::<LineVertex>())?;
        self.vertex_count = vetecies.len() as u32;

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer, image_index: usize) {
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.pipeline_layout,
            0,
            &[&self.descriptor_sets[image_index]],
        );

        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.draw(self.vertex_count);
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

impl app::vulkan::Vertex for LineVertex {
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
