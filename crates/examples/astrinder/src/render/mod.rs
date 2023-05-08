use std::mem::align_of;
use std::sync::mpsc::Receiver;

use app::anyhow::*;
use app::vulkan::ash::vk::Extent2D;
use app::vulkan::CommandBuffer;
use app::vulkan::{ash::vk, Context};

use crate::camera::Camera;
use crate::chunk::CHUNK_PART_SIZE;
use crate::math::transform::Transform;
use crate::settings::Settings;

use self::part::RenderParticle;
use self::vulkan::RenderUBO;
use self::{part::RenderPart, vulkan::ChunkRendererVulkan};

pub mod part;
pub mod vulkan;

pub struct ChunkRenderer {
    rendered_parts: usize,
    pub vulkan: ChunkRendererVulkan,

    from_controller_transforms: Receiver<(usize, Transform)>,
    from_controller_particles: Receiver<(
        usize,
        [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    )>,
}

impl ChunkRenderer {
    pub fn new(
        context: &Context,
        color_attachment_format: vk::Format,
        images_len: u32,
        from_controller_transforms: Receiver<(usize, Transform)>,
        from_controller_particles: Receiver<(
            usize,
            [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
        )>,
        settings: Settings,
    ) -> Result<Self> {
        let rendered_parts = settings.max_rendered_parts;

        let mut parts = Vec::new();
        parts.resize(settings.max_rendered_parts, RenderPart::default());

        Ok(Self {
            rendered_parts,
            vulkan: ChunkRendererVulkan::new(
                context,
                color_attachment_format,
                images_len,
                rendered_parts,
            )?,

            from_controller_transforms,
            from_controller_particles,
        })
    }

    pub fn recive_parts(&mut self) -> Result<()> {
        loop {
            let data = self.from_controller_transforms.try_recv();
            if data.is_err() {
                break;
            }

            let (id, transform) = data.unwrap();
            self.vulkan
                .part_ubo
                .copy_data_to_buffer_complex(&[transform], id, 16)?;
        }

        loop {
            let data = self.from_controller_particles.try_recv();
            if data.is_err() {
                break;
            }

            let (id, particles) = data.unwrap();
            self.vulkan.particles_ssbo.copy_data_to_buffer_complex(
                &particles,
                id * (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize,
                align_of::<RenderParticle>(),
            )?;
        }

        Ok(())
    }

    pub fn upload(&mut self, camera: &Camera) -> Result<()> {
        self.vulkan.render_ubo.copy_data_to_buffer_complex(
            &[RenderUBO::new(camera.to_owned())],
            0,
            16,
        )?;

        Ok(())
    }

    pub fn render(&self, buffer: &CommandBuffer, image_index: usize, extent: Extent2D) {
        self.vulkan
            .render(buffer, image_index, extent, self.rendered_parts as u32)
    }
}
