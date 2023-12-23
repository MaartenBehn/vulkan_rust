use std::time::Duration;

use app::anyhow::Result;
use app::camera::Camera;
use app::controls::Controls;
use app::glam::Vec3;
use app::vulkan::ash::vk;
use app::vulkan::CommandBuffer;
use app::{log, App, BaseApp};
use mesh::Mesh;
use renderer::{RenderBuffer, Renderer};

use crate::rule::RuleSet;
use crate::ship::Ship;

pub mod math;
pub mod mesh;
pub mod renderer;
pub mod rule;
pub mod ship;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "Space ship builder";

fn main() -> Result<()> {
    app::run::<SpaceShipBuilder>(APP_NAME, WIDTH, HEIGHT, false, false)
}
struct SpaceShipBuilder {
    mesh: Mesh,
    renderer: Renderer,
    camera: Camera,
}

impl App for SpaceShipBuilder {
    type Gui = ();

    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let ruleset = RuleSet::new();
        let ship = Ship::new(&ruleset)?;

        let mesh = Mesh::from_ship(&ship)?;

        let renderer = Renderer::new(
            context,
            base.swapchain.images.len() as u32,
            base.swapchain.format,
            &mesh,
        )?;

        log::info!("Creating Camera");
        let mut camera = Camera::base(base.swapchain.extent);

        camera.position = Vec3::new(0.0, 0.0, -5.0);
        camera.direction = Vec3::new(0.0, 0.0, 1.0).normalize();
        camera.speed = 2.0;
        camera.z_far = 100.0;

        Ok(Self {
            mesh,
            renderer,
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
        delta_time: Duration,
        controls: &Controls,
    ) -> Result<()> {
        self.camera.update(controls, delta_time);

        self.renderer
            .render_buffer
            .copy_data_to_buffer(&[RenderBuffer {
                view_proj_matrix: self.camera.projection_matrix() * self.camera.view_matrix(),
            }])?;

        self.renderer
            .vertex_buffer
            .copy_data_to_buffer(self.mesh.vertecies.as_slice())?;

        self.renderer
            .index_buffer
            .copy_data_to_buffer(self.mesh.indecies.as_slice())?;

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
        buffer.bind_graphics_pipeline(&self.renderer.pipeline);

        buffer.bind_vertex_buffer(&self.renderer.vertex_buffer);
        buffer.bind_index_buffer(&self.renderer.index_buffer);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.renderer.pipeline_layout,
            0,
            &[&self.renderer.descriptor_sets[image_index]],
        );

        buffer.set_viewport(base.swapchain.extent);
        buffer.set_scissor(base.swapchain.extent);

        buffer.draw_indexed(self.mesh.indecies.len() as u32);
        buffer.end_rendering();

        Ok(())
    }
}
