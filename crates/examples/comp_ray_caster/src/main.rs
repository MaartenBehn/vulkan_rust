use std::mem::size_of;
use std::time::{Duration};

use app::anyhow::Result;
use app::glam::{Vec3};
use app::vulkan::ash::vk::{self, MemoryBarrier2};
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{
    Buffer, CommandBuffer, ComputePipeline, ComputePipelineCreateInfo,
    DescriptorPool, DescriptorSet, DescriptorSetLayout, PipelineLayout, 
    WriteDescriptorSet, WriteDescriptorSetKind, MemoryBarrier,
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
    octtree_info_buffer: Buffer,
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

        let octtree_buffer = create_gpu_only_buffer_from_data(
            context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &[octtree],
        )?;

        let octtree_info_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<OcttreeInfo>() as _,
        )?;

        octtree_info_buffer.copy_data_to_buffer(&[OcttreeInfo {
            octtreeBufferSize: OCTTREE_NODE_COUNT as u32,
            octtreeDepth: OCTTREE_DEPTH as u32,

            fill_0: 0,
            fill_1: 0,
        }])?;
        
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

        let update_octtree_descriptor_pool = context.create_descriptor_pool(
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

        let update_octtree_descriptor_layout = context.create_descriptor_set_layout(&[
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


        let update_octtree_descriptor_set = update_octtree_descriptor_pool.allocate_set(&update_octtree_descriptor_layout)?;
        update_octtree_descriptor_set.update(&[
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

        base.camera.position = Vec3::new(-5.0, 1.0, 1.0);
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
            octtree_info_buffer,
            update_octtree: false,
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
            mode: gui.mode,
            cleanUp: 1, // (self.render_counter == 128) as u32,
            pos: base.camera.position + Vec3::new(self.render_counter as f32, 0.0, 0.0),
            fill_1: 0,
            dir: base.camera.direction,
            fill_2: 0,
        }])?;

        self.update_octtree = true; //&& self.render_counter == 0;

        // Updateing Gui
        gui.pos = base.camera.position;
        gui.dir = base.camera.direction;

        self.render_counter = if self.render_counter < 255{
            self.render_counter + 1
        }else{
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
            buffer.pipeline_memory_barriers(&[MemoryBarrier {
                src_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                dst_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            }]);


            buffer.bind_compute_pipeline(&self.update_octtree_pipeline);

            buffer.bind_descriptor_sets(
                vk::PipelineBindPoint::COMPUTE,
                &self.update_octtree_pipeline_layout,
                0,
            &[&self.update_octtree_descriptor_set],
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
    mode: u32,
    cach: bool,
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            pos: Vec3::default(),
            dir: Vec3::default(),
            mode: 1,
            cach: false,
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
    cleanUp: u32,

    pos: Vec3,
    fill_1: u32,

    dir: Vec3,
    fill_2: u32,
}


#[derive(Clone, Copy)]
#[allow(dead_code)]
struct OcttreeInfo {
    octtreeBufferSize: u32,
    octtreeDepth: u32,
    fill_0: u32,
    fill_1: u32,
}
