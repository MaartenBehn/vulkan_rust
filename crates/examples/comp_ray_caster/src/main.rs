use std::mem::size_of;
use std::time::{Duration};

use app::anyhow::Result;
use app::glam::{Vec3};
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{
    Buffer, CommandBuffer, ComputePipeline, ComputePipelineCreateInfo,
    DescriptorPool, DescriptorSet, DescriptorSetLayout, PipelineLayout, 
    WriteDescriptorSet, WriteDescriptorSetKind,
};
use app::{App, BaseApp};
use gui::imgui::{Condition, Ui};


mod octtree;
use octtree::*;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Ray Caster";

const RENDER_DISPATCH_GROUP_SIZE_X: u32 = 32;
const RENDER_DISPATCH_GROUP_SIZE_Y: u32 = 32;

fn main() -> Result<()> {
    app::run::<RayCaster>(APP_NAME, WIDTH, HEIGHT, false, true)
}
struct RayCaster {
    render_ubo_buffer: Buffer,
    _render_descriptor_pool: DescriptorPool,
    _render_descriptor_layout: DescriptorSetLayout,
    render_descriptor_sets: Vec<DescriptorSet>,
    render_pipeline_layout: PipelineLayout,
    render_pipeline: ComputePipeline,

    octtree: Octtree,
    octtree_buffer: Buffer,
    update_octtree: bool,
    _update_octtree_descriptor_pool: DescriptorPool,
    _update_octtree_descriptor_layout: DescriptorSetLayout,
    update_octtree_descriptor_set: DescriptorSet,
    update_octtree_pipeline_layout: PipelineLayout,
    update_octtree_pipeline: ComputePipeline,

    render_counter: u8, 
}

impl App for RayCaster {
    type Gui = Gui;

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let images = &base.swapchain.images;
        let images_len = images.len() as u32;

        let render_ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<ComputeUbo>() as _,
        )?;

        let octtree = Octtree::new();

        /*
        let octtree_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::CpuToGpu,
            (size_of::<OcttreeNode>() * OCTTREE_NODE_COUNT) as _,
        )?;

        octtree_buffer.copy_data_to_buffer(&[octtree])?;
        */

        let octtree_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
            &[octtree],
        )?;
        
        let render_descriptor_pool = context.create_descriptor_pool(
            images_len * 3,
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
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: images_len,
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
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
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
                WriteDescriptorSet {
                    binding: 2,
                    kind: WriteDescriptorSetKind::StorageBuffer { 
                        buffer: &octtree_buffer
                    } 
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

        let render_pipeline_layout =
            context.create_pipeline_layout(&[&render_descriptor_layout])?;

        let render_pipeline = context.create_compute_pipeline(
            &render_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/ray_caster.comp.spv")[..],
            },
        )?;

        let update_octtree_pipeline_layout =
            context.create_pipeline_layout(&[&update_octtree_descriptor_layout])?;

        let update_octtree_pipeline = context.create_compute_pipeline(
            &render_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/build_tree.comp.spv")[..],
            },
        )?;

        base.camera.position = Vec3::new(-10.0, 8.0, 8.0);
        base.camera.direction = Vec3::new(1.0, 0.0,0.0).normalize();
        base.camera.z_far = 100.0;

        Ok(Self {
            render_ubo_buffer,
            _render_descriptor_pool: render_descriptor_pool,
            _render_descriptor_layout: render_descriptor_layout,
            render_descriptor_sets,
            render_pipeline_layout,
            render_pipeline,

            octtree,
            octtree_buffer,
            update_octtree: true,
            _update_octtree_descriptor_pool: update_octtree_descriptor_pool,
            _update_octtree_descriptor_layout: update_octtree_descriptor_layout,
            update_octtree_descriptor_set,
            update_octtree_pipeline_layout,
            update_octtree_pipeline,

            render_counter: 0,
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
            root_node_index: 0,
            //render_counter: self.render_counter, 
            render_counter: self.render_counter as u32,
            //fill_01: 0,
            pos: base.camera.position,
            fill_1: 0,
            dir: base.camera.direction,
            fill_2: 0
        }])?;

        self.update_octtree = false;

        // Updateing Gui
        gui.pos = base.camera.position;
        gui.dir = base.camera.direction;
        gui.render_counter = self.render_counter;

        // Incrementing render counter
        if self.render_counter < 255{
            self.render_counter += 1;
        }

        self.render_counter = if self.render_counter < 255 {
            self.render_counter + 1
        } else {
            0
        };

        
        Ok(())
    }

    fn record_compute_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize
    ) -> Result<()> {

        if self.update_octtree {
            buffer.bind_compute_pipeline(&self.update_octtree_pipeline);

            buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::COMPUTE,
                &self.update_octtree_pipeline_layout,
        0,
            &[&self.update_octtree_descriptor_set],
            );

            buffer.dispatch(
                OCTTREE_SIZE as u32, 
                OCTTREE_SIZE as u32, 
                OCTTREE_SIZE as u32,
            );
        }

        buffer.bind_compute_pipeline(&self.render_pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.render_pipeline_layout,
            0,
            &[&self.render_descriptor_sets[image_index]],
        );

        buffer.dispatch(
            (base.swapchain.extent.width / RENDER_DISPATCH_GROUP_SIZE_X) + 1, 
            (base.swapchain.extent.height / RENDER_DISPATCH_GROUP_SIZE_Y) + 1, 
            1);

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
    pos: Vec3,
    dir: Vec3,
    render_counter: u8,
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            pos: Vec3::default(),
            dir: Vec3::default(),
            render_counter: 0,
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Ray caster")
            .position([5.0, 150.0], Condition::FirstUseEver)
            .size([300.0, 100.0], Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .build(|| {
                let pos = self.pos;
                ui.text(format!("Pos: {pos}"));

                let dir = self.dir;
                ui.text(format!("Dir: {dir}"));

                let render_counter = self.render_counter;
                ui.text(format!("Render Counter: {render_counter}"));
            });
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ComputeUbo {
    screen_size: [f32; 2],
    root_node_index: u32,
    render_counter: u32,

    pos: Vec3,
    fill_1: u32,

    dir: Vec3,
    fill_2: u32,
}

