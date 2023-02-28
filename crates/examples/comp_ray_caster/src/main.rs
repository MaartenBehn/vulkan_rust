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
const APP_NAME: &str = "Ray Caster";

const DISPATCH_GROUP_SIZE_X: u32 = 32;
const DISPATCH_GROUP_SIZE_Y: u32 = 32;

const OCTTREE_SIZE: usize = 32;
const OCTTREE_NODE_COUNT: usize = OCTTREE_SIZE * OCTTREE_SIZE * OCTTREE_SIZE;

fn main() -> Result<()> {
    app::run::<Particles>(APP_NAME, WIDTH, HEIGHT, false, true)
}
struct Particles {
    render_ubo_buffer: Buffer,
    _render_descriptor_pool: DescriptorPool,
    _render_descriptor_layout: DescriptorSetLayout,
    render_descriptor_sets: Vec<DescriptorSet>,
    render_pipeline_layout: PipelineLayout,
    render_pipeline: ComputePipeline,
}

impl App for Particles {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let images = &base.swapchain.images;
        let render_ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<ComputeUbo>() as _,
        )?;
        
        let octtree_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::CpuToGpu,
            (size_of::<OcttreeNode>() * OCTTREE_NODE_COUNT) as _,
        )?;

        let render_descriptor_pool = context.create_descriptor_pool(
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

        let update_octtree_descriptor_pool = context.create_descriptor_pool(
            1,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                },
            ],
        )?;

        let render_descriptor_layout = context.create_descriptor_set_layout(&[
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
        ])?;

        let update_octtree_descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
        ])?;

        let mut render_descriptor_sets = Vec::new();
        for i in 0..images.len(){
            let render_descriptor_set =
            render_descriptor_pool.allocate_set(&render_descriptor_layout)?;

            render_descriptor_set.update(&[
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
                        buffer: &render_ubo_buffer,
                    },
                },
            ]);
            render_descriptor_sets.push(render_descriptor_set);
        }


        let update_octtree_descriptor_set = update_octtree_descriptor_pool.allocate_set(&update_octtree_descriptor_layout)?;
        update_octtree_descriptor_set.update(&[
            WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::StorageBuffer { 
                    buffer: &octtree_buffer
                } 
            },
        ]);
        let update_octtree_descriptor_sets = [update_octtree_descriptor_set];


        let render_pipeline_layout =
            context.create_pipeline_layout(&[&render_descriptor_layout])?;

        let render_pipeline = context.create_compute_pipeline(
            &render_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/ray_caster.comp.spv")[..],
            },
        )?;

        base.camera.position.z = 2.0;
        base.camera.z_far = 100.0;

        Ok(Self {
            render_ubo_buffer,
            _render_descriptor_pool: render_descriptor_pool,
            _render_descriptor_layout: render_descriptor_layout,
            render_descriptor_sets,
            render_pipeline_layout,
            render_pipeline,
        })
    }

    fn update(
        &mut self,
        base: &BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _: usize,
        delta_time: Duration,
    ) -> Result<()> {
    
        self.render_ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
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

        buffer.bind_compute_pipeline(&self.render_pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.render_pipeline_layout,
            0,
            &[&self.render_descriptor_sets[image_index]],
        );

        buffer.dispatch((base.swapchain.extent.width / DISPATCH_GROUP_SIZE_X) + 1, (base.swapchain.extent.height / DISPATCH_GROUP_SIZE_Y) + 1, 1);

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()> {
        base.storage_images
            .iter()
            .enumerate()
            .for_each(|(index, img)| {
                let set = &self.render_descriptor_sets[index];

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


#[derive(Clone, Copy)]
struct OcttreeNode {
    childrenStartIndex: u32,
    data: u32,
}
