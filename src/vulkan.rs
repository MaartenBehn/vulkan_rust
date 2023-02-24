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
mod transform;
mod world;
mod mesh;

use crate::{vulkan::{context::VkContext, debug::*, swapchain::*, texture::Texture, camera::Camera, mesh::Mesh}};

use ash::extensions::khr::{Surface, Swapchain};
use ash::{vk, Entry};
use imgui::*;
use imgui_rs_vulkan_renderer::*;
use imgui_winit_support::{WinitPlatform, HiDpiMode};
use winit::window::Window;

use self::{device::QueueFamiliesIndices, sync::InFlightFrames};

const FRAMES_IN_FLIGHT: u32 = 2;

pub struct VulkanApp {
    vk_context: VkContext,
    pub setup: Vulkan_Setup,
    size_dependent: Size_Dependent,

    pub is_left_clicked: bool,
    pub cursor_position: [i32; 2],
    pub cursor_delta: Option<[i32; 2]>,
    pub wheel_delta: Option<f32>,
    pub keys_pressed: [bool; 255]
}

pub struct Vulkan_Setup {
    queue_families_indices: QueueFamiliesIndices,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    command_pool: vk::CommandPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    transient_command_pool: vk::CommandPool,
    msaa_samples: vk::SampleCountFlags,
    depth_format: vk::Format,
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

    pub camera: Camera,
    pub imgui: Context,
    pub platform: WinitPlatform
}

pub struct Size_Dependent {
    swapchain: Swapchain,
    swapchain_khr: vk::SwapchainKHR,
    swapchain_properties: SwapchainProperties,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
    color_texture: Texture,
    depth_texture: Texture,
    
    renderer: Renderer,
}

