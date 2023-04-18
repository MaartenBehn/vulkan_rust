use std::mem::size_of;
use std::time::Duration;

use app::anyhow::Result;
use app::glam::Vec2;
use app::vulkan::ash::vk;
use app::vulkan::utils::create_gpu_only_buffer_from_data;
use app::vulkan::{
    Buffer, CommandBuffer, Context, GraphicsPipeline, GraphicsPipelineCreateInfo,
    GraphicsShaderCreateInfo, PipelineLayout,
};
use app::{App, BaseApp};
use camera::Camera;
use chunk::ChunkController;
use chunk::render::ChunkRenderer;

mod chunk;
mod aabb;
mod camera;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Astrinder";

fn main() -> Result<()> {
    app::run::<Astrinder>(APP_NAME, WIDTH, HEIGHT, false, false)
}
struct Astrinder {
    chunk_controller: ChunkController,
    chunk_renderer: ChunkRenderer,
    camera: Camera
}

impl App for Astrinder {
    type Gui = ();

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let chunk_controller = ChunkController::new();

        let chunk_renderer = ChunkRenderer::new(
            context, 
            base.swapchain.format,
            base.swapchain.images.len() as u32)?;

        let camera = Camera::new(base.swapchain.extent);

        Ok(Self {
            chunk_controller,
            chunk_renderer,
            camera,
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
        _: Duration,
    ) -> Result<()> {

        self.chunk_renderer.update(&self.camera, &self.chunk_controller)?;

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
        
        buffer.end_rendering();

        Ok(())
    }
}




