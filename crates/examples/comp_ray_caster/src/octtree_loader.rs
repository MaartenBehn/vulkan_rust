use std::mem::size_of;

use app::BaseApp;
use app::anyhow::{Result, Ok};
use app::vulkan::ash::vk;
use app::vulkan::gpu_allocator::MemoryLocation;
use app::vulkan::{DescriptorPool, DescriptorSetLayout, DescriptorSet, PipelineLayout, ComputePipeline, Context, Buffer, WriteDescriptorSet, WriteDescriptorSetKind, ComputePipelineCreateInfo, CommandBuffer, MemoryBarrier};
use octtree::octtree_node::OcttreeNode;

use crate::RayCaster;
use crate::octtree_controller::OcttreeController;

pub const LOAD_DEBUG_DATA_SIZE: usize = 2;
pub const REQUEST_STEP: usize = 4;
const REQUEST_NOTE_STEP: usize = 4;

pub struct OcttreeLoader {
    pub transfer_buffer: Buffer,
    pub request_buffer: Buffer,
    pub request_note_buffer: Buffer,

    pub load_tree: bool,
    pub descriptor_pool: DescriptorPool,
    pub descriptor_layout: DescriptorSetLayout,
    pub descriptor_set: DescriptorSet,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: ComputePipeline,
}

impl OcttreeLoader {
    pub fn new(
        context: &Context, 
        octtree_controller: &OcttreeController,
        octtree_buffer: &Buffer, 
        octtree_info_buffer: &Buffer,
    ) -> Result<Self> {

        let transfer_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::CpuToGpu, 
            (size_of::<OcttreeNode>() * octtree_controller.transfer_size) as _,
        )?;

        let request_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::GpuToCpu, 
            (size_of::<u32>() * (REQUEST_STEP * octtree_controller.transfer_size + LOAD_DEBUG_DATA_SIZE)) as _,
        )?;

        let request_note_buffer = context.create_buffer(
            vk::BufferUsageFlags::STORAGE_BUFFER, 
            MemoryLocation::GpuOnly, 
            (size_of::<u32>() * (REQUEST_NOTE_STEP * octtree_controller.transfer_size + 1)) as _,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            5,
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

        let descriptor_layout = context.create_descriptor_set_layout(&[
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


        let descriptor_set = descriptor_pool.allocate_set(&descriptor_layout)?;
        descriptor_set.update(&[
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
                    buffer: &transfer_buffer
                } 
            },
            WriteDescriptorSet {
                binding: 3,
                kind: WriteDescriptorSetKind::StorageBuffer {  
                    buffer: &request_buffer
                } 
            },
            WriteDescriptorSet {
                binding: 4,
                kind: WriteDescriptorSetKind::StorageBuffer {  
                    buffer: &request_note_buffer
                } 
            },
        ]);

        let pipeline_layout =
            context.create_pipeline_layout(&[&descriptor_layout])?;

        let pipeline = context.create_compute_pipeline(
            &pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/load_tree.comp.spv")[..],
            },
        )?;

        Ok(OcttreeLoader { 
            transfer_buffer,
            request_buffer,
            request_note_buffer,

            load_tree: true, 
            descriptor_pool,
            descriptor_layout, 
            descriptor_set, 
            pipeline_layout,
            pipeline 
        })
    }

    pub fn render(
        &self, 
        _base: &BaseApp<RayCaster>,
        buffer: &CommandBuffer,
        _image_index: usize
    ) -> Result<()> {
        buffer.pipeline_memory_barriers(&[MemoryBarrier {
            src_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
            src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
            dst_access_mask: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
            dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
        }]);

        buffer.bind_compute_pipeline(&self.pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.pipeline_layout,
            0,
        &[&self.descriptor_set],
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

        Ok(())
    }
    
}