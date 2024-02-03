use std::time::Duration;

use app::anyhow::Result;
use app::camera::Camera;
use app::controls::Controls;
use app::glam::{vec2, vec3, Vec3};
use app::vulkan::ash::vk::{self, Format, ImageUsageFlags};
use app::vulkan::{gpu_allocator, CommandBuffer};
use app::{log, App, BaseApp};
use renderer::RenderBuffer;

use crate::builder::Builder;
use crate::node::NodeController;
use crate::renderer::Renderer;
use crate::ship::Ship;
use crate::voxel_loader::VoxelLoader;

pub mod builder;
pub mod math;
pub mod node;
pub mod pattern_config;
pub mod renderer;
pub mod rotation;
pub mod ship;
pub mod ship_mesh;
pub mod voxel_loader;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Space ship builder";

fn main() -> Result<()> {
    app::run::<SpaceShipBuilder>(APP_NAME, WIDTH, HEIGHT, false, false)
}
struct SpaceShipBuilder {
    total_time: Duration,

    node_controller: NodeController,
    ship: Ship,
    builder: Builder,
    renderer: Renderer,
    camera: Camera,
}

impl App for SpaceShipBuilder {
    type Gui = ();

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        // fastrand::seed(42);

        let voxel_loader = VoxelLoader::new("./assets/models/space_ship_v2.vox".to_owned())?;

        let node_controller =
            NodeController::new(voxel_loader, "./assets/models/space_ship_config.json")?;
        let ship = Ship::new(context, &node_controller)?;

        let builder = Builder::new(context)?;
        let renderer = Renderer::new(
            context,
            &node_controller,
            base.swapchain.images.len() as u32,
            base.swapchain.format,
            Format::D32_SFLOAT,
            base.swapchain.extent,
        )?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.extent);

        camera.position = Vec3::new(5.0, -5.0, 5.0);
        camera.direction = Vec3::new(0.0, 1.0, 0.0).normalize();
        camera.speed = 2.0;
        camera.z_far = 100.0;
        camera.up = vec3(0.0, 0.0, 1.0);

        Ok(Self {
            total_time: Duration::ZERO,

            node_controller,
            ship,
            builder,
            renderer,
            camera,
        })
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()> {
        self.renderer
            .on_recreate_swapchain(&base.context, base.swapchain.extent)?;

        Ok(())
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

        self.renderer
            .on_update(&self.camera, base.swapchain.extent)?;

        self.builder.update(
            controls,
            &self.camera,
            &mut self.ship,
            &self.node_controller,
        )?;

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
            Some(&self.renderer.depth_image_view),
            base.swapchain.extent,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );
        buffer.bind_graphics_pipeline(&self.renderer.pipeline);

        buffer.set_viewport(base.swapchain.extent);
        buffer.set_scissor(base.swapchain.extent);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.renderer.pipeline_layout,
            0,
            &[&self.renderer.descriptor_sets[image_index]],
        );

        self.ship.mesh.render(buffer);
        self.builder.render(buffer);

        buffer.end_rendering();

        Ok(())
    }
}
