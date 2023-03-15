use std::mem::size_of;
use std::thread;
use std::time::{Duration, self};

use app::anyhow::Result;
use app::glam::{Vec3};
use app::vulkan::ash::vk::{self};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{
    Buffer, CommandBuffer, ComputePipeline, ComputePipelineCreateInfo,
    DescriptorPool, DescriptorSet, DescriptorSetLayout, PipelineLayout, 
    WriteDescriptorSet, WriteDescriptorSetKind, BufferBarrier, MemoryBarrier,
};
use app::{App, BaseApp, log};
use gui::imgui::{Condition, Ui};


mod octtree;
use octtree::*;

mod materials;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Ray Caster";

const RENDER_DISPATCH_GROUP_SIZE_X: u32 = 32;
const RENDER_DISPATCH_GROUP_SIZE_Y: u32 = 32;

const LOAD_DISPATCH_GROUP_SIZE: u32 = 32;
const BUILD_DISPATCH_GROUP_SIZE: u32 = 32;
const LOAD_DEBUG_DATA_SIZE: usize = 2;

fn main() -> Result<()> {
    app::run::<RayCaster>(APP_NAME, WIDTH, HEIGHT, false, true)
}
struct RayCaster {
    total_time: Duration,
    frameCounter: usize,

    render_ubo_buffer: Buffer,
    _render_descriptor_pool: DescriptorPool,
    _render_descriptor_layout: DescriptorSetLayout,
    render_descriptor_sets: Vec<DescriptorSet>,
    render_pipeline_layout: PipelineLayout,
    render_pipeline: ComputePipeline,

    octtree_controller: OcttreeController,
    octtree_buffer: Buffer,
    octtree_info_buffer: Buffer,
    octtree_transfer_buffer: Buffer,
    octtree_request_buffer: Buffer,
    octtree_request_note_buffer: Buffer,

    build_tree: bool,
    _build_descriptor_pool: DescriptorPool,
    _build_descriptor_layout: DescriptorSetLayout,
    build_octtree_descriptor_set: DescriptorSet,
    build_octtree_pipeline_layout: PipelineLayout,
    build_octtree_pipeline: ComputePipeline,

    load_tree: bool,
    _load_descriptor_pool: DescriptorPool,
    _load_descriptor_layout: DescriptorSetLayout,
    load_octtree_descriptor_set: DescriptorSet,
    load_octtree_pipeline_layout: PipelineLayout,
    load_octtree_pipeline: ComputePipeline,
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

        let depth = 7;
        let mut octtree_controller = OcttreeController::new(
            Octtree::new(depth, 123), 
            u16::MAX as usize, //Octtree::get_tree_size(depth),
            1000,
        );