impl VulkanApp {
    pub fn new(window: &Window, dimensions: [u32; 2]) -> Self {


        log::info!("Creating application.");
        let entry = unsafe { Entry::load().unwrap() };
        let instance = Self::create_instance(&entry, window);

        let surface = Surface::new(&entry, &instance);
        let surface_khr =
            unsafe { ash_window::create_surface(&entry, &instance, window, None).unwrap() };

        let debug_report_callback = setup_debug_messenger(&entry, &instance);

        let (physical_device, queue_families_indices) = Self::pick_physical_device(&instance, &surface, surface_khr);

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

        log::info!("Context done.");

       
        log::info!("Command Pools");
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

        let command_buffers = Self::create_and_register_command_buffers(
            vk_context.device(),
            &command_pool,
        );


        log::info!("Image Formats");
        let depth_format = Self::find_depth_format(&vk_context);
        let msaa_samples = vk_context.get_max_usable_sample_count();


        log::info!("Loading Mesh.");
        let mut mesh = Self::get_world_mesh();
        let model_index_count = mesh.get_indices().len();

        let (vertex_buffer, vertex_buffer_memory) = Self::create_vertex_buffer(
            &vk_context,
            transient_command_pool,
            graphics_queue,
            mesh.get_transformed_vertices(),
        );
        let (index_buffer, index_buffer_memory) = Self::create_index_buffer(
            &vk_context,
            transient_command_pool,
            graphics_queue,
            mesh.get_indices(),
        );
        

        log::info!("Creating Uniform Buffers.");
        let (uniform_buffers, uniform_buffer_memories) =
            Self::create_uniform_buffers(&vk_context, (FRAMES_IN_FLIGHT + 1) as usize);


        log::info!("Creating Descriptors.");
        let descriptor_set_layout = Self::create_descriptor_set_layout(vk_context.device());
        let descriptor_pool = Self::create_descriptor_pool(vk_context.device(), FRAMES_IN_FLIGHT + 1);
        let descriptor_sets = Self::create_descriptor_sets(
            vk_context.device(),
            descriptor_pool,
            descriptor_set_layout,
            &uniform_buffers,
        );



        let in_flight_frames = Self::create_sync_objects(vk_context.device());


        info!("imgui");
        let mut imgui = Context::create();
        imgui.set_ini_filename(None);
         
        let mut platform = WinitPlatform::init(&mut imgui);

        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: include_bytes!("../assets/fonts/mplus-1p-regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.75,
                    glyph_ranges: FontGlyphRanges::japanese(),
                    ..FontConfig::default()
                }),
            },
        ]);
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);

        // Generate UI
        platform
            .prepare_frame(imgui.io_mut(), &window)
            .expect("Failed to prepare frame");
        info!("Context done");


        let mut setup = Vulkan_Setup{
            queue_families_indices,
            graphics_queue,
            present_queue,

            command_pool,
            transient_command_pool,
            command_buffers,

            depth_format,
            msaa_samples,

            model_index_count,
            vertex_buffer_memory,
            vertex_buffer,
            index_buffer_memory,
            index_buffer,

            uniform_buffer_memories,
            uniform_buffers,

            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,

            in_flight_frames,

            platform,
            imgui,

            camera: Default::default(),
        };

        let size_dependent = Self::create_size_dependent(&vk_context, &mut setup, dimensions, window);
        

        Self {
            vk_context,
            setup,
            size_dependent,

            is_left_clicked: false,
            cursor_position: [0, 0],
            cursor_delta: None,
            wheel_delta: None,
            keys_pressed: [false; 255],
        }
    }

    fn create_size_dependent(
        vk_context: &VkContext, 
        setup: &mut Vulkan_Setup,
        dimensions: [u32; 2],
        window: &Window
    ) -> Size_Dependent {

        let (swapchain, swapchain_khr, swapchain_properties, images) =
            Self::create_swapchain_and_images(&vk_context, setup.queue_families_indices, dimensions);
        let swapchain_image_views =
            Self::create_swapchain_image_views(vk_context.device(), &images, swapchain_properties);

        

        let render_pass =
            Self::create_render_pass(vk_context.device(), swapchain_properties, setup.msaa_samples, setup.depth_format);
        
        let (pipeline, pipeline_layout) = Self::create_pipeline(
            vk_context.device(),
            swapchain_properties,
            setup.msaa_samples,
            render_pass,
            setup.descriptor_set_layout,
        );

        let color_texture = Self::create_color_texture(
            &vk_context,
            setup.command_pool,
            setup.graphics_queue,
            swapchain_properties,
            setup.msaa_samples,
        );

        let depth_texture = Self::create_depth_texture(
            &vk_context,
            setup.command_pool,
            setup.graphics_queue,
            setup.depth_format,
            swapchain_properties.extent,
            setup.msaa_samples,
        );

        let swapchain_framebuffers = Self::create_framebuffers(
            vk_context.device(),
            &swapchain_image_views,
            color_texture,
            depth_texture,
            render_pass,
            swapchain_properties,
        );

        info!("imgui renderer");
        let renderer = Renderer::with_default_allocator(
            &vk_context.instance(),
            vk_context.physical_device(),
            vk_context.device().clone(),
            setup.graphics_queue,
            setup.command_pool,
            render_pass,
            &mut setup.imgui,
            Some(Options {
                in_flight_frames: FRAMES_IN_FLIGHT as usize,
                ..Default::default()
            }),
        ).unwrap();

        Size_Dependent{
            swapchain,
            swapchain_khr,
            swapchain_properties,
            swapchain_image_views,
            render_pass,
            pipeline_layout,
            pipeline,
            swapchain_framebuffers,
            color_texture,
            depth_texture,
            renderer,
        }
    }

    


    pub fn draw_frame(&mut self, window: &Window, fps: f64) -> bool {
        let sync_objects = self.setup.in_flight_frames.next().unwrap();
        let image_available_semaphore = sync_objects.image_available_semaphore;
        let render_finished_semaphore = sync_objects.render_finished_semaphore;
        let in_flight_fence = sync_objects.fence;
        let wait_fences = [in_flight_fence];
        let device = self.vk_context.device();

        let ui = self.setup.imgui.frame();

        ui.window("Debug")
            .position([10.0, 10.0], Condition::Always)
            .size([200.0, 100.0], Condition::FirstUseEver)
            .build(|| {
                ui.text_wrapped(format!("FPS: {:.1}", fps));
                let mouse_pos = ui.io().mouse_pos;
                ui.text(format!(
                    "Mouse Position: ({:.1},{:.1})",
                    mouse_pos[0], mouse_pos[1]
                ));
            });

        self.setup.platform.prepare_render(&ui, &window);
        let draw_data = self.setup.imgui.render();
        
        unsafe {
            self.vk_context
                .device()
                .wait_for_fences(&wait_fences, true, std::u64::MAX)
                .unwrap()
        };

        let result = unsafe {
            self.size_dependent.swapchain.acquire_next_image(
                self.size_dependent.swapchain_khr,
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

        let command_buffer = self.setup.command_buffers[image_index as usize];

        Self::updating_command_buffer(
            image_index as usize,
            &command_buffer,
            device,
            &self.size_dependent.swapchain_framebuffers,
            self.size_dependent.render_pass,
            self.size_dependent.swapchain_properties,
            self.setup.vertex_buffer,
            self.setup.index_buffer,
            self.setup.model_index_count,
            self.size_dependent.pipeline_layout,
            &self.setup.descriptor_sets,
            self.size_dependent.pipeline,
            &mut self.size_dependent.renderer,
            draw_data,
        );

        Self::update_uniform_buffers(
            image_index,
            &self.size_dependent.swapchain_properties.extent,
            &mut self.setup.camera,
            &self.setup.uniform_buffer_memories,
            device,
        );

        let wait_semaphores = [image_available_semaphore];
        let signal_semaphores = [render_finished_semaphore];

        // Submit command buffer
        {
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = [self.setup.command_buffers[image_index as usize]];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores)
                .build();
            let submit_infos = [submit_info];
            unsafe {
                device
                    .queue_submit(self.setup.graphics_queue, &submit_infos, in_flight_fence)
                    .unwrap()
            };
        }

        let swapchains = [self.size_dependent.swapchain_khr];
        let images_indices = [image_index];

        {
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&images_indices)
                // .results() null since we only have one swapchain
                .build();
            let result = unsafe {
                self.size_dependent.swapchain
                    .queue_present(self.setup.present_queue, &present_info)
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

    fn cleanup_size_dependent(&mut self){
        let device = self.vk_context.device();
        unsafe {
            self.size_dependent.depth_texture.destroy(device);
            self.size_dependent.color_texture.destroy(device);
            self.size_dependent.swapchain_framebuffers
                .iter()
                .for_each(|f| device.destroy_framebuffer(*f, None));
            device.free_command_buffers(self.setup.command_pool, &self.setup.command_buffers);
            device.destroy_pipeline(self.size_dependent.pipeline, None);
            device.destroy_pipeline_layout(self.size_dependent.pipeline_layout, None);
            device.destroy_render_pass(self.size_dependent.render_pass, None);
            self.size_dependent.swapchain_image_views
                .iter()
                .for_each(|v| device.destroy_image_view(*v, None));
            self.size_dependent.swapchain.destroy_swapchain(self.size_dependent.swapchain_khr, None);
        }
    }

    pub fn recreate_size_dependent(&mut self, dimensions: [u32; 2], window: &Window){
        self.wait_gpu_idle();

        self.cleanup_size_dependent();

        self.size_dependent = Self::create_size_dependent(&self.vk_context, &mut self.setup, dimensions, window)


    }

}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        debug!("Dropping application.");
        self.cleanup_size_dependent();

        let device = self.vk_context.device();
        self.setup.in_flight_frames.destroy(device);
        unsafe {
            device.destroy_descriptor_pool(self.setup.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.setup.descriptor_set_layout, None);
            self.setup.uniform_buffer_memories
                .iter()
                .for_each(|m| device.free_memory(*m, None));
            self.setup.uniform_buffers
                .iter()
                .for_each(|b| device.destroy_buffer(*b, None));
            device.free_memory(self.setup.index_buffer_memory, None);
            device.destroy_buffer(self.setup.index_buffer, None);
            device.destroy_buffer(self.setup.vertex_buffer, None);
            device.free_memory(self.setup.vertex_buffer_memory, None);
            device.destroy_command_pool(self.setup.transient_command_pool, None);
            device.destroy_command_pool(self.setup.command_pool, None);
        }
    }
}