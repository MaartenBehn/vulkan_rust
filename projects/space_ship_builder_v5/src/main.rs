use std::time::Duration;

use crate::builder::Builder;
use crate::rules::Rules;
use crate::ship_renderer::ShipRenderer;
use crate::voxel_loader::VoxelLoader;
use octa_force::glam::{ivec2, uvec2, IVec2, IVec3, UVec3};
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
use crate::debug::{DebugController, DebugMode::OFF};
use crate::ship::{Ship, CHUNK_SIZE};
use crate::ship_save::ShipSave;

pub mod builder;
#[cfg(debug_assertions)]
pub mod debug;
pub mod math;
pub mod node;
pub mod rotation;
pub mod rules;
pub mod ship;
pub mod ship_mesh;
pub mod ship_renderer;
mod ship_save;
pub mod voxel_loader;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Space ship builder";
const INPUT_INTERVALL: Duration = Duration::from_secs(1);

const VOX_FILE_PATH: &str = "./assets/space_ship.vox";
const SHIP_SAVE_FILE_PATH: &str = "./assets/ship.bin";

fn main() -> Result<()> {
    octa_force::run::<SpaceShipBuilder>(APP_NAME, uvec2(WIDTH, HEIGHT), false)
}

struct SpaceShipBuilder {
    total_time: Duration,
    last_input: Duration,

    voxel_loader: VoxelLoader,
    rules: Rules,
    builder: Builder,
    renderer: ShipRenderer,
    camera: Camera,

    #[cfg(debug_assertions)]
    debug_controller: DebugController,
}

impl App for SpaceShipBuilder {
    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let voxel_loader = VoxelLoader::new(VOX_FILE_PATH)?;

        let rules = Rules::new(&voxel_loader);

        let ship_save = ShipSave::load(SHIP_SAVE_FILE_PATH);
        let ship: Ship = if ship_save.is_ok() {
            ship_save.unwrap().into()
        } else {
            let mut ship = Ship::new(CHUNK_SIZE);
            ship.add_chunk(IVec3::ZERO);
            ship
        };

        let builder = Builder::new(ship, base.num_frames, &voxel_loader)?;

        let renderer = ShipRenderer::new(
            &base.context,
            base.num_frames as u32,
            base.swapchain.format,
            Format::D32_SFLOAT,
            base.swapchain.extent,
            &voxel_loader,
        )?;

        #[cfg(debug_assertions)]
        let debug_controller = DebugController::new(
            &base.context,
            base.num_frames,
            base.swapchain.format,
            &base.window,
            &builder.ship,
            &renderer,
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
            last_input: Duration::ZERO,

            voxel_loader,
            rules,
            builder,
            renderer,
            camera,

            #[cfg(debug_assertions)]
            debug_controller,
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

        if base.controls.q && self.last_input + INPUT_INTERVALL < self.total_time {
            self.last_input = self.total_time;

            log::info!("reloading .vox File");
            let voxel_loader = VoxelLoader::new("./assets/space_ship.vox")?;
            self.rules = Rules::new(&voxel_loader);

            self.builder.on_rules_changed()?;
            self.builder.ship.recompute();
            self.renderer
                .on_rules_changed(&self.voxel_loader, &base.context, base.num_frames)?;

            log::info!(".vox File loaded");
        }

        if base.controls.f12 && self.last_input + INPUT_INTERVALL < self.total_time {
            self.last_input = self.total_time;

            log::info!("saving Ship");
            let ship_save = ShipSave::from(&self.builder.ship);
            ship_save.save(SHIP_SAVE_FILE_PATH)?;

            log::info!("saved Ship");
        }

        self.builder.update(
            image_index,
            &base.context,
            &self.renderer.chunk_descriptor_layout,
            &self.renderer.descriptor_pool,
            &base.controls,
            &self.camera,
            &self.rules,
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
                &self.rules,
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

        #[cfg(not(debug_assertions))]
        self.renderer.render(buffer, image_index, &self.builder);

        #[cfg(debug_assertions)]
        {
            if self.debug_controller.mode == OFF {
                self.renderer.render(buffer, image_index, &self.builder);
            }

            self.debug_controller.render(
                buffer,
                image_index,
                &self.camera,
                base.swapchain.extent,
                &self.renderer,
            )?;
        }

        buffer.end_rendering();

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &mut BaseApp<Self>) -> Result<()> {
        self.renderer
            .on_recreate_swapchain(&base.context, base.swapchain.extent)?;

        Ok(())
    }
}