        let octtree_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::GpuOnly, 
            (size_of::<u32>() * 4 * 3 * octtree_controller.buffer_size) as _,
        )?;

        let octtree_info_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<OcttreeInfo>() as _,
        )?;

        let octtree_transfer_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::CpuToGpu, 
            (size_of::<OcttreeNode>() * octtree_controller.transfer_size) as _,
        )?;

        let octtree_request_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::GpuToCpu, 
            (size_of::<u32>() * (octtree_controller.transfer_size + LOAD_DEBUG_DATA_SIZE)) as _,
        )?;

        let octtree_request_note_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::GpuOnly, 
            (size_of::<u32>() * 4 * octtree_controller.transfer_size) as _,
        )?;
        
        let render_descriptor_pool = context.create_descriptor_pool(
            images_len * 4,
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
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: images_len,
                },
            ],
        )?;

        let build_octtree_descriptor_pool = context.create_descriptor_pool(
            1,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
            ],
        )?;

        let load_octtree_descriptor_pool = context.create_descriptor_pool(
            1,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                },
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
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
        ])?;

        let build_octtree_descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
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

        let load_octtree_descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
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
                    },
                },
                WriteDescriptorSet {
                    binding: 3,
                    kind: WriteDescriptorSetKind::UniformBuffer {  
                        buffer: &octtree_info_buffer
                    },
                },
            ]);
            render_descriptor_sets.push(render_descriptor_set);
        }

        let build_octtree_descriptor_set = build_octtree_descriptor_pool.allocate_set(&build_octtree_descriptor_layout)?;
        build_octtree_descriptor_set.update(&[
            WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::StorageBuffer { 
                    buffer: &octtree_buffer
                } 
            },
            WriteDescriptorSet {
                binding: 1,
                kind: WriteDescriptorSetKind::UniformBuffer {  
                    buffer: &octtree_info_buffer
                } 
            },
        ]);

        let load_octtree_descriptor_set = load_octtree_descriptor_pool.allocate_set(&load_octtree_descriptor_layout)?;
        load_octtree_descriptor_set.update(&[
            WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::StorageBuffer { 
                    buffer: &octtree_buffer
                } 
            },
            WriteDescriptorSet {
                binding: 1,
                kind: WriteDescriptorSetKind::UniformBuffer {  
                    buffer: &octtree_info_buffer
                } 
            },
            WriteDescriptorSet {
                binding: 2,
                kind: WriteDescriptorSetKind::StorageBuffer {  
                    buffer: &octtree_transfer_buffer
                } 
            },
            WriteDescriptorSet {
                binding: 3,
                kind: WriteDescriptorSetKind::StorageBuffer {  
                    buffer: &octtree_request_buffer
                } 
            },
            WriteDescriptorSet {
                binding: 4,
                kind: WriteDescriptorSetKind::StorageBuffer {  
                    buffer: &octtree_request_note_buffer
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

        let build_octtree_pipeline_layout =
            context.create_pipeline_layout(&[&build_octtree_descriptor_layout])?;

        let build_octtree_pipeline = context.create_compute_pipeline(
            &build_octtree_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/build_tree.comp.spv")[..],
            },
        )?;

        let load_octtree_pipeline_layout =
            context.create_pipeline_layout(&[&load_octtree_descriptor_layout])?;

        let load_octtree_pipeline = context.create_compute_pipeline(
            &load_octtree_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/load_tree.comp.spv")[..],
            },
        )?;

        base.camera.position = Vec3::new(-5.0, 0.0, 0.0);
        base.camera.direction = Vec3::new(1.0, 0.0,0.0).normalize();
        base.camera.z_far = 100.0;

        Ok(Self {
            total_time: Duration::ZERO,
            frameCounter: 0,

            render_ubo_buffer,
            _render_descriptor_pool: render_descriptor_pool,
            _render_descriptor_layout: render_descriptor_layout,
            render_descriptor_sets,
            render_pipeline_layout,
            render_pipeline,

            octtree_controller,
            octtree_buffer,
            octtree_info_buffer,
            octtree_transfer_buffer,
            octtree_request_buffer,
            octtree_request_note_buffer,

            build_tree: false,
            _build_descriptor_pool: build_octtree_descriptor_pool,
            _build_descriptor_layout: build_octtree_descriptor_layout,
            build_octtree_descriptor_set,
            build_octtree_pipeline_layout,
            build_octtree_pipeline,

            load_tree: false,
            _load_descriptor_pool: load_octtree_descriptor_pool,
            _load_descriptor_layout: load_octtree_descriptor_layout,
            load_octtree_descriptor_set,
            load_octtree_pipeline_layout,
            load_octtree_pipeline,
        })
    }

    fn update(
        &mut self,
        base: &BaseApp<Self>,
        gui: &mut <Self as App>::Gui,
        _: usize,
        delta_time: Duration,
    ) -> Result<()> {

        self.total_time += delta_time;
        
        self.octtree_info_buffer.copy_data_to_buffer(&[self.octtree_controller.octtree_info])?;
        self.render_ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
            screen_size: [base.swapchain.extent.width as f32, base.swapchain.extent.height as f32],
            mode: gui.mode,
            debug_scale: gui.debug_scale,
            pos: base.camera.position,
            fill_1: 0,
            dir: base.camera.direction,
            fill_2: 0,
        }])?;

        self.build_tree = gui.build || self.frameCounter == 0;
        self.load_tree = gui.load && self.frameCounter != 0;

        if self.load_tree {
            let mut request_data: Vec<u32> = self.octtree_request_buffer.get_data_from_buffer(self.octtree_controller.transfer_size + LOAD_DEBUG_DATA_SIZE)?;

            // Debug data from load shader
            gui.render_counter = request_data[self.octtree_controller.transfer_size] as usize;
            gui.needs_children_counter = request_data[self.octtree_controller.transfer_size + 1] as usize;
            request_data.truncate(self.octtree_controller.transfer_size);

            //log::debug!("{:?}", request_data);
            let (requested_nodes, counter) = self.octtree_controller.get_requested_nodes(request_data);
            self.octtree_transfer_buffer.copy_data_to_buffer(&requested_nodes)?;

            gui.transfer_counter = counter;
        }

        // Updateing Gui
        gui.pos = base.camera.position;
        gui.dir = base.camera.direction;
        gui.octtree_buffer_size = self.octtree_controller.buffer_size;
        gui.transfer_buffer_size = self.octtree_controller.transfer_size;


        self.octtree_controller.step();
        self.frameCounter += 1;

        Ok(())
    }

    fn record_compute_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize
    ) -> Result<()> {

        if self.load_tree {
            buffer.bind_compute_pipeline(&self.load_octtree_pipeline);

            buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::COMPUTE,
                &self.load_octtree_pipeline_layout,
                0,
            &[&self.load_octtree_descriptor_set],
            );

            buffer.dispatch(
                1, 
                1, 
                1,
            );
            buffer.pipeline_memory_barriers(&[MemoryBarrier {
                src_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                dst_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }]);
        }

        if self.build_tree {
            buffer.bind_compute_pipeline(&self.build_octtree_pipeline);

            buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::COMPUTE,
                &self.build_octtree_pipeline_layout,
                0,
            &[&self.build_octtree_descriptor_set],
            );

            buffer.dispatch(
                (self.octtree_controller.buffer_size as u32 / BUILD_DISPATCH_GROUP_SIZE) + 1, 
                1, 
                1,
            );
        }

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.render_pipeline_layout,
            0,
            &[&self.render_descriptor_sets[image_index]],
        );

        buffer.bind_compute_pipeline(&self.render_pipeline);
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
    mode: u32,
    build: bool,
    load: bool,
    debug_scale: u32,

    render_counter: usize,
    needs_children_counter: usize,
    octtree_buffer_size: usize,

    transfer_counter: usize,
    transfer_buffer_size: usize
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            pos: Vec3::default(),
            dir: Vec3::default(),
            mode: 1,
            build: false,
            load: true,
            debug_scale: 1,

            render_counter: 0,
            needs_children_counter: 0,
            octtree_buffer_size: 0,
            transfer_counter: 0,
            transfer_buffer_size: 0,
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Ray caster")
            .position([5.0, 150.0], Condition::FirstUseEver)
            .size([300.0, 300.0], Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .build(|| {
                let pos = self.pos;
                ui.text(format!("Pos: {pos}"));

                let dir = self.dir;
                ui.text(format!("Dir: {dir}"));

                let mut mode = self.mode as i32;
                ui.input_int("Mode", &mut mode).build();
                mode = mode.clamp(0, 4);
                self.mode = mode as u32;

                let mut debug_scale = self.debug_scale as i32;
                ui.input_int("Scale", &mut debug_scale).build();
                debug_scale = debug_scale.clamp(1, 100);
                self.debug_scale = debug_scale as u32;

                let mut build = self.build;
                if ui.radio_button_bool("Build Tree", build){
                    build = !build;
                }
                self.build = build;

                let mut load = self.load;
                if ui.radio_button_bool("Load Tree", load){
                    load = !load;
                }
                self.load = load;

                let render_counter = self.render_counter;
                let percent = (self.render_counter as f32 / self.octtree_buffer_size as f32) * 100.0; 
                ui.text(format!("Rendered Nodes: {render_counter} ({:.0}%)", percent));

                let needs_children = self.needs_children_counter;
                ui.text(format!("Needs Children: {needs_children}"));


                let transfer_counter = self.transfer_counter;
                let percent = (self.transfer_counter as f32 / self.transfer_buffer_size as f32) * 100.0; 
                ui.text(format!("Transfered Nodes: {transfer_counter} ({:.0}%)", percent));

            });
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ComputeUbo {
    screen_size: [f32; 2],
    mode: u32,
    debug_scale: u32,

    pos: Vec3,
    fill_1: u32,

    dir: Vec3,
    fill_2: u32,
}
