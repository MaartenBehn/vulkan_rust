use crate::render::compute_raytracing::renderer::ComputeRaytracingRenderer;
use crate::render::parallax::renderer::ParallaxRenderer;
use crate::rules::Rules;
use crate::world::block_object::{BlockChunk, BlockObject, ChunkIndex};
use crate::world::manager::WorldManager;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::glam::{Mat4, UVec2};
use octa_force::run;
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::Format;
use octa_force::vulkan::{CommandBuffer, Context, Swapchain};

pub mod compute_raytracing;
pub mod parallax;
// pub mod native_raytracer;

pub enum ActiveRenderer {
    None,
    Parallax,
    ComputeRaytracer,
    Raytracing,
}

pub struct Renderer {
    pub parallax_renderer: Option<ParallaxRenderer>,
    pub compute_raytracing_renderer: Option<ComputeRaytracingRenderer>,
    pub active_renderer: ActiveRenderer,
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer {
            parallax_renderer: None,
            compute_raytracing_renderer: None,
            active_renderer: ActiveRenderer::None,
        }
    }

    pub fn enable_parallax(
        &mut self,
        context: &Context,
        num_frames: usize,
        color_attachment_format: Format,
        depth_attachment_format: Format,
        rules: &Rules,
    ) -> Result<()> {
        if self.parallax_renderer.is_none() {
            self.parallax_renderer = Some(ParallaxRenderer::new(
                context,
                num_frames,
                color_attachment_format,
                depth_attachment_format,
                rules,
            )?);
        }
        self.active_renderer = ActiveRenderer::Parallax;

        Ok(())
    }

    pub fn enable_compute_raytracer(
        &mut self,
        context: &Context,
        format: Format,
        res: UVec2,
        num_frames: usize,
        rules: &Rules,
    ) -> Result<()> {
        if self.compute_raytracing_renderer.is_none() {
            self.compute_raytracing_renderer = Some(ComputeRaytracingRenderer::new(
                context, format, res, num_frames, rules,
            )?);
        }
        self.active_renderer = ActiveRenderer::ComputeRaytracer;

        Ok(())
    }

    pub fn update(&mut self, camera: &Camera, res: UVec2, frame_index: usize) -> Result<()> {
        match self.active_renderer {
            ActiveRenderer::None => {}
            ActiveRenderer::Parallax => {
                let renderer = self.parallax_renderer.as_mut().unwrap();
                renderer.update(camera, res, frame_index)?;
            }
            ActiveRenderer::ComputeRaytracer => {
                let renderer = self.compute_raytracing_renderer.as_ref().unwrap();
                renderer.update(camera, res)?;
            }
            ActiveRenderer::Raytracing => {}
        }

        Ok(())
    }

    pub fn update_object(
        &mut self,
        object: &mut BlockObject,
        changed_chunks: Vec<ChunkIndex>,
        context: &Context,
        frame_index: usize,
        num_frames: usize,
    ) -> Result<()> {
        match self.active_renderer {
            ActiveRenderer::None => {}
            ActiveRenderer::Parallax => {
                let renderer = self.parallax_renderer.as_mut().unwrap();
                renderer.update_object(object, changed_chunks, context, frame_index, num_frames)?;
            }
            ActiveRenderer::ComputeRaytracer => {
                let renderer = self.compute_raytracing_renderer.as_mut().unwrap();
                renderer.update_object(object, changed_chunks)?;
            }
            ActiveRenderer::Raytracing => {}
        }

        Ok(())
    }

    pub fn render(
        &self,
        buffer: &CommandBuffer,
        frame_index: usize,
        world_manager: &WorldManager,
        swapchain: &Swapchain,
    ) -> Result<()> {
        match self.active_renderer {
            ActiveRenderer::None => {}
            ActiveRenderer::Parallax => {
                let renderer = self.parallax_renderer.as_ref().unwrap();
                renderer.begin_render(buffer, frame_index, swapchain)?;

                for region in world_manager.loaded_regions.iter() {
                    for object in region.loaded_objects.iter() {
                        for chunk in object.chunks.iter() {
                            if chunk.parallax_data.is_none() {
                                continue;
                            }

                            renderer.render_data(
                                buffer,
                                frame_index,
                                chunk.parallax_data.as_ref().unwrap(),
                                &object.transform,
                            );
                        }
                    }
                }

                renderer.end_rendering(buffer);
            }
            ActiveRenderer::ComputeRaytracer => {
                let renderer = self.compute_raytracing_renderer.as_ref().unwrap();
                renderer.render(buffer, frame_index, swapchain)?;
            }
            ActiveRenderer::Raytracing => {}
        }

        Ok(())
    }

    pub fn on_rules_changed(
        &mut self,
        rules: &Rules,
        context: &Context,
        num_frames: usize,
    ) -> Result<()> {
        match self.active_renderer {
            ActiveRenderer::None => {}
            ActiveRenderer::Parallax => {
                let renderer = self.parallax_renderer.as_mut().unwrap();
                renderer.on_rules_changed(rules, context, num_frames)?;
            }
            ActiveRenderer::ComputeRaytracer => {}
            ActiveRenderer::Raytracing => {}
        }

        Ok(())
    }

    pub fn on_recreate_swapchain(
        &mut self,
        context: &Context,
        format: Format,
        num_frames: usize,
        res: UVec2,
    ) -> Result<()> {
        match self.active_renderer {
            ActiveRenderer::None => {}
            ActiveRenderer::Parallax => {}
            ActiveRenderer::ComputeRaytracer => {
                let renderer = self.compute_raytracing_renderer.as_mut().unwrap();
                renderer.on_recreate_swapchain(context, format, num_frames, res)?;
            }
            ActiveRenderer::Raytracing => {}
        }

        Ok(())
    }
}
