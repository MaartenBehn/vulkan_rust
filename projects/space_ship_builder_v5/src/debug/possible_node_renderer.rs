use crate::math::to_1d_i;
use crate::ship::Ship;
use crate::ship_mesh::{RenderNode, ShipMesh};
use crate::ship_renderer::{ShipRenderer, RENDER_MODE_BUILD};
use octa_force::anyhow::Result;
use octa_force::glam::{ivec3, IVec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};

pub struct DebugPossibleNodeRenderer {
    mesh: ShipMesh,
    render_nodes: Vec<RenderNode>,
}

impl DebugPossibleNodeRenderer {
    pub fn new(image_len: usize, ship: &Ship) -> Result<Self> {
        let render_nodes = Self::get_debug_render_nodes(ship);
        Ok(DebugPossibleNodeRenderer {
            mesh: ShipMesh::new(image_len, IVec3::ONE * 128, ship.nodes_per_chunk)?,
            render_nodes,
        })
    }

    fn get_debug_render_nodes(ship: &Ship) -> Vec<RenderNode> {
        let mut render_nodes = vec![RenderNode(false); ship.node_length_plus_padding()];

        for x in 1..=ship.nodes_per_chunk.x {
            for y in 1..=ship.nodes_per_chunk.y {
                for z in 1..=ship.nodes_per_chunk.z {
                    let i = to_1d_i(ivec3(x, y, z), ship.node_size_plus_padding()) as usize;
                    render_nodes[i] = RenderNode(true);
                }
            }
        }

        render_nodes
    }

    pub fn update(
        &mut self,
        ship: &Ship,
        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.mesh.update_possible_node_debug(
            ship,
            &self.render_nodes,
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
