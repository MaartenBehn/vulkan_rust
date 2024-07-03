use std::time::Duration;

use crate::rules::Rules;
use crate::voxel_loader::VoxelLoader;
use octa_force::egui_winit::winit::event::WindowEvent;
use octa_force::glam::{ivec2, uvec2, vec2, IVec2, IVec3, UVec3};
use octa_force::vulkan::{
    ash::vk::{self, Format},
    CommandBuffer, Context,
};
use octa_force::{
    anyhow::Result,
    camera::Camera,
    controls::Controls,
    glam::{uvec3, vec3, Vec3},
    EngineConfig, EngineFeatureValue,
};
use octa_force::{log, App, BaseApp};

#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode::OFF};
use crate::ship::renderer::RENDER_MODE_BASE;
use crate::ship::{Ship, ShipManager, CHUNK_SIZE};

#[cfg(debug_assertions)]
pub mod debug;
pub mod math;
pub mod node;
pub mod rotation;
pub mod rules;
pub mod ship;
pub mod voxel_loader;

const WIDTH: u32 = 2200;
const HEIGHT: u32 = 1250;
const APP_NAME: &str = "Space ship builder";
const INPUT_INTERVALL: Duration = Duration::from_secs(1);

const VOX_FILE_PATH: &str = "./assets/space_ship.vox";

fn main() -> Result<()> {
    octa_force::run::<SpaceShipBuilder>(EngineConfig {
        name: APP_NAME.to_string(),
        start_size: uvec2(WIDTH, HEIGHT),
        ray_tracing: EngineFeatureValue::NotUsed,
        validation_layers: EngineFeatureValue::Needed,
        shader_debug_printing: EngineFeatureValue::Needed,
    })
}

struct SpaceShipBuilder {
    total_time: Duration,
    last_input: Duration,

    voxel_loader: VoxelLoader,
    rules: Rules,

    ship_manager: ShipManager,

    camera: Camera,

    #[cfg(debug_assertions)]
    debug_controller: DebugController,
}

impl App for SpaceShipBuilder {
    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let voxel_loader = VoxelLoader::new(VOX_FILE_PATH)?;

        let mut rules = Rules::new(&voxel_loader)?;

        #[cfg(debug_assertions)]
        let test_node_id = rules.load_node("Test", &voxel_loader).unwrap();

        let ship_manager = ShipManager::new(
            &base.context,
            base.swapchain.format,
            Format::D32_SFLOAT,
            base.swapchain.size,
            base.num_frames,
            &rules,
        )?;

        #[cfg(debug_assertions)]
        let debug_controller = DebugController::new(
            &base.context,
            base.num_frames,
            base.swapchain.format,
            base.swapchain.depth_format,
            &base.window,
            test_node_id,
            &ship_manager,
        )?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.size.as_vec2());

        camera.position = Vec3::new(1.0, -2.0, 1.0);
        camera.direction = Vec3::new(0.0, 1.0, 0.0).normalize();
        camera.speed = 4.0;
        camera.z_far = 100.0;
        camera.up = vec3(0.0, 0.0, 1.0);

        Ok(Self {
            total_time: Duration::ZERO,
            last_input: Duration::ZERO,

            voxel_loader,
            rules,
            ship_manager,
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
            self.voxel_loader.reload()?;
            self.rules = Rules::new(&self.voxel_loader)?;

            self.ship_manager
                .on_voxel_change(&base.context, base.num_frames, &mut self.rules)?;

            log::info!(".vox File loaded");
        }

        #[cfg(debug_assertions)]
        {
            if self.debug_controller.mode == OFF {
                self.ship_manager.update(
                    &mut self.rules,
                    self.total_time,
                    delta_time,
                    image_index,
                    &base.context,
                    &base.controls,
                    &self.camera,
                    base.swapchain.size,
                )?;
            }

            self.debug_controller.update(
                &base.context,
                &base.controls,
                &mut self.voxel_loader,
                self.total_time,
                &mut self.ship_manager,
                image_index,
                &self.rules,
                &self.camera,
                base.swapchain.size,
            )?;
        }

        #[cfg(not(debug_assertions))]
        self.ship_manager.update(
            &mut self.rules,
            self.total_time,
            delta_time,
            image_index,
            &base.context,
            &base.controls,
            &self.camera,
            base.swapchain.extent,
        )?;

        Ok(())
    }

    fn on_window_event(&mut self, base: &mut BaseApp<Self>, event: &WindowEvent) -> Result<()> {
        #[cfg(debug_assertions)]
        self.debug_controller.on_event(&base.window, event);

        Ok(())
    }

    fn record_render_commands(
        &mut self,
        base: &mut BaseApp<Self>,
        image_index: usize,
    ) -> Result<()> {
        let buffer = &base.command_buffers[image_index];

        buffer
            .swapchain_image_render_barrier(&base.swapchain.images_and_views[image_index].image)?;
        buffer.begin_rendering(
            &base.swapchain.images_and_views[image_index].view,
            &self.ship_manager.renderer.depth_image_view,
            base.swapchain.size,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );

        buffer.set_viewport_size(base.swapchain.size.as_vec2());
        buffer.set_scissor_size(base.swapchain.size.as_vec2());

        #[cfg(not(debug_assertions))]
        self.ship_manager.render(buffer, image_index);

        #[cfg(debug_assertions)]
        {
            if self.debug_controller.mode == OFF {
                self.ship_manager.render(buffer, image_index);
            }

            self.debug_controller.render(
                &base.context,
                &base.window,
                buffer,
                image_index,
                base.swapchain.size,
                &self.camera,
                &self.ship_manager.renderer,
            )?;
        }

        buffer.end_rendering();

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &mut BaseApp<Self>) -> Result<()> {
        self.ship_manager
            .renderer
            .on_recreate_swapchain(&base.context, base.swapchain.size)?;

        Ok(())
    }
}
