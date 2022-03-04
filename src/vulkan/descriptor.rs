use super::{VulkanApp, buffer::UniformBufferObject, MAX_FRAMES_IN_FLIGHT};

use ash::{vk::{self, ImageLayout, ImageView, DescriptorImageInfo, DescriptorSetLayoutBinding}, Device};
use std::mem::size_of;

impl VulkanApp{

    pub fn create_descriptor_set_layout(device: &Device) -> (vk::DescriptorSetLayout, DescriptorSetLayoutBinding) {

        let binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::ALL)
            // .immutable_samplers() null since we're not creating a sampler descriptor
            .build();

        let bindings = [binding];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .build();

        let layout = unsafe {
             device
                .create_descriptor_set_layout(&layout_info, None)
                .unwrap()};
        (layout, binding)
    }

    /// Create a descriptor pool to allocate the descriptor sets.
    pub fn create_descriptor_pool(device: &Device) -> vk::DescriptorPool {
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_IMAGE,
            descriptor_count: MAX_FRAMES_IN_FLIGHT + 1,
        };
        
        let pool_sizes = [pool_size];

        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(MAX_FRAMES_IN_FLIGHT + 1)
            .build();

        unsafe { device.create_descriptor_pool(&pool_info, None).unwrap() }
    }

    /// Create one descriptor set for each uniform buffer.
    pub fn create_descriptor_sets(
        device: &Device,
        pool: vk::DescriptorPool,
        layout: vk::DescriptorSetLayout,
        image_views: Vec<ImageView>
    ) -> Vec<vk::DescriptorSet> {

        let layouts = (0..image_views.len())
            .map(|_| layout)
            .collect::<Vec<_>>();
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&layouts)
            .build();
        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info).unwrap() };

        descriptor_sets
            .iter()
            .zip(image_views.iter())
            .for_each(|(set, image_view)| {
                let image_info = vk::DescriptorImageInfo::builder()
                    .image_view(*image_view)
                    .image_layout(ImageLayout::GENERAL)
                    .build();

                let image_infos = [image_info];

                let descriptor_write = vk::WriteDescriptorSet::builder()
                    .dst_set(*set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                    .image_info(&image_infos)
                    .build();

                let descriptor_writes = [descriptor_write];

                unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) }
            });

        descriptor_sets
    }
}