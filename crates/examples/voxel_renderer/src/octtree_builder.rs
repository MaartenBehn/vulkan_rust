use app::BaseApp;
use app::anyhow::Result;
use app::vulkan::ash::vk;
use app::vulkan::{Context, DescriptorPool, DescriptorSetLayout, DescriptorSet, PipelineLayout, ComputePipeline, Buffer, WriteDescriptorSet, WriteDescriptorSetKind, ComputePipelineCreateInfo, CommandBuffer};

use crate::RayCaster;

const BUILD_DISPATCH_GROUP_SIZE: u32 = 32;

pub struct OcttreeBuilder{
    pub buffer_size: u32,
    pub build_tree: bool,
    pub descriptor_pool: DescriptorPool,
    pub descriptor_layout: DescriptorSetLayout,
    pub descriptor_set: DescriptorSet,
    pub pipeline_layout: PipelineLayout,
    pub pipeline: ComputePipeline,
}

impl OcttreeBuilder{
    pub fn new(
        context: &Context, 
        octtree_buffer: &Buffer, 
        octtree_info_buffer: &Buffer,
        buffer_size: usize,
    ) -> Result<Self> {

        let descriptor_pool = context.create_descriptor_pool(
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
        ]);

        let pipeline_layout =
            context.create_pipeline_layout(&[&descriptor_layout])?;

        let pipeline = context.create_compute_pipeline(
            &pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/build_tree.comp.spv")[..],
            },
        )?;

        Ok(OcttreeBuilder{
            buffer_size: buffer_size as u32,
            build_tree: true,
            descriptor_pool,
            descriptor_layout,
            descriptor_set,
            pipeline_layout,
            pipeline,
        })
    }

    pub fn render(
        &self, 
        _base: &BaseApp<RayCaster>,
        buffer: &CommandBuffer,
        _image_index: usize
    ) -> Result<()> {
        buffer.bind_compute_pipeline(&self.pipeline);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.pipeline_layout,
            0,
        &[&self.descriptor_set],
        );

        buffer.dispatch(
            (self.buffer_size as u32 / BUILD_DISPATCH_GROUP_SIZE) + 1, 
            1, 
            1,
        );

        Ok(())
    }
}