use super::{VulkanApp, swapchain::SwapchainProperties, device::*, FRAMES_IN_FLIGHT};

use ash::{vk::{self, CommandBuffer}, Device, };
use imgui::DrawData;
use imgui_rs_vulkan_renderer::Renderer;

impl VulkanApp{

    pub fn create_command_pool(
        device: &Device,
        queue_families_indices: QueueFamiliesIndices,
        create_flags: vk::CommandPoolCreateFlags,
    ) -> vk::CommandPool {
        let command_pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_families_indices.graphics_index)
            .flags(create_flags)
            .build();

        unsafe {
            device
                .create_command_pool(&command_pool_info, None)
                .unwrap()
        }
    }

    /// Create a one time use command buffer and pass it to `executor`.
    pub fn execute_one_time_commands<F: FnOnce(vk::CommandBuffer)>(
        device: &Device,
        command_pool: &vk::CommandPool,
        queue: &vk::Queue,
        executor: F,
    ) {
        let command_buffer = {
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_pool(command_pool.clone())
                .command_buffer_count(1)
                .build();

            unsafe { device.allocate_command_buffers(&alloc_info).unwrap()[0] }
        };
        let command_buffers = [command_buffer];

        // Begin recording
        {
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build();
            unsafe {
                device
                    .begin_command_buffer(command_buffer, &begin_info)
                    .unwrap()
            };
        }

        // Execute user function
        executor(command_buffer);

        // End recording
        unsafe { device.end_command_buffer(command_buffer).unwrap() };

        // Submit and wait
        {
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&command_buffers)
                .build();
            let submit_infos = [submit_info];
            unsafe {
                device
                    .queue_submit(queue.clone(), &submit_infos, vk::Fence::null())
                    .unwrap();
                device.queue_wait_idle(queue.clone()).unwrap();
            };
        }

        // Free
        unsafe { device.free_command_buffers(command_pool.clone(), &command_buffers) };
    }

    /// Find a memory type in `mem_properties` that is suitable
    /// for `requirements` and supports `required_properties`.
    ///
    /// # Returns
    ///
    /// The index of the memory type from `mem_properties`.
    pub fn find_memory_type(
        requirements: vk::MemoryRequirements,
        mem_properties: vk::PhysicalDeviceMemoryProperties,
        required_properties: vk::MemoryPropertyFlags,
    ) -> u32 {
        for i in 0..mem_properties.memory_type_count {
            if requirements.memory_type_bits & (1 << i) != 0
                && mem_properties.memory_types[i as usize]
                    .property_flags
                    .contains(required_properties)
            {
                return i;
            }
        }
        panic!("Failed to find suitable memory type.")
    }

    pub fn create_and_register_command_buffers(
        device: &Device,
        pool: &vk::CommandPool,
    ) -> Vec<vk::CommandBuffer> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(*pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(FRAMES_IN_FLIGHT + 1)
            .build();

        let buffers = unsafe { device.allocate_command_buffers(&allocate_info).unwrap() };

        buffers
    }

    pub fn updating_command_buffer(
        i: usize,
        buffer: &CommandBuffer,  
        device: &Device,
        pool: &vk::CommandPool,
        framebuffers: &[vk::Framebuffer],
        render_pass: vk::RenderPass,
        swapchain_properties: SwapchainProperties,
        vertex_buffer: vk::Buffer,
        index_buffer: vk::Buffer,
        index_count: usize,
        pipeline_layout: vk::PipelineLayout,
        descriptor_sets: &[vk::DescriptorSet],
        graphics_pipeline: vk::Pipeline,
        renderer: &mut Renderer,
        draw_data: &DrawData,
    ){
        let buffer = *buffer;
        let framebuffer = framebuffers[i].clone();

        //unsafe { device.reset_command_pool(pool.clone(), vk::CommandPoolResetFlags::empty()).expect("command pool reset") };

        // begin command buffer
        {
            let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
                //.flags(vk::CommandBufferUsageFlags::)
                //.inheritance_info() null since it's a primary command buffer
                .build();
            unsafe {
                device
                    .begin_command_buffer(buffer, &command_buffer_begin_info)
                    .unwrap()
            };
        }

        // begin render pass
        {
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(render_pass)
                .framebuffer(framebuffer)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: swapchain_properties.extent,
                })
                .clear_values(&clear_values)
                .build();

            unsafe {
                device.cmd_begin_render_pass(
                    buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                )
            };
        }

        // Bind pipeline
        unsafe {
            device.cmd_bind_pipeline(buffer, vk::PipelineBindPoint::GRAPHICS, graphics_pipeline)
        };

        // Bind vertex buffer
        let vertex_buffers = [vertex_buffer];
        let offsets = [0];
        unsafe { device.cmd_bind_vertex_buffers(buffer, 0, &vertex_buffers, &offsets) };

        // Bind index buffer
        unsafe { device.cmd_bind_index_buffer(buffer, index_buffer, 0, vk::IndexType::UINT32) };

        // Bind descriptor set
        unsafe {
            let null = [];
            device.cmd_bind_descriptor_sets(
                buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &descriptor_sets[i..=i],
                &null,
            )
        };

        // Draw
        unsafe { device.cmd_draw_indexed(buffer, index_count as _, 1, 0, 0, 0) };

        renderer.cmd_draw(buffer, draw_data).expect("Imgui render failed");

        // End render pass
        unsafe { device.cmd_end_render_pass(buffer) };

        // End command buffer
        unsafe { device.end_command_buffer(buffer).unwrap() };

    }
}