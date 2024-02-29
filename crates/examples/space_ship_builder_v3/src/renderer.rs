use crate::ship::{SHIP_TYPE_BASE, SHIP_TYPE_BUILD};
use crate::{
    builder::{self, Builder},
    node::{Node, NodeController},
    ship::{Ship, ShipType},
    ship_mesh::{self, ShipMesh},
};
use app::{
    anyhow::Result,
    camera::Camera,
    glam::{vec2, BVec3, Mat4, Vec2, Vec3},
    log,
    vulkan::{
        ash::vk::{self, ImageUsageFlags, PushConstantRange, ShaderStageFlags},
        gpu_allocator::{self, MemoryLocation},
        push_constant::create_push_constant_range,
        utils::create_gpu_only_buffer_from_data,
        Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSet, DescriptorSetLayout,
        GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, Image, ImageView,
        PipelineLayout, WriteDescriptorSet, WriteDescriptorSetKind,
    },
};
use std::mem::size_of;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vec3,
    pub data: u32, // node_index : 22, rot : 7, uv: 3
}

pub struct Renderer {
    pub render_buffer: Buffer,
    pub node_buffer: Buffer,
    pub mat_buffer: Buffer,

    pub descriptor_pool: DescriptorPool,
    pub descriptor_layout: DescriptorSetLayout,
    pub descriptor_sets: Vec<DescriptorSet>,

    pub pipeline_layout: PipelineLayout,
    pub pipeline: GraphicsPipeline,

    pub depth_attachment_format: vk::Format,
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
    pub fill: u32,
    pub screen_size: Vec2,
    pub fill_1: [u32; 10],
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct PushConstant {
    pub ship_type: ShipType,
}

impl Renderer {
    pub fn new(
        context: &Context,
        node_controller: &NodeController,
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

        let node_buffer_size = node_controller.nodes.len() * size_of::<Node>();
        log::info!(
            "Node Buffer Size: {:?} MB",
            node_buffer_size as f32 / 1000000.0
        );

        let node_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &node_controller.nodes,
        )?;

        let mat_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &node_controller.mats,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            images_len * 3,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: images_len,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
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

        let mut descriptor_sets = Vec::new();
        for _ in 0..images_len {
            let render_descriptor_set = descriptor_pool.allocate_set(&descriptor_layout)?;

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

        let push_constant_range =
            create_push_constant_range(ShaderStageFlags::FRAGMENT, size_of::<PushConstant>());

        let pipeline_layout =
            context.create_pipeline_layout(&[&descriptor_layout], &[push_constant_range])?;

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
            node_buffer,
            mat_buffer,

            descriptor_pool,
            descriptor_layout,
            descriptor_sets,
            pipeline_layout,
            pipeline,
            depth_attachment_format,
            depth_image,
            depth_image_view,
        })
    }

    pub fn update(&mut self, camera: &Camera, extent: vk::Extent2D) -> Result<()> {
        self.render_buffer.copy_data_to_buffer(&[RenderBuffer {
            proj_matrix: camera.projection_matrix(),
            view_matrix: camera.view_matrix(),
            dir: camera.direction,
            fill: 0,
            screen_size: vec2(extent.width as f32, extent.height as f32),
            fill_1: [0; 10],
        }])?;
        Ok(())
    }

    pub fn on_recreate_swapchain(&mut self, context: &Context, extent: vk::Extent2D) -> Result<()> {
        self.depth_image = context.create_image(
            ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            MemoryLocation::GpuOnly,
            self.depth_attachment_format,
            extent.width,
            extent.height,
        )?;

        self.depth_image_view = self.depth_image.create_image_view(true)?;

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer, image_index: usize, builder: &Builder) {
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.pipeline_layout,
            0,
            &[&self.descriptor_sets[image_index]],
        );

        self.render_ship_mesh(buffer, &builder.base_ship_mesh, SHIP_TYPE_BASE);
        self.render_ship_mesh(buffer, &builder.build_ship_mesh, SHIP_TYPE_BUILD);
    }

    pub fn render_ship_mesh(
        &self,
        buffer: &CommandBuffer,
        ship_mesh: &ShipMesh,
        ship_type: ShipType,
    ) {
        buffer.bind_vertex_buffer(&ship_mesh.vertex_buffer);
        buffer.bind_index_buffer(&ship_mesh.index_buffer);
        buffer.push_constant(
            &self.pipeline_layout,
            ShaderStageFlags::FRAGMENT,
            &PushConstant { ship_type },
        );
        buffer.draw_indexed(ship_mesh.index_counter);
    }
}

impl Vertex {
    pub fn new(pos: Vec3, uv: BVec3, node_id_bits: u32) -> Vertex {
        let data: u32 =
            (node_id_bits << 3) + ((uv.x as u32) << 2) + ((uv.y as u32) << 1) + (uv.z as u32);
        Vertex { pos, data }
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
        vec![vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: 0,
        }]
    }
}
