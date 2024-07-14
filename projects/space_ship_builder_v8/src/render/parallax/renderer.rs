use crate::render::parallax::node_parallax_mesh::NodeParallaxMesh;
use crate::rules::Rules;
use crate::world::block_object::{BlockChunk, BlockObject, ChunkIndex};
use crate::world::data::node::Node;
use block_mesh::ilattice::glam::{vec4, Vec4};
use octa_force::glam::{IVec3, UVec2, UVec3};
use octa_force::vulkan::ash::vk::IndexType;
use octa_force::vulkan::Swapchain;
use octa_force::{
    anyhow::Result,
    camera::Camera,
    glam::{Mat4, Vec2, Vec3},
    vulkan::{
        ash::vk::{self, ImageUsageFlags, ShaderStageFlags},
        gpu_allocator::MemoryLocation,
        push_constant::create_push_constant_range,
        Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSet, DescriptorSetLayout,
        GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, Image, ImageView,
        PipelineLayout, WriteDescriptorSet, WriteDescriptorSetKind,
    },
};
use std::mem::size_of;

type RenderMode = u32;
pub const RENDER_MODE_BASE: RenderMode = 0;
pub const RENDER_MODE_BUILD: RenderMode = 1;

pub struct ParallaxRenderer {
    pub render_buffer: Buffer,
    pub node_buffer: Buffer,
    pub mat_buffer: Buffer,

    pub descriptor_pool: DescriptorPool,
    pub static_descriptor_layout: DescriptorSetLayout,
    pub chunk_descriptor_layout: DescriptorSetLayout,
    pub static_descriptor_sets: Vec<DescriptorSet>,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: GraphicsPipeline,

