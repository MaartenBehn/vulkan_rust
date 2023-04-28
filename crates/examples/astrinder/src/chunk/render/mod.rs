use std::sync::mpsc::Receiver;

use app::vulkan::{Context, ash::vk};
use app::anyhow::*;

use crate::camera::Camera;

use self::part::RenderParticle;
use self::vulkan::{PartUBO, RenderUBO};
use self::{part::{RenderPart}, vulkan::ChunkRendererVulkan};

use super::particle::Particle;
use super::transform::Transform;
use super::{MAX_AMMOUNT_OF_PARTS, CHUNK_PART_SIZE};

pub mod vulkan;
pub mod part;

pub struct ChunkRenderer{
    rendered_parts: usize,
    pub vulkan: ChunkRendererVulkan,
    parts: [RenderPart; MAX_AMMOUNT_OF_PARTS],

    from_controller_transforms: Receiver<(usize, Transform)>,
    from_controller_particles: Receiver<(usize, [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
} 

impl ChunkRenderer {
    pub fn new(
        context: &Context,
        color_attachment_format: vk::Format,
        images_len: u32,
        rendered_parts: usize,
        from_controller_transforms: Receiver<(usize, Transform)>,
        from_controller_particles: Receiver<(usize, [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
    ) -> Result<Self> {

        Ok(Self { 
            rendered_parts,
            vulkan: ChunkRendererVulkan::new(context, color_attachment_format, images_len, rendered_parts)?,
            parts: [RenderPart::default(); MAX_AMMOUNT_OF_PARTS as usize],

            from_controller_transforms,
            from_controller_particles,
        })
    }

    pub fn recive_parts(&mut self) {
        loop {
            let data = self.from_controller_transforms.try_recv();
            if data.is_err() {
                break;
            }


            let (id, transform) = data.unwrap();
            self.parts[id].transform = transform;
        }

        loop {
            let data = self.from_controller_particles.try_recv();
            if data.is_err() {
                break;
            }

            let (id, particles) = data.unwrap();
            for (i, particle) in particles.iter().enumerate() {
                self.parts[id].particles[i] = RenderParticle::from(particle)
            }
        }
    }

    pub fn upload (
        &mut self, 
        camera: &Camera,
    ) -> Result<()> {
        
        self.vulkan.render_ubo.copy_data_to_buffer(&[RenderUBO::new(camera.to_owned())])?;

        self.upload_all_parts()?;

        Ok(())
    }

    fn upload_all_parts(&mut self) -> Result<()> {

        let mut i = 0;
        for part in self.parts.iter() {
            self.vulkan.part_ubo_data[i] = PartUBO::new(part.transform);

            let start_index = i * (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize;
            let end_index = (i + 1) * (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize;
            self.vulkan.particle_buffer_data.splice(start_index..end_index, part.particles.iter().cloned());

            i += 1;

            if i >= self.rendered_parts {
                break;
            }
        }

        self.vulkan.part_ubo.copy_data_to_buffer(&self.vulkan.part_ubo_data)?;
        self.vulkan.particles_ssbo.copy_data_to_buffer(&self.vulkan.particle_buffer_data)?;

        Ok(())
    }

}