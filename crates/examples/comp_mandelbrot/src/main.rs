use std::mem::size_of;
use std::time::Duration;

use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::glam::{uvec2, Vec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    Buffer, ComputePipeline, ComputePipelineCreateInfo, DescriptorPool, DescriptorSet,
    DescriptorSetLayout, PipelineLayout, WriteDescriptorSet, WriteDescriptorSetKind,
};
use octa_force::{App, BaseApp, ImageAndView};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Comp Mandelbrot";

const DISPATCH_GROUP_SIZE_X: u32 = 32;
const DISPATCH_GROUP_SIZE_Y: u32 = 32;

fn main() -> Result<()> {
    octa_force::run::<Mandelbrot>(APP_NAME, uvec2(WIDTH, HEIGHT), false)
}
struct Mandelbrot {
    stored_images: Vec<ImageAndView>,
    compute_ubo_buffer: Buffer,
    _compute_descriptor_pool: DescriptorPool,
    _compute_descriptor_layout: DescriptorSetLayout,
    compute_descriptor_sets: Vec<DescriptorSet>,
    compute_pipeline_layout: PipelineLayout,
    compute_pipeline: ComputePipeline,

    camera: Camera,
}

impl App for Mandelbrot {
    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let images = &base.swapchain.images;

        let stored_images = context.create_storage_images(
            base.swapchain.format,
            base.swapchain.extent,
            images.len(),
        )?;

        let compute_ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<ComputeUbo>() as _,
        )?;

        let compute_descriptor_pool = context.create_descriptor_pool(
            4 * images.len() as u32,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_IMAGE,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
            ],
        )?;

        let compute_descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                stage_flags: vk::ShaderStageFlags::ALL,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
        ])?;

        let mut compute_descriptor_sets = Vec::new();
        for i in 0..images.len() {
            let compute_descriptor_set =
                compute_descriptor_pool.allocate_set(&compute_descriptor_layout)?;

            compute_descriptor_set.update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::StorageImage {
                        layout: vk::ImageLayout::GENERAL,
                        view: &stored_images[i].view,
                    },
                },
                WriteDescriptorSet {
                    binding: 1,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &compute_ubo_buffer,
                    },
                },
            ]);
            compute_descriptor_sets.push(compute_descriptor_set);
        }

        let compute_pipeline_layout =
            context.create_pipeline_layout(&[&compute_descriptor_layout], &[])?;

        let compute_pipeline = context.create_compute_pipeline(
            &compute_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/mandelbrot.comp.spv")[..],
            },
        )?;

        let mut camera = Camera::base(base.swapchain.extent);
        camera.position.z = 2.0;
        camera.z_far = 100.0;

        Ok(Self {
            stored_images,
            compute_ubo_buffer,
            _compute_descriptor_pool: compute_descriptor_pool,
            _compute_descriptor_layout: compute_descriptor_layout,
            compute_descriptor_sets,
            compute_pipeline_layout,
            compute_pipeline,
            camera,
        })
    }

    fn record_render_commands(
        &mut self,
        base: &mut BaseApp<Self>,
        image_index: usize,
    ) -> Result<()> {
        let buffer = &base.command_buffers[image_index];

        buffer.bind_compute_pipeline(&self.compute_pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.compute_pipeline_layout,
            0,
            &[&self.compute_descriptor_sets[image_index]],
        );

        buffer.dispatch(
            (base.swapchain.extent.width / DISPATCH_GROUP_SIZE_X) + 1,
            (base.swapchain.extent.height / DISPATCH_GROUP_SIZE_Y) + 1,
            1,
        );

        buffer.swapchain_image_copy_from_compute_storage_image(
            &self.stored_images[image_index].image,
            &base.swapchain.images[image_index],
        )?;

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &mut BaseApp<Self>) -> Result<()> {
        let stored_images = base.context.create_storage_images(
            base.swapchain.format,
            base.swapchain.extent,
            base.swapchain.images.len(),
        )?;

        stored_images.iter().enumerate().for_each(|(index, img)| {
            let set = &self.compute_descriptor_sets[index];

            set.update(&[WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::StorageImage {
                    layout: vk::ImageLayout::GENERAL,
                    view: &img.view,
                },
            }]);
        });

        self.stored_images = stored_images;

        Ok(())
    }

    fn update(
        &mut self,
        _base: &mut BaseApp<Self>,
        _image_index: usize,
        _delta_time: Duration,
    ) -> Result<()> {
        self.compute_ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
            pos: self.camera.position,
            dir: self.camera.direction,
        }])
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ComputeUbo {
    pos: Vec3,
    dir: Vec3,
}
