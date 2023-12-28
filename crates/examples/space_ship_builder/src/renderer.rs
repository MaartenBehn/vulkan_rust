use std::mem::size_of;

use app::{
    anyhow::Result,
    glam::{Mat4, Vec3, Vec4},
    vulkan::{
        ash::vk::{self, Extent2D, Format, ImageUsageFlags},
        gpu_allocator::{self, MemoryLocation},
        utils::create_gpu_only_buffer_from_data,
        Buffer, Context, DescriptorPool, DescriptorSet, DescriptorSetLayout, GraphicsPipeline,
        GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, Image, ImageView, PipelineLayout,
        WriteDescriptorSet, WriteDescriptorSetKind,
    },
};

use crate::ship_mesh::ShipMesh;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Vec4,
    pub uv: Vec3,
}

pub struct Renderer {
    pub render_buffer: Buffer,

    pub descriptor_pool: DescriptorPool,
    pub descriptor_layout: DescriptorSetLayout,
    pub descriptor_sets: Vec<DescriptorSet>,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: GraphicsPipeline,

    pub depth_image: Image,
    pub depth_image_view: ImageView,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct RenderBuffer {
    pub proj_matrix: Mat4,
    pub view_matrix: Mat4,
    pub dir: Vec3,
    pub fill: [u32; 13],
}

impl Renderer {
    pub fn new(
        context: &Context,
        images_len: u32,
        color_attachment_format: vk::Format,
        depth_attachment_format: vk::Format,
        extent: vk::Extent2D,
    ) -> Result<Self> {
        let render_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<RenderBuffer>() as _,
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
                    buffer: &render_buffer,
                },
            }]);
            descriptor_sets.push(render_descriptor_set);
        }

        let pipeline_layout = context.create_pipeline_layout(&[&descriptor_layout])?;

        let pipeline = context.create_graphics_pipeline::<Vertex>(
            &pipeline_layout,
            GraphicsPipelineCreateInfo {
                shaders: &[
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../shaders/shader.vert.spv")[..],
                        stage: vk::ShaderStageFlags::VERTEX,
                    },
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../shaders/shader.frag.spv")[..],
                        stage: vk::ShaderStageFlags::FRAGMENT,
                    },
                ],
                primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
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

        let depth_image = context.create_image(
            ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            gpu_allocator::MemoryLocation::GpuOnly,
            depth_attachment_format,
            extent.width,
            extent.height,
        )?;

        let depth_image_view = depth_image.create_image_view(true)?;

        Ok(Renderer {
            render_buffer,
            descriptor_pool,
            descriptor_layout,
            descriptor_sets,
            pipeline_layout,
            pipeline,
            depth_image,
            depth_image_view,
        })
    }
}

impl Vertex {
    pub fn new(pos: Vec3, color: Vec4, uv: Vec3) -> Vertex {
        Vertex {
            position: pos,
            color: color,
            uv: uv,
        }
    }
}

impl app::vulkan::Vertex for Vertex {
    fn bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 16,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 32,
            },
        ]
    }
}
