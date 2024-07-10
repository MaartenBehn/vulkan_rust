use std::time::Duration;

use crate::rules::Rules;
use octa_force::egui_winit::winit::event::WindowEvent;
use octa_force::vulkan::ash::vk::{self, Format};
use octa_force::{
    anyhow::Result,
    camera::Camera,
    glam::{uvec2, vec3, Vec3},
    EngineConfig, EngineFeatureValue,
};
use octa_force::{log, App, BaseApp};

#[cfg(debug_assertions)]
use crate::debug::{DebugController, DebugMode::Off};
use crate::render::parallax::renderer::ParallaxRenderer;
use crate::render::{RenderFunctions, Renderer};
use crate::world::asteroid::AsteroidManager;
use crate::world::data::voxel_loader::VoxelLoader;
use crate::world::ship::ShipManager;

#[cfg(debug_assertions)]
pub mod debug;
pub mod math;
pub mod render;
pub mod rules;
pub mod world;

const WIDTH: u32 = 2200;
const HEIGHT: u32 = 1250;
const APP_NAME: &str = "Space ship builder";
const INPUT_INTERVALL: Duration = Duration::from_secs(1);

const VOX_FILE_PATH: &str = "./assets/space_ship.vox";

const SHOW_ASTEROID: bool = false;

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

    renderer: Renderer,

    ship_manager: ShipManager,
    asteroid_manager: AsteroidManager,

    camera: Camera,

    #[cfg(debug_assertions)]
    debug_controller: DebugController,
}

impl App for SpaceShipBuilder {
    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let voxel_loader = VoxelLoader::new(VOX_FILE_PATH)?;

        let mut rules = Rules::new(&voxel_loader)?;

        let renderer = Renderer::Parallax(ParallaxRenderer::new(
            &base.context,
            base.num_frames as u32,
            base.swapchain.format,
            Format::D32_SFLOAT,
            base.swapchain.size,
            &rules,
        )?);

        #[cfg(debug_assertions)]
        let test_node_id = rules.load_node("Test", &voxel_loader).unwrap();

        let ship_manager = ShipManager::new(base.num_frames, &rules)?;
        let asteroid_manager = AsteroidManager::new(base.num_frames, &rules);

        #[cfg(debug_assertions)]
        let debug_controller = DebugController::new(
            &base.context,
            base.num_frames,
            base.swapchain.format,
            base.swapchain.depth_format,
            &base.window,
            test_node_id,
            &ship_manager,
            renderer.as_parallax().unwrap(),
        )?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.size.as_vec2());

        camera.position = Vec3::new(1.0, 1.0, 1.0);
        //camera.position = Vec3::new(1.0, -100.0, 1.0);
        camera.direction = Vec3::new(0.0, 1.0, 0.0).normalize();
        camera.speed = 10.0;
        camera.z_far = 100.0;
        camera.up = vec3(0.0, 0.0, 1.0);

        Ok(Self {
            total_time: Duration::ZERO,
            last_input: Duration::ZERO,

            voxel_loader,
            rules,
            renderer,
            ship_manager,
            asteroid_manager,
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
        self.renderer.update(&self.camera, base.swapchain.size)?;

        if base.controls.q && self.last_input + INPUT_INTERVALL < self.total_time {
            self.last_input = self.total_time;

            log::info!("reloading .vox File");
            self.voxel_loader.reload()?;
            self.rules = Rules::new(&self.voxel_loader)?;

            self.renderer
                .on_rules_changed(&mut self.rules, &base.context, base.num_frames)?;
            self.ship_manager.on_voxel_change(&mut self.rules)?;

            log::info!(".vox File loaded");
        }

        #[cfg(debug_assertions)]
        {
            if self.debug_controller.mode == Off {
                if !SHOW_ASTEROID {
                    self.ship_manager.update(
                        &mut self.rules,
                        self.total_time,
                        delta_time,
                        image_index,
                        &base.context,
                        &base.controls,
                        &self.camera,
                        &self.renderer,
                    )?;
                } else {
                    self.asteroid_manager.update(
                        &base.context,
                        image_index,
                        delta_time,
                        &self.rules,
                        &self.renderer,
                    )?;
                }
            }

            self.debug_controller.update(
                &base.context,
                &base.controls,
                self.total_time,
                &mut self.ship_manager,
                image_index,
                &self.rules,
                &self.camera,
                base.swapchain.size,
                self.renderer.as_parallax().unwrap(),
            )?;
        }

        #[cfg(not(debug_assertions))]
        {
            if !SHOW_ASTEROID {
                self.ship_manager.update(
                    &mut self.rules,
                    self.total_time,
                    delta_time,
                    image_index,
                    &base.context,
                    &base.controls,
                    &self.camera,
                    &self.renderer,
                )?;
            } else {
                self.asteroid_manager.update(
                    &base.context,
                    image_index,
                    delta_time,
                    &self.rules,
                    &self.renderer,
                )?;
            }
        }

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
            &base.swapchain.depht_images_and_views[image_index].view,
            base.swapchain.size,
            vk::AttachmentLoadOp::CLEAR,
            None,
        );

        buffer.set_viewport_size(base.swapchain.size.as_vec2());
        buffer.set_scissor_size(base.swapchain.size.as_vec2());

        #[cfg(debug_assertions)]
        {
            if self.debug_controller.mode == Off {
                self.ship_manager
                    .render(buffer, image_index, &self.renderer);
                self.asteroid_manager
                    .render(buffer, image_index, &self.renderer);
            }

            self.debug_controller.render(
                &base.context,
                &base.window,
                buffer,
                image_index,
                base.swapchain.size,
                &self.camera,
                self.renderer.as_parallax().unwrap(),
            )?;
        }

        #[cfg(not(debug_assertions))]
        {
            self.ship_manager
                .render(buffer, image_index, &mut self.renderer);
            self.asteroid_manager
                .render(buffer, image_index, &self.renderer);
        }

        buffer.end_rendering();

        Ok(())
    }

    fn on_recreate_swapchain(&mut self, base: &mut BaseApp<Self>) -> Result<()> {
        self.renderer
            .on_recreate_swapchain(&base.context, base.swapchain.size)?;

        Ok(())
    }
}
