use app::vulkan::{Context, ash::vk};
use app::anyhow::*;

use crate::camera::Camera;

use self::vulkan::{PartUBO, RenderUBO};
use self::{part::{RenderPart}, vulkan::ChunkRendererVulkan};

use super::{MAX_AMMOUNT_OF_PARTS, CHUNK_PART_SIZE};

mod vulkan;
mod part;

pub struct ChunkRederer{
    rendered_parts: usize,
    vulkan: ChunkRendererVulkan,
    parts: [RenderPart; MAX_AMMOUNT_OF_PARTS],
}

impl ChunkRederer {
    pub fn new(context: &Context,
        color_attachment_format: vk::Format,
        images_len: u32,
        rendered_parts: usize
    ) -> Result<Self> {

        Ok(Self { 
            rendered_parts,
            vulkan: ChunkRendererVulkan::new(context, color_attachment_format, images_len, rendered_parts)?,
            parts: [RenderPart::default(); MAX_AMMOUNT_OF_PARTS as usize],
        })
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