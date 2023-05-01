use std::mem::size_of;

use app::{glam::{Vec2, ivec2}, vulkan::{Context, Buffer, utils::create_gpu_only_buffer_from_data, ash::vk::{self, Extent2D, ColorComponentFlags, BlendOp, BlendFactor}, PipelineLayout, GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, CommandBuffer, gpu_allocator::MemoryLocation, WriteDescriptorSet, WriteDescriptorSetKind, DescriptorPool, DescriptorSetLayout, DescriptorSet}, anyhow::Ok};
use app::anyhow::Result;

use crate::{camera::Camera, chunk::{CHUNK_PART_SIZE, math::{part_pos_to_world, part_corners}, transform::Transform}};

use super::part::RenderParticle;

pub struct ChunkRendererVulkan {
    vertex_buffer: Buffer,
    index_buffer: Buffer,

    pub part_ubo_data: Vec<PartUBO>,
    pub particle_buffer_data: Vec<RenderParticle>,

    pub render_ubo: Buffer,
    pub part_ubo: Buffer,
    pub particles_ssbo: Buffer,

    _descriptor_pool: DescriptorPool,
    _descriptor_layout: DescriptorSetLayout,
    descriptor_sets: Vec<DescriptorSet>,

    _pipeline_layout: PipelineLayout,
    pipeline: GraphicsPipeline,
}

impl ChunkRendererVulkan {
    pub fn new (
        context: &Context,
        color_attachment_format: vk::Format,
        images_len: u32,
        rendered_parts: usize,
    ) -> Result<Self> {
        let (vertices, indecies) = create_mesh();

        let vertex_buffer =
            create_gpu_only_buffer_from_data(context, vk::BufferUsageFlags::VERTEX_BUFFER, &vertices)?;

        let index_buffer =
            create_gpu_only_buffer_from_data(context, vk::BufferUsageFlags::INDEX_BUFFER, &indecies)?;


        let render_ubo = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<RenderUBO>() as _,
        )?;

        let part_ubo = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<PartUBO>() * rendered_parts) as _,
        )?;

        let particles_ssbo = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::CpuToGpu, 
            (size_of::<RenderParticle>() * rendered_parts * (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize) as _,
        )?;

        let mut part_ubo_data = Vec::new();
        let mut particle_buffer_data = Vec::new();
        for i in 0..rendered_parts {
            part_ubo_data.push(PartUBO::new(part_pos_to_world(Transform::default(), ivec2(i as i32, 0), Vec2::ZERO)));
            particle_buffer_data.extend_from_slice(&[RenderParticle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])
        }
        part_ubo.copy_data_to_buffer(&part_ubo_data)?;
        particles_ssbo.copy_data_to_buffer(&particle_buffer_data)?;


        let descriptor_pool = context.create_descriptor_pool(
            images_len * 3,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: images_len,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: images_len,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: images_len,
                },
            ],
        )?;

        let descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ])?;

        let mut descriptor_sets = Vec::new();
        for _ in 0..images_len {
            let render_descriptor_set =
            descriptor_pool.allocate_set(&descriptor_layout)?;

            render_descriptor_set.update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &render_ubo,
                    },
                },
                WriteDescriptorSet {
                    binding: 1,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &part_ubo,
                    },
                },
                WriteDescriptorSet {
                    binding: 2,
                    kind: WriteDescriptorSetKind::StorageBuffer { 
                        buffer: &particles_ssbo
                    },
                },
            ]);
            descriptor_sets.push(render_descriptor_set);
        }

        let pipeline_layout = context.create_pipeline_layout(&[&descriptor_layout])?;

        let color_blending = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(ColorComponentFlags::RGBA)
            .blend_enable(true)

            .src_color_blend_factor(BlendFactor::ONE)
            .dst_color_blend_factor(BlendFactor::ONE)
            .color_blend_op(BlendOp::MIN)

            .src_alpha_blend_factor(BlendFactor::ONE)
            .dst_alpha_blend_factor(BlendFactor::ZERO)
            .alpha_blend_op(BlendOp::ADD)
            .build();

        let pipeline = context.create_graphics_pipeline::<Vertex>(
            &pipeline_layout,
            GraphicsPipelineCreateInfo {
                shaders: &[
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../../shaders/chunk.vert.spv")[..],
                        stage: vk::ShaderStageFlags::VERTEX,
                    },
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../../shaders/chunk.frag.spv")[..],
                        stage: vk::ShaderStageFlags::FRAGMENT,
                    },
                ],
                primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
                extent: None,
                color_attachment_format,
                color_attachment_blend: Some(color_blending),
                dynamic_states: Some(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]),
            },
        )?;

        Ok(Self { 
            vertex_buffer: vertex_buffer, 
            index_buffer: index_buffer,

            part_ubo_data: part_ubo_data,
            particle_buffer_data: particle_buffer_data,

            render_ubo,
            part_ubo,
            particles_ssbo,

            _descriptor_pool: descriptor_pool,
            _descriptor_layout: descriptor_layout,
            descriptor_sets: descriptor_sets,

            _pipeline_layout: pipeline_layout,
            pipeline: pipeline, 
        })
    }

    pub fn render(
        &self, 
        buffer: &CommandBuffer,
        image_index: usize,
        extent: Extent2D,
    ){
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_vertex_buffer(&self.vertex_buffer);
        buffer.bind_index_buffer(&self.index_buffer);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self._pipeline_layout,
            0,
            &[&self.descriptor_sets[image_index]],
        );

        buffer.set_viewport(extent);
        buffer.set_scissor(extent);

        buffer.draw_indexed_instanced(6, 100);
    }
}

fn create_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let corners = part_corners();
    let vertices = vec![
        Vertex {
            position: corners[0],
        },
        Vertex {
            position: corners[1],
        },
        Vertex {
            position: corners[2],
        },
        Vertex {
            position: corners[3],
        },
    ];
    let indecies = vec![2, 1, 0, 1, 2, 3];

    (vertices, indecies)
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Vertex {
    position: Vec2,
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
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
        ]
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct RenderUBO {
    camera: Camera,
}

impl RenderUBO {
    pub fn new(camera: Camera) -> Self {
        Self { camera: camera }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct PartUBO {
    transform: Transform,
    fill: f32,
}

impl PartUBO {
    pub fn new (transform: Transform) -> Self {
        Self { 
            transform: transform,
            fill: 0.0
        }
    }
}
