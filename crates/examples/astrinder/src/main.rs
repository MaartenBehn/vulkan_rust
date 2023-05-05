use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use app::anyhow::Result;
use app::vulkan::ash::vk;
use app::vulkan::CommandBuffer;
use app::{App, BaseApp};

use camera::Camera;
use chunk::ChunkController;
use debug::render::DebugRenderer;
use render::ChunkRenderer;
use settings::Settings;

mod aabb;
mod camera;
mod chunk;
mod debug;
mod math;
mod render;
mod settings;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Astrinder";
const ENABLE_DEBUG_RENDER: bool = true;

fn main() -> Result<()> {
    app::run::<Astrinder>(APP_NAME, WIDTH, HEIGHT, false, false)
}
struct Astrinder {
    chunk_renderer: ChunkRenderer,
    camera: Camera,
    debug_renderer: DebugRenderer,

    chunk_controller_handle: JoinHandle<()>,
}

impl App for Astrinder {
    type Gui = ();

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let settings = Settings::default();

        let (transform_sender, transform_reciver) = mpsc::channel();
        let (particle_sender, particle_reciver) = mpsc::channel();
        let (debug_sender, debug_reciver) = mpsc::channel();

        let chunk_renderer = ChunkRenderer::new(
            context,
            base.swapchain.format,
            base.swapchain.images.len() as u32,
            transform_reciver,
            particle_reciver,
            settings,
        )?;

        let debug_renderer = DebugRenderer::new(
            context,
            base.swapchain.format,
            base.swapchain.images.len() as u32,
            debug_reciver,
            settings,
        )?;

        let chunk_controller_handle = thread::spawn(move || {
            let mut chunk_controller =
                ChunkController::new(transform_sender, particle_sender, debug_sender, settings);
            chunk_controller.run(settings);
        });

        let camera = Camera::new(base.swapchain.extent);

        Ok(Self {
            chunk_renderer,
            camera,
            debug_renderer,
            chunk_controller_handle,
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
        self.chunk_renderer.recive_parts()?;
        self.chunk_renderer.upload(&self.camera)?;

        if ENABLE_DEBUG_RENDER && cfg!(debug_assertions) {
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

        self.chunk_renderer
            .render(buffer, image_index, base.swapchain.extent);

        if ENABLE_DEBUG_RENDER && cfg!(debug_assertions) {
            self.debug_renderer
                .render(buffer, image_index, base.swapchain.extent);
        }

        buffer.end_rendering();

        Ok(())
    }
}
