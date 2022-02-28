mod instance;
mod device;
mod swapchain;
mod context;
mod render_pass;
mod texture;
mod camera;
mod fs;
mod math;
mod debug;
mod descriptor;
mod pipeline;
mod image;
mod shader;
mod framebuffers;
mod buffer;
mod command;
mod sync;
mod vertex;

use crate::{vulkan::{context::VkContext, debug::*, swapchain::*, texture::Texture, camera::Camera}};

use ash::extensions::khr::{Surface, Swapchain};
use ash::{vk, Entry};
use winit::window::Window;

use self::{device::QueueFamiliesIndices, sync::InFlightFrames};

const MAX_FRAMES_IN_FLIGHT: u32 = 2;

pub struct VulkanApp {
    resize_dimensions: Option<[u32; 2]>,

    camera: Camera,
    pub is_left_clicked: bool,
    pub cursor_position: [i32; 2],
    pub cursor_delta: Option<[i32; 2]>,
    pub wheel_delta: Option<f32>,

    vk_context: VkContext,
    queue_families_indices: QueueFamiliesIndices,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: Swapchain,
    swapchain_khr: vk::SwapchainKHR,
    swapchain_properties: SwapchainProperties,
    images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    transient_command_pool: vk::CommandPool,
    msaa_samples: vk::SampleCountFlags,
    color_texture: Texture,
    depth_format: vk::Format,
    depth_texture: Texture,
    model_index_count: usize,
    vertex_buffer: vk::Buffer,
    vertex_buffer_memory: vk::DeviceMemory,
    index_buffer: vk::Buffer,
    index_buffer_memory: vk::DeviceMemory,
    uniform_buffers: Vec<vk::Buffer>,
    uniform_buffer_memories: Vec<vk::DeviceMemory>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    command_buffers: Vec<vk::CommandBuffer>,
    in_flight_frames: InFlightFrames,
}

impl VulkanApp {
    pub fn new(window: &Window, with: u32, height: u32) -> Self {
        log::debug!("Creating application.");

        let entry = unsafe { Entry::new().expect("Failed to create entry.") };
        let instance = Self::create_instance(&entry, window);

        let surface = Surface::new(&entry, &instance);
        let surface_khr =
            unsafe { ash_window::create_surface(&entry, &instance, window, None).unwrap() };

        let debug_report_callback = setup_debug_messenger(&entry, &instance);

        let (physical_device, queue_families_indices) =
            Self::pick_physical_device(&instance, &surface, surface_khr);

        let (device, graphics_queue, present_queue) =
            Self::create_logical_device_with_graphics_queue(
                &instance,
                physical_device,
                queue_families_indices,
            );

        let vk_context = VkContext::new(
            entry,
            instance,
            debug_report_callback,
            surface,
            surface_khr,
            physical_device,
            device,
        );

        let (swapchain, swapchain_khr, properties, images) =
            Self::create_swapchain_and_images(&vk_context, queue_families_indices, [with, height]);
        let swapchain_image_views =
            Self::create_swapchain_image_views(vk_context.device(), &images, properties);

        let msaa_samples = vk_context.get_max_usable_sample_count();
        let depth_format = Self::find_depth_format(&vk_context);

        let render_pass =
            Self::create_render_pass(vk_context.device(), properties, msaa_samples, depth_format);
        let descriptor_set_layout = Self::create_descriptor_set_layout(vk_context.device());
        let (pipeline, layout) = Self::create_pipeline(
            vk_context.device(),
            properties,
            msaa_samples,
            render_pass,
            descriptor_set_layout,
        );

        let command_pool = Self::create_command_pool(
            vk_context.device(),
            queue_families_indices,
            vk::CommandPoolCreateFlags::empty(),
        );
        let transient_command_pool = Self::create_command_pool(
            vk_context.device(),
            queue_families_indices,
            vk::CommandPoolCreateFlags::TRANSIENT,
        );

        let color_texture = Self::create_color_texture(
            &vk_context,
            command_pool,
            graphics_queue,
            properties,
            msaa_samples,
        );

        let depth_texture = Self::create_depth_texture(
            &vk_context,
            command_pool,
            graphics_queue,
            depth_format,
            properties.extent,
            msaa_samples,
        );

        let swapchain_framebuffers = Self::create_framebuffers(
            vk_context.device(),
            &swapchain_image_views,
            color_texture,
            depth_texture,
            render_pass,
            properties,
        );

        let (vertices, indices) = Self::cube_model();
        Self::voronoi_model();

        let (vertex_buffer, vertex_buffer_memory) = Self::create_vertex_buffer(
            &vk_context,
            transient_command_pool,
            graphics_queue,
            &vertices,
        );
        let (index_buffer, index_buffer_memory) = Self::create_index_buffer(
            &vk_context,
            transient_command_pool,
            graphics_queue,
            &indices,
        );
        let (uniform_buffers, uniform_buffer_memories) =
            Self::create_uniform_buffers(&vk_context, images.len());

        let descriptor_pool = Self::create_descriptor_pool(vk_context.device(), images.len() as _);
        let descriptor_sets = Self::create_descriptor_sets(
            vk_context.device(),
            descriptor_pool,
            descriptor_set_layout,
            &uniform_buffers,
        );

        let command_buffers = Self::create_and_register_command_buffers(
            vk_context.device(),
            command_pool,
            &swapchain_framebuffers,
            render_pass,
            properties,
            vertex_buffer,
            index_buffer,
            indices.len(),
            layout,
            &descriptor_sets,
            pipeline,
        );

        let in_flight_frames = Self::create_sync_objects(vk_context.device());

        Self {
            resize_dimensions: None,
            camera: Default::default(),
            is_left_clicked: false,
            cursor_position: [0, 0],
            cursor_delta: None,
            wheel_delta: None,
            vk_context,
            queue_families_indices,
            graphics_queue,
            present_queue,
            swapchain,
            swapchain_khr,
            swapchain_properties: properties,
            images,
            swapchain_image_views,
            render_pass,
            descriptor_set_layout,
            pipeline_layout: layout,
            pipeline,
            swapchain_framebuffers,
            command_pool,
            transient_command_pool,
            msaa_samples,
            color_texture,
            depth_format,
            depth_texture,
            model_index_count: indices.len(),
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            uniform_buffers,
            uniform_buffer_memories,
            descriptor_pool,
            descriptor_sets,
            command_buffers,
            in_flight_frames,
        }
    }


    

