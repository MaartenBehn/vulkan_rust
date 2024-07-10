use crate::render::parallax::mesh::ParallaxMesh;
use crate::render::parallax::renderer::ParallaxRenderer;
use crate::rules::Rules;
use crate::world::block_object::BlockObject;
use enum_as_inner::EnumAsInner;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::glam::UVec2;
use octa_force::vulkan::{CommandBuffer, Context};
use crate::render::raytracer::RaytraceRenderer;

pub mod parallax;
pub mod raytracer;

#[enum_delegate::implement(RenderFunctions)]
#[derive(EnumAsInner)]
pub enum Renderer {
    Parallax(ParallaxRenderer),
    Raytracing(RaytraceRenderer)
}

#[enum_delegate::register]
pub trait RenderFunctions {
    fn update(&mut self, camera: &Camera, res: UVec2) -> Result<()>;
    fn on_recreate_swapchain(&mut self, context: &Context, res: UVec2) -> Result<()>;

    fn render(
        &self,
        buffer: &CommandBuffer,
        image_index: usize,
        render_object: &RenderObject,
    ) -> Result<()>;

    fn on_rules_changed(
        &mut self,
        rules: &Rules,
        context: &Context,
        num_frames: usize,
    ) -> Result<()>;
}

#[enum_delegate::implement(RenderObjectFunctions)]
#[derive(EnumAsInner)]
pub enum RenderObject {
    Parallax(ParallaxMesh),
}

#[enum_delegate::register]
pub trait RenderObjectFunctions {
    fn update_from_block_object(
        &mut self,
        block_object: &BlockObject,
        changed_chunks: Vec<usize>,
        image_index: usize,
        context: &Context,
        renderer: &Renderer,
    ) -> Result<()>;
}
