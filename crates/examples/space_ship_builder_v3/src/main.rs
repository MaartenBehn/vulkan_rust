use std::time::Duration;

use octa_force::glam::{ivec2, uvec2, IVec2, UVec3};
use octa_force::imgui::{Condition, Ui};
use octa_force::vulkan::{
    ash::vk::{self, Format},
    CommandBuffer, Context,
};
use octa_force::{
    anyhow::Result,
    camera::Camera,
    controls::Controls,
    glam::{uvec3, vec3, Vec3},
};
use octa_force::{log, App, BaseApp};

#[cfg(debug_assertions)]
use crate::debug::DebugController;

use crate::debug::DebugMode::OFF;
use crate::{
    builder::Builder, node::NodeController, ship::Ship, ship_renderer::ShipRenderer,
    voxel_loader::VoxelLoader,
};

pub mod builder;

#[cfg(debug_assertions)]
pub mod debug;
pub mod math;
pub mod node;
pub mod rotation;
pub mod ship;
pub mod ship_mesh;
pub mod ship_renderer;
pub mod voxel_loader;
const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Space ship builder";
const VOX_FILE_RELODE_INTERVALL: Duration = Duration::from_secs(1);
fn main() -> Result<()> {
    octa_force::run::<SpaceShipBuilder>(APP_NAME, uvec2(WIDTH, HEIGHT), false)
}

struct SpaceShipBuilder {
    total_time: Duration,
    last_vox_reloade: Duration,

    node_controller: NodeController,
    builder: Builder,
    renderer: ShipRenderer,

    #[cfg(debug_assertions)]
    debug_controller: DebugController,

    camera: Camera,
}

impl App for SpaceShipBuilder {
    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let voxel_loader = VoxelLoader::new("./assets/models/space_ship_v3.vox")?;

        let node_controller =
            NodeController::new(voxel_loader, "./assets/models/space_ship_config_v3.json")?;

        let builder = Builder::new(base.swapchain.images.len(), &node_controller)?;

        let renderer = ShipRenderer::new(
            &base.context,
            &node_controller,
            base.swapchain.images.len() as u32,
            base.swapchain.format,
            Format::D32_SFLOAT,
            base.swapchain.extent,
        )?;

        #[cfg(debug_assertions)]
        let debug_gui_id = base.add_in_world_gui()?;

        #[cfg(debug_assertions)]
        let debug_controller = DebugController::new(
            &base.context,
            base.swapchain.images.len(),
            base.swapchain.format,
            &renderer,
            debug_gui_id,
        )?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.extent);

        camera.position = Vec3::new(1.0, -2.0, 1.0);
        camera.direction = Vec3::new(0.0, 1.0, 0.0).normalize();
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
        image_index: usize,
        delta_time: Duration,
    ) -> Result<()> {
        self.total_time += delta_time;

        self.camera.update(&base.controls, delta_time);

        if base.controls.q && self.last_vox_reloade + VOX_FILE_RELODE_INTERVALL < self.total_time {
            self.last_vox_reloade = self.total_time;

            log::info!("reloading .vox File");
            let voxel_loader = VoxelLoader::new("./assets/models/space_ship_v3.vox")?;
            self.node_controller.load(voxel_loader)?;

            self.builder.on_node_controller_change()?;
            self.builder
                .ship
                .on_node_controller_change(&self.node_controller)?;

            self.renderer = ShipRenderer::new(
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
            image_index,
            &base.context,
            &self.renderer.chunk_descriptor_layout,
            &self.renderer.descriptor_pool,
            &base.controls,
            &self.camera,
            &self.node_controller,
            delta_time,
            self.total_time,
            #[cfg(debug_assertions)]
            &mut self.debug_controller,
        )?;

        self.renderer.update(&self.camera, base.swapchain.extent)?;

        #[cfg(debug_assertions)]
        {
            self.debug_controller.update(
                &base.context,
                &base.controls,
                &self.renderer,
                self.total_time,
                &self.builder.ship,
                image_index,
                &mut base.in_world_guis[self.debug_controller.text_renderer.gui_id],
                &self.node_controller,
            )?;
        }

        Ok(())
    }

    fn record_render_commands(
        &mut self,
        base: &mut BaseApp<Self>,
        image_index: usize,
    ) -> Result<()> {
        let buffer = &base.command_buffers[image_index];

        buffer.swapchain_image_render_barrier(&base.swapchain.images[image_index])?;
        buffer.begin_rendering(
            &base.swapchain.views[image_index],
            Some(&self.renderer.depth_image_view),
            base.swapchain.extent,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );
        buffer.set_viewport(base.swapchain.extent);
        buffer.set_scissor(base.swapchain.extent);

        if self.debug_controller.mode == OFF {
            self.renderer.render(buffer, image_index, &self.builder);
        }

        #[cfg(debug_assertions)]
        self.debug_controller.render(
            buffer,
            image_index,
            &self.camera,
            base.swapchain.extent,
            &mut base.in_world_guis[self.debug_controller.text_renderer.gui_id],
            &self.renderer,
        )?;

        buffer.end_rendering();

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &BaseApp<Self>) -> Result<()> {
        self.renderer
            .on_recreate_swapchain(&base.context, base.swapchain.extent)?;

        Ok(())
    }
}
