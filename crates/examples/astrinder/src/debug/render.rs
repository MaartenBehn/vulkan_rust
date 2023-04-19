use std::mem::size_of;

use app::{glam::{Vec2, vec2, ivec2}, vulkan::{Context, Buffer, utils::create_gpu_only_buffer_from_data, ash::vk::{self, Extent2D, ColorComponentFlags, BlendOp, BlendFactor}, PipelineLayout, GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, CommandBuffer, gpu_allocator::MemoryLocation, WriteDescriptorSet, WriteDescriptorSetKind, DescriptorPool, DescriptorSetLayout, DescriptorSet}, anyhow::Ok};
use app::anyhow::Result;
use cgmath::Point2;

use crate::{camera::{Camera, self}, chunk::{ChunkController, ChunkPart, Chunk, transform::Transform, math::part_pos_to_world, render::RenderUBO}};

pub struct DebugRenderer {
    max_lines: usize,

    vertex_buffer: Buffer,
    _render_ubo: Buffer,

    _descriptor_pool: DescriptorPool,
    _descriptor_layout: DescriptorSetLayout,
    descriptor_sets: Vec<DescriptorSet>,

    _pipeline_layout: PipelineLayout,
    pipeline: GraphicsPipeline,
}

impl DebugRenderer {
    pub fn new (
        context: &Context,
        color_attachment_format: vk::Format,
        images_len: u32,
        max_lines: usize,
    ) -> Result<Self> {
       
        let vertex_buffer = context.create_buffer(
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<Vertex>() * max_lines * 2) as _,
        )?;

        let render_ubo = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<RenderUBO>() as _,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            images_len * 1,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
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
            ]);
            descriptor_sets.push(render_descriptor_set);
        }

        let pipeline_layout = context.create_pipeline_layout(&[&descriptor_layout])?;

        let color_blending = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(ColorComponentFlags::RGBA)
            .blend_enable(false)

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
                color_attachment_blend: Some(color_blending),
                dynamic_states: Some(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]),
            },
        )?;

        Ok(Self { 
            max_lines: max_lines,
            vertex_buffer: vertex_buffer,
            _render_ubo: render_ubo, 

            _descriptor_pool: descriptor_pool,
            _descriptor_layout: descriptor_layout,
            descriptor_sets: descriptor_sets,

            _pipeline_layout: pipeline_layout,
            pipeline: pipeline, 
        })
    }

    pub fn update (
        &mut self, 
        camera: &Camera,
        chunk_controller: &ChunkController,
    ) -> Result<()>{

        let mut vertex = Vec::new();
        let mut counter = 0;

        let push_line = |a: Point2<f32>, b: Point2<f32>, vertex: &mut Vec<f32>, part_transform: Transform, counter: &mut usize| {
            let pos0 = vec2(a.x, a.y);
            let pos1 = vec2(b.x, b.y);

            let angle_vec = Vec2::from_angle(part_transform.rot);
            let r_pos0 = Vec2::rotate(angle_vec, pos0);
            let r_pos1 = Vec2::rotate(angle_vec, pos1);

            vertex.push(r_pos0.x + part_transform.pos.x);
            vertex.push(r_pos0.y + part_transform.pos.y);
            vertex.push(r_pos1.x + part_transform.pos.x);
            vertex.push(r_pos1.y + part_transform.pos.y);

            *counter += 4;
        };

        for chunk in chunk_controller.chunks.iter() {
            for part in chunk.parts.iter() {
                let part_transform = part_pos_to_world(chunk.transform, part.pos, chunk.render_to_transform);

                for collider in part.colliders.iter() {
                    push_line(collider.vertices[0], collider.vertices[1], &mut vertex, part_transform, &mut counter);
                    push_line(collider.vertices[1], collider.vertices[2], &mut vertex, part_transform, &mut counter);
                    push_line(collider.vertices[2], collider.vertices[0], &mut vertex, part_transform, &mut counter);
                }
            }
        }

        for _ in 0..(self.max_lines * 2 - counter) {
            vertex.push(0.0);
        }

        self.vertex_buffer.copy_data_to_buffer(&vertex)?;

        self._render_ubo.copy_data_to_buffer(&[RenderUBO::new(camera.to_owned())])?;
        
        Ok(())
    }

    pub fn render(
        &self, 
        buffer: &CommandBuffer,
        image_index: usize,
        extent: Extent2D,
    ){
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_vertex_buffer(&self.vertex_buffer);

        buffer.set_viewport(extent);
        buffer.set_scissor(extent);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self._pipeline_layout,
            0,
            &[&self.descriptor_sets[image_index]],
        );

        buffer.draw(self.max_lines as u32);
    }
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