use std::mem::size_of;

use app::anyhow::{Ok, Result};
use app::glam::Vec3;
use app::vulkan::{
    ash::vk, gpu_allocator::MemoryLocation, Buffer, ComputePipeline, Context, DescriptorPool,
    DescriptorSet, DescriptorSetLayout, PipelineLayout,
};
use app::vulkan::{
    CommandBuffer, ComputePipelineCreateInfo, WriteDescriptorSet, WriteDescriptorSetKind,
};
use app::{BaseApp, ImageAndView};

use crate::RayCaster;

const RENDER_DISPATCH_GROUP_SIZE_X: u32 = 32;
const RENDER_DISPATCH_GROUP_SIZE_Y: u32 = 32;

pub struct Renderer {
    pub ubo_buffer: Buffer,
    pub descriptor_pool: DescriptorPool,
    pub descriptor_layout: DescriptorSetLayout,
    pub descriptor_sets: Vec<DescriptorSet>,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: ComputePipeline,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
#[repr(C)]
pub struct RenderBuffer {
    pub screen_size: [f32; 2],
    pub mode: u32,
    pub debug_scale: u32,

    pub pos: Vec3,
    pub step_to_root: u32,

    pub dir: Vec3,
    pub fill_2: u32,
}

impl Renderer {
    pub fn new(
        context: &Context,
        images_len: u32,
        storage_images: &Vec<ImageAndView>,
        octtree_buffer: &Buffer,
        octtree_lookup_buffer: &Buffer,
        material_buffer: &Buffer,
    ) -> Result<Self> {
        let render_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<RenderBuffer>() as _,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            images_len * 5,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_IMAGE,
                    descriptor_count: images_len,
                },
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
        ])?;

        let mut descriptor_sets = Vec::new();
        for i in 0..images_len {
            let render_descriptor_set = descriptor_pool.allocate_set(&descriptor_layout)?;

            render_descriptor_set.update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::StorageImage {
                        layout: vk::ImageLayout::GENERAL,
                        view: &storage_images[i as usize].view,
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
                        buffer: &octtree_lookup_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 3,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &octtree_buffer,
                    },
                },
                WriteDescriptorSet {
                    binding: 4,
                    kind: WriteDescriptorSetKind::StorageBuffer {
                        buffer: &material_buffer,
                    },
                },
            ]);
            descriptor_sets.push(render_descriptor_set);
        }

        let pipeline_layout = context.create_pipeline_layout(&[&descriptor_layout])?;

        let pipeline = context.create_compute_pipeline(
            &pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/ray_caster.comp.spv")[..],
            },
        )?;

        Ok(Renderer {
            ubo_buffer: render_buffer,
            descriptor_pool,
            descriptor_layout,
            descriptor_sets,
            pipeline_layout,
            pipeline,
        })
    }

    pub fn render(
        &self,
        base: &BaseApp<RayCaster>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        buffer.bind_compute_pipeline(&self.pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.pipeline_layout,
            0,
            &[&self.descriptor_sets[image_index]],
        );

        buffer.dispatch(
            (base.swapchain.extent.width / RENDER_DISPATCH_GROUP_SIZE_X) + 1,
            (base.swapchain.extent.height / RENDER_DISPATCH_GROUP_SIZE_Y) + 1,
            1,
        );

        Ok(())
    }
}
