use std::time::Duration;

use app::glam::{ivec2, uvec2, IVec2};
use app::imgui::{Condition, Ui};
use app::vulkan::{
    ash::vk::{self, Format},
    CommandBuffer,
};
use app::{
    anyhow::Result,
    camera::Camera,
    controls::Controls,
    glam::{uvec3, vec3, Vec3},
};
use app::{log, App, BaseApp};

#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugLineRenderer};

use crate::debug::DebugTextRenderer;
use crate::{
    builder::Builder, node::NodeController, renderer::Renderer, ship::Ship,
    voxel_loader::VoxelLoader,
};

pub mod builder;

#[cfg(debug_assertions)]
pub mod debug;
pub mod math;
pub mod node;
pub mod renderer;
pub mod rotation;
pub mod ship;
pub mod ship_mesh;
pub mod voxel_loader;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Space ship builder";
const VOX_FILE_RELODE_INTERVALL: Duration = Duration::from_secs(1);

fn main() -> Result<()> {
    app::run::<SpaceShipBuilder>(APP_NAME, WIDTH, HEIGHT, false, false)
}
struct SpaceShipBuilder {
    total_time: Duration,
    last_vox_reloade: Duration,

    node_controller: NodeController,
    builder: Builder,
    renderer: Renderer,

    #[cfg(debug_assertions)]
    debug_controller: DebugController,
    camera: Camera,
}

impl App for SpaceShipBuilder {
    type Gui = ();

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        //Rot::print_rot_permutations();

        let voxel_loader = VoxelLoader::new("./assets/models/space_ship_v3.vox")?;

        let node_controller =
            NodeController::new(voxel_loader, "./assets/models/space_ship_config_v3.json")?;

        let ship_size = uvec3(10, 10, 10);
        let ship = Ship::new(ship_size, context, &node_controller)?;

        let builder = Builder::new(ship, context, &node_controller)?;

        let renderer = Renderer::new(
            context,
            &node_controller,
            base.swapchain.images.len() as u32,
            base.swapchain.format,
            Format::D32_SFLOAT,
            base.swapchain.extent,
        )?;

        #[cfg(debug_assertions)]
        let debug_line_renderer = DebugLineRenderer::new(
            1000000,
            context,
            base.swapchain.images.len() as u32,
            base.swapchain.format,
            Format::D32_SFLOAT,
            &renderer,
        )?;

        #[cfg(debug_assertions)]
        let debug_text_renderer = DebugTextRenderer::new(
            context,
            &base.command_pool,
            base.swapchain.format,
            base.swapchain.images.len(),
            uvec2(base.swapchain.extent.width, base.swapchain.extent.height),
        )?;

        #[cfg(debug_assertions)]
        let debug_controller = DebugController::new(debug_line_renderer, debug_text_renderer)?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.extent);

        camera.position = Vec3::new(0.5, -2.0, 3.0);
        camera.direction = Vec3::new(0.0, 1.0, -1.0).normalize();
        camera.speed = 2.0;
        camera.z_far = 100.0;
        camera.up = vec3(0.0, 0.0, 1.0);

        Ok(Self {
            total_time: Duration::ZERO,
            last_vox_reloade: Duration::ZERO,

            node_controller,
            builder,
            renderer,

            #[cfg(debug_assertions)]
            debug_controller,

            camera,
        })
    }

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        _: &mut <Self as App>::Gui,
        _: usize,
        delta_time: Duration,
        controls: &Controls,
    ) -> Result<()> {
        self.total_time += delta_time;

        self.camera.update(controls, delta_time);

        if controls.q && self.last_vox_reloade + VOX_FILE_RELODE_INTERVALL < self.total_time {
            self.last_vox_reloade = self.total_time;

            log::info!("reloading .vox File");
            let voxel_loader = VoxelLoader::new("./assets/models/space_ship_v3.vox")?;
            self.node_controller.load(voxel_loader)?;

            self.builder
                .on_node_controller_change(&self.node_controller)?;

            self.renderer = Renderer::new(
                &base.context,
                &self.node_controller,
                base.swapchain.images.len() as u32,
                base.swapchain.format,
                Format::D32_SFLOAT,
                base.swapchain.extent,
            )?;
            log::info!(".vox File loaded");
        }

        self.builder.update(
            controls,
            &self.camera,
            &self.node_controller,
            delta_time,
            self.total_time,
            #[cfg(debug_assertions)]
            &mut self.debug_controller,
        )?;

        self.renderer.update(&self.camera, base.swapchain.extent)?;

        #[cfg(debug_assertions)]
        self.debug_controller
            .update(controls, self.total_time, delta_time)?;

        Ok(())
    }

    fn record_raster_commands(
        &mut self,
        base: &BaseApp<Self>,
        buffer: &CommandBuffer,
        image_index: usize,
    ) -> Result<()> {
        buffer.begin_rendering(
            &base.swapchain.views[image_index],
            Some(&self.renderer.depth_image_view),
            base.swapchain.extent,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );
        buffer.set_viewport(base.swapchain.extent);
        buffer.set_scissor(base.swapchain.extent);

        self.renderer.render(buffer, image_index, &self.builder);
        #[cfg(debug_assertions)]
        self.debug_controller
            .render(buffer, image_index, &self.camera)?;

        buffer.end_rendering();

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()> {
        self.renderer
            .on_recreate_swapchain(&base.context, base.swapchain.extent)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Gui {}

impl app::gui::Gui for Gui {
    fn new() -> Result<Self> {
        app::anyhow::Ok(Gui {})
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Test")
            .position([5.0, 150.0], Condition::FirstUseEver)
            .size([300.0, 300.0], Condition::FirstUseEver)
            .resizable(false)
            .movable(false)
            .build(|| {
                ui.text("Hello World");
            });
    }
}