    pub to_drop_buffers: Vec<Vec<Buffer>>,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct Vertex {
    pub data: u32,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct RenderBuffer {
    pub proj_matrix: Mat4,
    pub view_matrix: Mat4,
    pub dir: Vec3,
    pub fill: u32,
    pub screen_size: Vec2,
    pub fill_1: [u32; 10],
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct PushConstant {
    transform: Mat4,
    data: u32,
}

impl ParallaxRenderer {
    pub fn new(
        context: &Context,
        num_frames: usize,
        color_attachment_format: vk::Format,
        depth_attachment_format: vk::Format,
        rules: &Rules,
    ) -> Result<Self> {
        let render_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<RenderBuffer>() as _,
        )?;

        let node_buffer_size = rules.nodes.len() * size_of::<Node>();
        log::info!(
            "Node Buffer Size: {:?} MB",
            node_buffer_size as f32 / 1000000.0
        );

        let node_buffer = context
            .create_gpu_only_buffer_from_data(vk::BufferUsageFlags::STORAGE_BUFFER, &rules.nodes)?;

        let mat_buffer = context.create_gpu_only_buffer_from_data(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &rules.materials,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            (num_frames * 1000) as u32,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: num_frames as u32,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: (num_frames * 4) as u32,
                },
            ],
        )?;

        let static_descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::ALL_GRAPHICS,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ])?;

        let chunk_descriptor_layout =
            context.create_descriptor_set_layout(&[vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            }])?;

        let mut descriptor_sets = Vec::new();
        for _ in 0..num_frames {
            let render_descriptor_set = descriptor_pool.allocate_set(&static_descriptor_layout)?;

            render_descriptor_set.update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &render_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 1,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &node_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 2,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &mat_buffer,
                    },
                },
            ]);
            descriptor_sets.push(render_descriptor_set);
        }

        let push_constant_range = create_push_constant_range(
            ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
            size_of::<PushConstant>(),
        );

        let pipeline_layout = context.create_pipeline_layout(
            &[&static_descriptor_layout, &chunk_descriptor_layout],
            &[push_constant_range],
        )?;

        let pipeline = context.create_graphics_pipeline::<Vertex>(
            &pipeline_layout,
            GraphicsPipelineCreateInfo {
                shaders: &[
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../../shaders/parallax.vert.spv")[..],
                        stage: vk::ShaderStageFlags::VERTEX,
                    },
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../../shaders/parallax.frag.spv")[..],
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
                depth_attachment_format: depth_attachment_format,
                dynamic_states: Some(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]),
            },
        )?;

        let mut to_drop_buffers = Vec::new();
        for _ in 0..num_frames {
            to_drop_buffers.push(vec![])
        }

        Ok(ParallaxRenderer {
            render_buffer,
            node_buffer,
            mat_buffer,

            descriptor_pool,
            static_descriptor_layout,
            chunk_descriptor_layout,
            static_descriptor_sets: descriptor_sets,

            pipeline_layout,
            pipeline,

            to_drop_buffers,
        })
    }

    pub fn update(&mut self, camera: &Camera, res: UVec2, frame_index: usize) -> Result<()> {
        self.render_buffer.copy_data_to_buffer(&[RenderBuffer {
            proj_matrix: camera.projection_matrix(),
            view_matrix: camera.view_matrix(),
            dir: camera.direction,
            fill: 0,
            screen_size: res.as_vec2(),
            fill_1: [0; 10],
        }])?;

        self.to_drop_buffers[frame_index].clear();
        Ok(())
    }

    pub fn update_object(
        &mut self,
        object: &mut BlockObject,
        changed_chunks: Vec<ChunkIndex>,
        context: &Context,
        frame_index: usize,
        num_frames: usize,
    ) -> Result<()> {
        for chunk_index in changed_chunks {
            let chunk = &mut object.chunks[chunk_index];

            if chunk.parallax_data.is_none() {
                chunk.parallax_data = Some(NodeParallaxMesh::new(
                    chunk.pos,
                    object.nodes_per_chunk.x as u32,
                    object.nodes_length,
                    num_frames,
                    context,
                    &self.chunk_descriptor_layout,
                    &self.descriptor_pool,
                )?);
            }

            chunk
                .parallax_data
                .as_mut()
                .unwrap()
                .update(
                    object.nodes_per_chunk,
                    &chunk.node_id_bits,
                    &chunk.render_nodes,
                    context,
                    &mut self.to_drop_buffers[frame_index],
                )
                .unwrap();
        }

        Ok(())
    }

    pub fn begin_render(
        &self,
        buffer: &CommandBuffer,
        frame_index: usize,
        swapchain: &Swapchain,
    ) -> Result<()> {
        buffer.swapchain_image_render_barrier(&swapchain.images_and_views[frame_index].image)?;
        buffer.begin_rendering(
            &swapchain.images_and_views[frame_index].view,
            &swapchain.depht_images_and_views[frame_index].view,
            swapchain.size,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );

        buffer.set_viewport_size(swapchain.size.as_vec2());
        buffer.set_scissor_size(swapchain.size.as_vec2());

        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.pipeline_layout,
            0,
            &[&self.static_descriptor_sets[frame_index]],
        );

        Ok(())
    }

    pub fn render_data(
        &self,
        buffer: &CommandBuffer,
        frame_index: usize,
        data: &NodeParallaxMesh,
        base_transform: &Mat4,
    ) {
        if data.index_count == 0 {
            return;
        }

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.pipeline_layout,
            1,
            &[&data.descriptor_sets[frame_index]],
        );

        buffer.bind_vertex_buffer(&data.vertex_buffer);
        buffer.bind_index_buffer_complex(&data.index_buffer, 0, IndexType::UINT16);

        buffer.push_constant(
            &self.pipeline_layout,
            ShaderStageFlags::FRAGMENT | ShaderStageFlags::VERTEX,
            &PushConstant::new(base_transform, data.pos, data.size),
        );

        buffer.draw_indexed(data.index_count as u32);
    }

    pub fn end_rendering(&self, buffer: &CommandBuffer) {
        buffer.end_rendering()
    }

    pub fn on_rules_changed(
        &mut self,
        rules: &Rules,
        context: &Context,
        num_frames: usize,
    ) -> Result<()> {
        let node_buffer_size = rules.nodes.len() * size_of::<Node>();
        log::info!(
            "Node Buffer Size: {:?} MB",
            node_buffer_size as f32 / 1000000.0
        );

        self.node_buffer = context
            .create_gpu_only_buffer_from_data(vk::BufferUsageFlags::STORAGE_BUFFER, &rules.nodes)?;

        self.mat_buffer = context.create_gpu_only_buffer_from_data(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &rules.materials,
        )?;

        for i in 0..num_frames {
            self.static_descriptor_sets[i].update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &self.render_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 1,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &self.node_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 2,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &self.mat_buffer,
                    },
                },
            ]);
        }

        Ok(())
    }
}

impl Vertex {
    pub fn new(pos: UVec3, normal: IVec3) -> Vertex {
        let data = (pos.x & 0b111111111)
            + ((pos.y & 0b111111111) << 9)
            + ((pos.z & 0b111111111) << 18)
            + (((normal.x == 1) as u32) << 27)
            + (((normal.y == 1) as u32) << 28)
            + (((normal.z == 1) as u32) << 29);
        Vertex { data }
    }
}

impl octa_force::vulkan::Vertex for Vertex {
    fn bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32_UINT,
            offset: 0,
        }]
    }
}

impl PushConstant {
    pub fn new(transform: &Mat4, offset: IVec3, chunk_size: u32) -> Self {
        let chunk_size_bits = chunk_size.trailing_zeros();
        let data = chunk_size_bits;

        PushConstant {
            transform: transform.mul_mat4(&Mat4::from_translation(offset.as_vec3())),
            data,
        }
    }
}
