use std::mem::size_of;
use std::time::{Duration, Instant};

use app::anyhow::Result;
use app::glam::{vec3, Mat4, Vec3, Vec2};
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{
    Buffer, BufferBarrier, CommandBuffer, ComputePipeline, ComputePipelineCreateInfo, Context,
    DescriptorPool, DescriptorSet, DescriptorSetLayout, GraphicsPipeline,
    GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, PipelineLayout, Vertex,
    WriteDescriptorSet, WriteDescriptorSetKind,
};
use app::{log, App, BaseApp};
use gui::imgui::{Condition, Ui};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Comp Mandelbrot";

const DISPATCH_GROUP_SIZE_X: u32 = 32;
const DISPATCH_GROUP_SIZE_Y: u32 = 32;

fn main() -> Result<()> {
    app::run::<Particles>(APP_NAME, WIDTH, HEIGHT, false, true)
}
struct Particles {
    compute_ubo_buffer: Buffer,
    _compute_descriptor_pool: DescriptorPool,
    _compute_descriptor_layout: DescriptorSetLayout,
    compute_descriptor_sets: Vec<DescriptorSet>,
    compute_pipeline_layout: PipelineLayout,
    compute_pipeline: ComputePipeline,
}

impl App for Particles {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let images = &base.swapchain.images;
        let compute_ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<ComputeUbo>() as _,
        )?;

        let compute_descriptor_pool = context.create_descriptor_pool(
            3,
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
        for i in 0..images.len(){
            let compute_descriptor_set =
            compute_descriptor_pool.allocate_set(&compute_descriptor_layout)?;

            compute_descriptor_set.update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::StorageImage {
                        layout: vk::ImageLayout::GENERAL,
                        view: &base.storage_images[i].view,
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
            context.create_pipeline_layout(&[&compute_descriptor_layout])?;

        let compute_pipeline = context.create_compute_pipeline(
            &compute_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/ray_caster.comp.spv")[..],
            },
        )?;

        base.camera.position.z = 2.0;
        base.camera.z_far = 100.0;

        Ok(Self {
            compute_ubo_buffer,
            _compute_descriptor_pool: compute_descriptor_pool,
            _compute_descriptor_layout: compute_descriptor_layout,
            compute_descriptor_sets,
            compute_pipeline_layout,
            compute_pipeline,
        })
    }

    fn update(
        &mut self,
        base: &BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _: usize,
        delta_time: Duration,
    ) -> Result<()> {
    
        self.compute_ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
            screen_size: [base.swapchain.extent.width as f32, base.swapchain.extent.height as f32],
            pos: base.camera.position,
            dir: base.camera.direction,
        }])?;

        Ok(())
    }

    fn record_compute_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize
    ) -> Result<()> {

        buffer.bind_compute_pipeline(&self.compute_pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.compute_pipeline_layout,
            0,
            &[&self.compute_descriptor_sets[image_index]],
        );

        buffer.dispatch((base.swapchain.extent.width / DISPATCH_GROUP_SIZE_X) + 1, (base.swapchain.extent.height / DISPATCH_GROUP_SIZE_Y) + 1, 1);

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()> {
        base.storage_images
            .iter()
            .enumerate()
            .for_each(|(index, img)| {
                let set = &self.compute_descriptor_sets[index];

                set.update(&[WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::StorageImage {
                        layout: vk::ImageLayout::GENERAL,
                        view: &img.view,
                    },
                }]);
            });

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct Gui {
    
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Debug")
            .position([5.0, 5.0], Condition::FirstUseEver)
            .size([300.0, 250.0], Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .build(|| {
                ui.text("Compute");
            });
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ComputeUbo {
    screen_size: [f32; 2],
    pos: Vec3,
    dir: Vec3,
}