    pub fn draw_frame(&mut self) -> bool {
        log::trace!("Drawing frame.");
        let sync_objects = self.in_flight_frames.next().unwrap();
        let image_available_semaphore = sync_objects.image_available_semaphore;
        let render_finished_semaphore = sync_objects.render_finished_semaphore;
        let in_flight_fence = sync_objects.fence;
        let wait_fences = [in_flight_fence];

        unsafe {
            self.vk_context
                .device()
                .wait_for_fences(&wait_fences, true, std::u64::MAX)
                .unwrap()
        };

        let result = unsafe {
            self.swapchain.acquire_next_image(
                self.swapchain_khr,
                std::u64::MAX,
                image_available_semaphore,
                vk::Fence::null(),
            )
        };
        let image_index = match result {
            Ok((image_index, _)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                return true;
            }
            Err(error) => panic!("Error while acquiring next image. Cause: {}", error),
        };

        unsafe { self.vk_context.device().reset_fences(&wait_fences).unwrap() };

        self.update_uniform_buffers(image_index);

        let device = self.vk_context.device();
        let wait_semaphores = [image_available_semaphore];
        let signal_semaphores = [render_finished_semaphore];

        // Submit command buffer
        {
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [self.command_buffers[image_index as usize]];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores)
                .build();
            let submit_infos = [submit_info];
            unsafe {
                device
                    .queue_submit(self.graphics_queue, &submit_infos, in_flight_fence)
                    .unwrap()
            };
        }

        let swapchains = [self.swapchain_khr];
        let images_indices = [image_index];

        {
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&images_indices)
                // .results() null since we only have one swapchain
                .build();
            let result = unsafe {
                self.swapchain
                    .queue_present(self.present_queue, &present_info)
            };
            match result {
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    return true;
                }
                Err(error) => panic!("Failed to present queue. Cause: {}", error),
                _ => {}
            }
        }

        false
    }

}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        log::debug!("Dropping application.");
        self.cleanup_swapchain();

        let device = self.vk_context.device();
        self.in_flight_frames.destroy(device);
        unsafe {
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.uniform_buffer_memories
                .iter()
                .for_each(|m| device.free_memory(*m, None));
            self.uniform_buffers
                .iter()
                .for_each(|b| device.destroy_buffer(*b, None));
            device.free_memory(self.index_buffer_memory, None);
            device.destroy_buffer(self.index_buffer, None);
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_command_pool(self.transient_command_pool, None);
            device.destroy_command_pool(self.command_pool, None);
        }
    }
}