use crate::rules::Rules;
use crate::world::data::node::{Material, Node};
use crate::world::manager::CHUNK_SIZE;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::glam::{IVec2, Mat4, UVec2, Vec2, Vec3, Vec4};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::Format;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    Buffer, CommandBuffer, ComputePipeline, ComputePipelineCreateInfo, Context, DescriptorPool,
    DescriptorSet, DescriptorSetLayout, PipelineLayout, Swapchain, WriteDescriptorSet,
    WriteDescriptorSetKind,
};
use octa_force::ImageAndView;
use std::mem::size_of;

const RENDER_DISPATCH_GROUP_SIZE_X: u32 = 32;
const RENDER_DISPATCH_GROUP_SIZE_Y: u32 = 32;

const NUM_LOADED_CHUNKS: usize = 10;

pub struct ComputeRenderer {
    storage_images: Vec<ImageAndView>,
    render_buffer: Buffer,
    chunk_data_buffer: Buffer,
    chunk_node_ids_buffer: Buffer,
    node_buffer: Buffer,
    material_buffer: Buffer,

    descriptor_pool: DescriptorPool,
    descriptor_layout: DescriptorSetLayout,
    descriptor_sets: Vec<DescriptorSet>,
    pipeline_layout: PipelineLayout,
    pipeline: ComputePipeline,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct RenderBuffer {
    pub pos: Vec3,
    pub screen_size_x: f32,
    pub dir: Vec3,
    pub screen_size_y: f32,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct ChunkData {
    pub transform: Mat4,
    pub aabb: Vec4,
    pub chunk_size: u32,
    pub fill: [u32; 3],
}

impl ComputeRenderer {
    pub fn new(
        context: &Context,
        format: Format,
        res: UVec2,
        num_frames: usize,
        rules: &Rules,
    ) -> Result<ComputeRenderer> {
        let storage_images = context.create_storage_images(format, res, num_frames)?;

        let render_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<RenderBuffer>() as _,
        )?;

        let chunk_data_buffer_size = size_of::<ChunkData>() * NUM_LOADED_CHUNKS;
        log::info!(
            "Chunk Node ID Buffer Size: {:?} MB",
            chunk_data_buffer_size as f32 / 1000000.0
        );
        let chunk_data_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            chunk_data_buffer_size as _,
        )?;

        let chunk_node_ids_buffer_size = size_of::<u32>() * NUM_LOADED_CHUNKS * CHUNK_SIZE as usize;
        log::info!(
            "Chunk Node ID Buffer Size: {:?} MB",
            chunk_node_ids_buffer_size as f32 / 1000000.0
        );
        let chunk_node_ids_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            chunk_node_ids_buffer_size as _,
        )?;

        let node_buffer_size = rules.nodes.len() * size_of::<Node>();
        log::info!(
            "Node Buffer Size: {:?} MB",
            node_buffer_size as f32 / 1000000.0
        );
        let node_buffer = context
            .create_gpu_only_buffer_from_data(vk::BufferUsageFlags::UNIFORM_BUFFER, &rules.nodes)?;

        let material_buffer_size = rules.materials.len() * size_of::<Material>();
        log::info!(
            "Material Buffer Size: {:?} MB",
            material_buffer_size as f32 / 1000000.0
        );
        let material_buffer = context.create_gpu_only_buffer_from_data(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            &rules.materials,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            num_frames as u32,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_IMAGE,
                    descriptor_count: num_frames as u32,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: num_frames as u32 * 5,
                },
            ],
        )?;

        let descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 4,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 5,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
        ])?;

        let mut descriptor_sets = Vec::new();
        for i in 0..num_frames {
            let descriptor_set = descriptor_pool.allocate_set(&descriptor_layout)?;

            descriptor_set.update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::StorageImage {
                        layout: vk::ImageLayout::GENERAL,
                        view: &storage_images[i].view,
                    },
                },
                WriteDescriptorSet {
                    binding: 1,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &render_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 2,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &chunk_data_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 3,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &chunk_node_ids_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 4,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &node_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 5,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &material_buffer,
                    },
                },
            ]);
            descriptor_sets.push(descriptor_set);
        }

        let pipeline_layout = context.create_pipeline_layout(&[&descriptor_layout], &[])?;

        let pipeline = context.create_compute_pipeline(
            &pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../../../shaders/ray_caster.comp.spv")[..],
            },
        )?;

        Ok(ComputeRenderer {
            storage_images,
            render_buffer,
            chunk_data_buffer,
            chunk_node_ids_buffer,
            node_buffer,
            material_buffer,

            descriptor_pool,
            descriptor_layout,
            descriptor_sets,

            pipeline_layout,
            pipeline,
        })
    }

    pub fn update(&self, camera: &Camera, res: UVec2) -> Result<()> {
        self.render_buffer.copy_data_to_buffer(&[RenderBuffer::new(
            camera.position,
            camera.direction,
            res,
        )])?;
        Ok(())
    }

    pub fn render(
        &self,
        buffer: &CommandBuffer,
        frame_index: usize,
        swapchain: &Swapchain,
    ) -> Result<()> {
        buffer.bind_compute_pipeline(&self.pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.pipeline_layout,
            0,
            &[&self.descriptor_sets[frame_index]],
        );

        buffer.dispatch(
            (swapchain.size.x / RENDER_DISPATCH_GROUP_SIZE_X) + 1,
            (swapchain.size.y / RENDER_DISPATCH_GROUP_SIZE_Y) + 1,
            1,
        );

        buffer.swapchain_image_copy_from_compute_storage_image(
            &self.storage_images[frame_index].image,
            &swapchain.images_and_views[frame_index].image,
        )?;

        Ok(())
    }

    pub fn on_recreate_swapchain(
        &mut self,
        context: &Context,
        format: Format,
        num_frames: usize,
        res: UVec2,
    ) -> Result<()> {
        self.storage_images = context.create_storage_images(format, res, num_frames)?;

        for (i, descriotor_set) in self.descriptor_sets.iter().enumerate() {
            descriotor_set.update(&[WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::StorageImage {
                    layout: vk::ImageLayout::GENERAL,
                    view: &self.storage_images[i].view,
                },
            }]);
        }

        Ok(())
    }
}

impl RenderBuffer {
    pub fn new(pos: Vec3, dir: Vec3, res: UVec2) -> RenderBuffer {
        RenderBuffer {
            pos,
            dir,
            screen_size_x: res.x as f32,
            screen_size_y: res.y as f32,
        }
    }
}
