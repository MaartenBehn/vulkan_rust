use crate::render::mesh::Mesh;
use crate::rules::Rules;
use crate::world::data::node::Node;
use octa_force::glam::{IVec3, UVec2, UVec3};
use octa_force::vulkan::ash::vk::IndexType;
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

pub struct MeshRenderer {
    pub render_buffer: Buffer,
    pub node_buffer: Buffer,
    pub mat_buffer: Buffer,

    pub descriptor_pool: DescriptorPool,
    pub static_descriptor_layout: DescriptorSetLayout,
    pub chunk_descriptor_layout: DescriptorSetLayout,
    pub static_descriptor_sets: Vec<DescriptorSet>,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: GraphicsPipeline,

    pub depth_attachment_format: vk::Format,
    pub depth_image: Image,
    pub depth_image_view: ImageView,
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
    data: u32,
}

impl MeshRenderer {
    pub fn new(
        context: &Context,
        images_len: u32,
        color_attachment_format: vk::Format,
        depth_attachment_format: vk::Format,
        res: UVec2,
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
            images_len * 1000,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: images_len,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: images_len * 4,
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
        for _ in 0..images_len {
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
                        source: &include_bytes!("../../shaders/chunk.vert.spv")[..],
                        stage: vk::ShaderStageFlags::VERTEX,
                    },
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../shaders/chunk.frag.spv")[..],
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

        let depth_image = context.create_image(
            ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            MemoryLocation::GpuOnly,
            depth_attachment_format,
            res.x,
            res.y,
        )?;

        let depth_image_view = depth_image.create_image_view(true)?;

        Ok(MeshRenderer {
            render_buffer,
            node_buffer,
            mat_buffer,

            descriptor_pool,
            static_descriptor_layout,
            chunk_descriptor_layout,
            static_descriptor_sets: descriptor_sets,

            pipeline_layout,
            pipeline,
            depth_attachment_format,
            depth_image,
            depth_image_view,
        })
    }

    pub fn update(&mut self, camera: &Camera, res: UVec2) -> Result<()> {
        self.render_buffer.copy_data_to_buffer(&[RenderBuffer {
            proj_matrix: camera.projection_matrix(),
            view_matrix: camera.view_matrix(),
            dir: camera.direction,
            fill: 0,
            screen_size: res.as_vec2(),
            fill_1: [0; 10],
        }])?;
        Ok(())
    }

    pub fn on_recreate_swapchain(&mut self, context: &Context, res: UVec2) -> Result<()> {
        self.depth_image = context.create_image(
            ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            MemoryLocation::GpuOnly,
            self.depth_attachment_format,
            res.x,
            res.y,
        )?;

        self.depth_image_view = self.depth_image.create_image_view(true)?;

        Ok(())
    }

    pub fn render(
        &self,
        buffer: &CommandBuffer,
        image_index: usize,
        render_mode: RenderMode,
        mesh: &Mesh,
    ) {
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.pipeline_layout,
            0,
            &[&self.static_descriptor_sets[image_index]],
        );

        for chunk in mesh.chunks.iter() {
            if chunk.index_count == 0 {
                continue;
            }

            buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                &self.pipeline_layout,
                1,
                &[&chunk.descriptor_sets[image_index]],
            );

            buffer.bind_vertex_buffer(&chunk.vertex_buffer);
            buffer.bind_index_buffer_complex(&chunk.index_buffer, 0, IndexType::UINT16);

            buffer.push_constant(
                &self.pipeline_layout,
                ShaderStageFlags::FRAGMENT | ShaderStageFlags::VERTEX,
                &PushConstant::new(
                    chunk.pos / mesh.render_size,
                    mesh.size.x as u32,
                    (mesh.size.x / mesh.render_size.x) as u32,
                    render_mode,
                ),
            );

            buffer.draw_indexed(chunk.index_count as u32);
        }
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
    pub fn new(
        chunk_pos: IVec3,
        chunk_size: u32,
        chunk_scale_down: u32,
        render_mode: RenderMode,
    ) -> Self {
        // 8 Bit Chunk Pos X
        // 8 Bit Chunk Pos Y
        // 8 Bit Chunk Pos Z
        // 4 Bit Chunk Size
        // 3 Bit Chunk Scale Down
        // 1 Bit Render Mode

        let chunk_pos_x_bits = (chunk_pos.x + 128) as u32;
        let chunk_pos_y_bits = (chunk_pos.y + 128) as u32;
        let chunk_pos_z_bits = (chunk_pos.z + 128) as u32;
        let chunk_size_bits = chunk_size.trailing_zeros();
        let chunk_scale_bits = chunk_scale_down.trailing_zeros();

        let data = chunk_pos_x_bits
            + (chunk_pos_y_bits << 8)
            + (chunk_pos_z_bits << 16)
            + (chunk_size_bits << 24)
            + (chunk_scale_bits << 28)
            + (render_mode << 31);

        PushConstant { data }
    }
}
