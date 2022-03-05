use super::VulkanApp;

use ash::{vk::{self, AccessFlags, PipelineStageFlags, ImageMemoryBarrier, CommandBuffer}, Device};

impl VulkanApp{

    // (src_access_mask, dst_access_mask, src_stage, dst_stage)
    fn get_transition_image_layout(
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> (AccessFlags, AccessFlags, PipelineStageFlags, PipelineStageFlags) {
        match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            (
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            (
                vk::ImageLayout::UNDEFINED, 
                vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            ),
            (   
                vk::ImageLayout::UNDEFINED, 
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
            ) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::COLOR_ATTACHMENT_READ
                    | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ),
            (
                vk::ImageLayout::UNDEFINED, 
                vk::ImageLayout::PRESENT_SRC_KHR
            ) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::TOP_OF_PIPE,
            ),
            (
                vk::ImageLayout::PRESENT_SRC_KHR, 
                vk::ImageLayout::GENERAL
            ) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::TOP_OF_PIPE,
            ),
            _ => panic!(
                "Unsupported layout transition({:?} => {:?}).",
                old_layout, new_layout
            ),
        }
    }

    fn get_transition_image_layout_barrier (
        device: &Device,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_access_mask: AccessFlags,
        dst_access_mask: AccessFlags,
    ) -> ImageMemoryBarrier {
        
        let aspect_mask = if new_layout == vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL {
            let mut mask = vk::ImageAspectFlags::DEPTH;
            if Self::has_stencil_component(format) {
                mask |= vk::ImageAspectFlags::STENCIL;
            }
            mask
        } else {
            vk::ImageAspectFlags::COLOR
        };

        vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: vk::REMAINING_MIP_LEVELS,
                base_array_layer: 0,
                layer_count: vk::REMAINING_ARRAY_LAYERS,
            })
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .build()
    }

    pub fn transition_image_layout_one_time (
        device: &Device,
        command_pool: &vk::CommandPool,
        transition_queue: &vk::Queue,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let (src_access_mask, dst_access_mask, src_stage, dst_stage) = 
            Self::get_transition_image_layout(old_layout, new_layout);

        let barrier = Self::get_transition_image_layout_barrier(device, image, format, old_layout, new_layout, src_access_mask, dst_access_mask);

        Self::execute_one_time_commands(device, command_pool, transition_queue, |buffer| {
            unsafe {
                device.cmd_pipeline_barrier(
                    buffer,
                    src_stage,
                    dst_stage,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier],
                )
            };
        });
    }

    pub fn transition_image_layout_with_command_buffer (
        device: &Device,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        buffer: CommandBuffer,
    ) {
        let (src_access_mask, dst_access_mask, src_stage, dst_stage) = 
            Self::get_transition_image_layout(old_layout, new_layout);

        let barrier = Self::get_transition_image_layout_barrier(device, image, format, old_layout, new_layout, src_access_mask, dst_access_mask);

        unsafe {
            device.cmd_pipeline_barrier(
                buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            )
        };
    }
}