use std::time::Duration;

use app::anyhow::Result;
use app::glam::vec2;
use app::vulkan::ash::vk;
use app::vulkan::{CommandBuffer};
use app::{App, BaseApp};
use camera::Camera;
use chunk::ChunkController;
use chunk::render::ChunkRenderer;
use debug::render::DebugRenderer;

mod chunk;
mod aabb;
mod camera;
mod debug;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Astrinder";
const ENABLE_DEBUG_RENDER: bool = true;

fn main() -> Result<()> {
    app::run::<Astrinder>(APP_NAME, WIDTH, HEIGHT, false, false)
}
struct Astrinder {
    chunk_controller: ChunkController,
    chunk_renderer: ChunkRenderer,
    camera: Camera,
    debug_renderer: DebugRenderer,
}

impl App for Astrinder {
    type Gui = ();

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let chunk_controller = ChunkController::new();

        let chunk_renderer = ChunkRenderer::new(
            context, 
            base.swapchain.format,
            base.swapchain.images.len() as u32,
            100)?;

        let camera = Camera::new(base.swapchain.extent);

        let debug_renderer = DebugRenderer::new(context, 
            base.swapchain.format,
            base.swapchain.images.len() as u32,
            10000)?;

        Ok(Self {
            chunk_controller,
            chunk_renderer,
            camera,
            debug_renderer,
        })
    }

    fn on_recreate_swapchain(&mut self, _: &BaseApp<Self>) -> Result<()> {
        Ok(())
    }

    fn update(
        &mut self,
        _: &mut BaseApp<Self>,
        _: &mut <Self as App>::Gui,
        _: usize,
        duration: Duration,
    ) -> Result<()> {
        self.chunk_controller.update_physics(duration.as_secs_f32());
        self.chunk_renderer.update(&self.camera, &self.chunk_controller)?;

        if ENABLE_DEBUG_RENDER {
            self.debug_renderer.clear_lines();
            self.chunk_controller.debug_colliders(&mut self.debug_renderer);
            self.chunk_controller.debug_chunk_transforms(&mut self.debug_renderer);
            self.chunk_controller.debug_parts_borders(&mut self.debug_renderer);
            self.chunk_controller.debug_chunk_velocity(&mut self.debug_renderer);

            self.debug_renderer.update(&self.camera)?;
        }

        Ok(())
    }

    fn record_raster_commands(
        &self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        buffer.begin_rendering(
            &base.swapchain.views[image_index],
            base.swapchain.extent,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );
        
        self.chunk_renderer.render(buffer, image_index, base.swapchain.extent);

        if ENABLE_DEBUG_RENDER {
            self.debug_renderer.render(buffer, image_index, base.swapchain.extent);
        }
        
        buffer.end_rendering();

        Ok(())
    }
}




