use crate::math::to_1d_i;
use crate::rules::Rules;
use crate::ship_mesh::{RenderNode, ShipMesh};
use crate::ship_renderer::{ShipRenderer, RENDER_MODE_BUILD};
use octa_force::anyhow::Result;
use octa_force::glam::{ivec3, IVec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};

pub const RULES_SIZE: i32 = 7;

pub struct DebugRulesRenderer {
    mesh: ShipMesh,
    render_nodes: Vec<RenderNode>,
    rule_index: usize,
}

impl DebugRulesRenderer {
    pub fn new(image_len: usize) -> Result<Self> {
        let size = IVec3::ONE * RULES_SIZE * 4;
        let render_size = IVec3::ONE * RULES_SIZE;
        let render_nodes = Self::get_debug_render_nodes(render_size);
        Ok(DebugRulesRenderer {
            mesh: ShipMesh::new(image_len, size, render_size)?,
            render_nodes,
            rule_index: 0,
        })
    }

    fn get_debug_render_nodes(render_size: IVec3) -> Vec<RenderNode> {
        let mut render_nodes =
            vec![RenderNode(false); (render_size + 2).element_product() as usize];

        for x in 1..=render_size.x {
            for y in 1..=render_size.y {
                for z in 1..=render_size.z {
                    let i = to_1d_i(ivec3(x, y, z), render_size + 2) as usize;
                    render_nodes[i] = RenderNode(true);
                }
            }
        }

        render_nodes
    }

    pub fn update(
        &mut self,

        rules: &Rules,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.mesh.update_rules_debug(
            rules,
            self.rule_index,
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
