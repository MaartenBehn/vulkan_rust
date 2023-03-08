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
    WriteDescriptorSet, WriteDescriptorSetKind, BufferBarrier,
};
use app::{App, BaseApp, log};
use gui::imgui::{Condition, Ui};


mod octtree;
use octtree::*;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Ray Caster";

const RENDER_DISPATCH_GROUP_SIZE_X: u32 = 32;
const RENDER_DISPATCH_GROUP_SIZE_Y: u32 = 32;

const BUILD_DISPATCH_GROUP_SIZE: u32 = 32;

fn main() -> Result<()> {
    app::run::<RayCaster>(APP_NAME, WIDTH, HEIGHT, false, true)
}
struct RayCaster {
    total_time: Duration,

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

    update_octtree: bool,

    _build_descriptor_pool: DescriptorPool,
    _build_descriptor_layout: DescriptorSetLayout,
    build_octtree_descriptor_set: DescriptorSet,
    build_octtree_pipeline_layout: PipelineLayout,
    build_octtree_pipeline: ComputePipeline,

    _load_descriptor_pool: DescriptorPool,
    _load_descriptor_layout: DescriptorSetLayout,
    load_octtree_descriptor_set: DescriptorSet,
    load_octtree_pipeline_layout: PipelineLayout,
    load_octtree_pipeline: ComputePipeline,

    update_octtree_intervall: Duration,
    update_octtree_last_time: Duration,
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

        let octtree_controller = OcttreeController::new(Octtree::new(4, 123), 2000, 16);

        let octtree_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            octtree_controller.get_inital_buffer_data(),
        )?;

        let octtree_info_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<OcttreeInfo>() as _,
        )?;
        octtree_info_buffer.copy_data_to_buffer(&[octtree_controller.get_octtree_info()])?;

        let octtree_transfer_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::CpuToGpu, 
            (size_of::<OcttreeNode>() * octtree_controller.transfer_size) as _,
        )?;

        let octtree_request_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::GpuToCpu, 
            (size_of::<u32>() * octtree_controller.transfer_size) as _,
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
            2,
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
            2,
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

            update_octtree: false,

            _build_descriptor_pool: build_octtree_descriptor_pool,
            _build_descriptor_layout: build_octtree_descriptor_layout,
            build_octtree_descriptor_set,
            build_octtree_pipeline_layout,
            build_octtree_pipeline,

            _load_descriptor_pool: load_octtree_descriptor_pool,
            _load_descriptor_layout: load_octtree_descriptor_layout,
            load_octtree_descriptor_set,
            load_octtree_pipeline_layout,
            load_octtree_pipeline,

            update_octtree_intervall: Duration::from_millis(10),
            update_octtree_last_time: Duration::ZERO,
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

        self.render_ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
            screen_size: [base.swapchain.extent.width as f32, base.swapchain.extent.height as f32],
            mode: gui.mode,
            debug_scale: gui.debug_scale,
            pos: base.camera.position,
            fill_1: 0,
            dir: base.camera.direction,
            fill_2: 0,
        }])?;

        self.update_octtree = if self.update_octtree_last_time + self.update_octtree_intervall < self.total_time {
            self.update_octtree_last_time = self.total_time;
            gui.cach
        }else{
            false
        };

        self.update_octtree = gui.cach;

        if self.update_octtree {
            let request_data: Vec<u32> = self.octtree_request_buffer.get_data_from_buffer(self.octtree_controller.transfer_size)?;
            let requested_nodes = self.octtree_controller.get_requested_nodes(request_data);
            self.octtree_transfer_buffer.copy_data_to_buffer(&requested_nodes)?;

        }

        // Updateing Gui
        gui.pos = base.camera.position;
        gui.dir = base.camera.direction;

        Ok(())
    }

    fn record_compute_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize
    ) -> Result<()> {

        if self.update_octtree {
            /* 
            buffer.pipeline_memory_barriers(&[MemoryBarrier {
                src_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                dst_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }]);
            */

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

            buffer.pipeline_buffer_barriers(&[BufferBarrier {
                buffer: &self.octtree_buffer,
                src_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                dst_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }]);
            
            buffer.bind_compute_pipeline(&self.build_octtree_pipeline);

            buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::COMPUTE,
                &self.build_octtree_pipeline_layout,
                0,
            &[&self.build_octtree_descriptor_set],
            );

            buffer.dispatch(
                (self.octtree_controller.octtree.size as u32 / BUILD_DISPATCH_GROUP_SIZE) + 1, 
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
    cach: bool,
    debug_scale: u32
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            pos: Vec3::default(),
            dir: Vec3::default(),
            mode: 1,
            cach: true,
            debug_scale: 1,
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Ray caster")
            .position([5.0, 150.0], Condition::FirstUseEver)
            .size([300.0, 150.0], Condition::FirstUseEver)
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

                let mut cach = self.cach;
                if ui.radio_button_bool("Use Cach", cach){
                    cach = !cach;
                }
                self.cach = cach;

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
