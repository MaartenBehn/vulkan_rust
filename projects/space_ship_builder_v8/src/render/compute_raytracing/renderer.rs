use crate::math::aabb::get_aabb_of_transformed_cube;
use crate::render::compute_raytracing::compute_raytracing_data::ComputeRaytracingData;
use crate::render::parallax::node_parallax_mesh::NodeParallaxMesh;
use crate::rules::Rules;
use crate::world::block_object::{BlockObject, ChunkIndex};
use crate::world::data::node::{Material, Node};
use crate::world::manager::CHUNK_SIZE;
use log::error;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::egui::UserAttentionType;
use octa_force::glam::{IVec2, IVec3, Mat4, UVec2, Vec2, Vec3, Vec4};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::Format;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    Buffer, CommandBuffer, ComputePipeline, ComputePipelineCreateInfo, Context, DescriptorPool,
    DescriptorSet, DescriptorSetLayout, PipelineLayout, Swapchain, WriteDescriptorSet,
    WriteDescriptorSetKind,
};
use octa_force::ImageAndView;
use std::mem::{align_of, size_of};

const RENDER_DISPATCH_GROUP_SIZE_X: u32 = 32;
const RENDER_DISPATCH_GROUP_SIZE_Y: u32 = 32;

const NUM_LOADED_CHUNKS: usize = 100;

pub struct ComputeRaytracingRenderer {
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

    free_chunks: Vec<usize>,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct RenderBuffer {
    pub pos: Vec3,
    pub screen_size_x: f32,
    pub dir: Vec3,
    pub screen_size_y: f32,
    pub num_chunks: u32,
    pub fill: [u32; 3],
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct ChunkData {
    pub transform: Mat4,
    pub aabb_min: Vec3,
    pub chunk_size: u32,
    pub aabb_max: Vec3,
    pub fill: u32,
}

impl ComputeRaytracingRenderer {
    pub fn new(
        context: &Context,
        res: UVec2,
        num_frames: usize,
        rules: &Rules,
    ) -> Result<ComputeRaytracingRenderer> {
        let storage_images = context.create_storage_images(res, num_frames)?;

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
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            chunk_data_buffer_size as _,
        )?;

        let chunk_node_ids_buffer_size =
            size_of::<u32>() * NUM_LOADED_CHUNKS * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
        log::info!(
            "Chunk Node ID Buffer Size: {:?} MB",
            chunk_node_ids_buffer_size as f32 / 1000000.0
        );
        let chunk_node_ids_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            chunk_node_ids_buffer_size as _,
        )?;

        let node_buffer_size = rules.nodes.len() * size_of::<Node>();
        log::info!(
            "Node Buffer Size: {:?} MB",
            node_buffer_size as f32 / 1000000.0
        );
        let node_buffer = context
            .create_gpu_only_buffer_from_data(vk::BufferUsageFlags::STORAGE_BUFFER, &rules.nodes)?;

        let material_buffer_size = rules.materials.len() * size_of::<Material>();
        log::info!(
            "Material Buffer Size: {:?} MB",
            material_buffer_size as f32 / 1000000.0
        );
        let material_buffer = context.create_gpu_only_buffer_from_data(
            vk::BufferUsageFlags::STORAGE_BUFFER,
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
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: num_frames as u32 * 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: num_frames as u32 * 4,
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
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 4,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 5,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
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
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &chunk_data_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 3,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &chunk_node_ids_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 4,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &node_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 5,
                    kind: WriteDescriptorSetKind::StorageBuffer {
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

        let free_chunks = (0..NUM_LOADED_CHUNKS).rev().into_iter().collect();

        Ok(ComputeRaytracingRenderer {
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

            free_chunks,
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

    pub fn update_object(
        &mut self,
        object: &mut BlockObject,
        changed_chunks: Vec<ChunkIndex>,
    ) -> Result<()> {
        for chunk_index in changed_chunks {
            let chunk = &mut object.chunks[chunk_index];

            if chunk.compute_raytracing_data.is_none() {
                if self.free_chunks.is_empty() {
                    error!("Compute Raytracer has no free chunk slot.");
                    return Ok(());
                }

                chunk.compute_raytracing_data =
                    Some(ComputeRaytracingData::new(self.free_chunks.pop().unwrap()))
            }

            let index = chunk.compute_raytracing_data.as_ref().unwrap().index;

            if index != 0 {
                continue;
            }

            let chunk_data =
                ChunkData::new(object.transform, chunk.pos, object.nodes_per_chunk.x as u32);
            let align = align_of::<ChunkData>();
            self.chunk_data_buffer
                .copy_data_to_buffer_complex(&[chunk_data], index, align)?;

            let nodes_align = align_of::<u32>();
            self.chunk_node_ids_buffer.copy_data_to_buffer_complex(
                &chunk.node_id_bits,
                index,
                nodes_align,
            )?;
        }

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
        num_frames: usize,
        res: UVec2,
    ) -> Result<()> {
        self.storage_images = context.create_storage_images(res, num_frames)?;

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
            num_chunks: NUM_LOADED_CHUNKS as u32,
            fill: [0; 3],
        }
    }
}

impl ChunkData {
    pub fn new(object_transform: Mat4, chunk_pos: IVec3, nodes_per_chunk: u32) -> ChunkData {
        let transform = object_transform.mul_mat4(&Mat4::from_translation(chunk_pos.as_vec3()));
        let (aabb_min, aabb_max) =
            get_aabb_of_transformed_cube(transform, Vec3::ONE * nodes_per_chunk as f32);

        ChunkData {
            transform,
            aabb_min,
            aabb_max,
            chunk_size: nodes_per_chunk,
            fill: 0,
        }
    }
}
