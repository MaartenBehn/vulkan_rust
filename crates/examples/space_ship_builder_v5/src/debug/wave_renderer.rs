use crate::ship::Ship;
use crate::ship_mesh::ShipMesh;
use crate::ship_renderer::{ShipRenderer, RENDER_MODE_BUILD};
use crate::voxel_loader::VoxelLoader;
use octa_force::anyhow::Result;
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};

pub const WAVE_DEBUG_PS: u32 = 18;
pub const WAVE_DEBUG_RS: i32 = 64;

pub struct DebugWaveRenderer {
    mesh: ShipMesh,
}

impl DebugWaveRenderer {
    pub fn new(image_len: usize) -> Result<Self> {
        Ok(DebugWaveRenderer {
            mesh: ShipMesh::new(image_len, 128)?,
        })
    }

    pub fn update(
        &mut self,
        ship: &Ship,
        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.mesh.update_node_debug(
            ship,
            image_index,
            context,
            descriptor_layout,
            descriptor_pool,
        )
    }

    pub fn render(&mut self, buffer: &CommandBuffer, renderer: &ShipRenderer, image_index: usize) {
        buffer.bind_graphics_pipeline(&renderer.pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &renderer.pipeline_layout,
            0,
            &[&renderer.static_descriptor_sets[image_index]],
        );

        renderer.render_ship_mesh(buffer, image_index, &self.mesh, RENDER_MODE_BUILD)
    }
}
