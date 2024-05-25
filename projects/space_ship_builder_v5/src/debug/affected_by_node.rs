use crate::debug::DebugController;
use crate::math::to_1d_i;
use crate::rules::Rules;
use crate::ship::mesh::{MeshChunk, RenderNode, ShipMesh};
use crate::ship::renderer::{ShipRenderer, RENDER_MODE_BUILD};
use log::info;
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{ivec3, vec3, vec4, IVec3, Vec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::{Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::time::Duration;

pub const RULES_SIZE: i32 = 4;
const NEXT_NODE_SPEED: Duration = Duration::from_millis(100);

pub struct AffectedByNodeRenderer {
    mesh: ShipMesh,
    render_nodes: Vec<RenderNode>,
    rule_index: usize,
    last_action_time: Duration,
}

impl AffectedByNodeRenderer {
    pub fn new(image_len: usize) -> Self {
        let size = IVec3::ONE * RULES_SIZE;
        let render_nodes = Self::get_debug_render_nodes(size);
        AffectedByNodeRenderer {
            mesh: ShipMesh::new(image_len, size, size),
            render_nodes,
            rule_index: 0,
            last_action_time: Duration::ZERO,
        }
    }

    fn update_rule_index(&mut self, controls: &Controls, rules: &Rules, total_time: Duration) {
        if controls.t && (self.last_action_time + NEXT_NODE_SPEED) < total_time {
            self.last_action_time = total_time;

            self.rule_index += 1;
            if self.rule_index >= rules.node_rules.len() {
                self.rule_index = 0;
            }

            info!("Debug Rule: {}", self.rule_index);
        }
    }

    fn update_renderer(
        &mut self,

        node_id_bits: &Vec<u32>,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        // Buffers from the last swapchain iteration are being dropped
        self.mesh.to_drop_buffers[image_index].clear();

        if !self.mesh.chunks.is_empty() {
            self.mesh.chunks[0].update_from_data(
                node_id_bits,
                &self.render_nodes,
                context,
                &mut self.mesh.to_drop_buffers[image_index],
            )?;
        } else {
            let new_chunk = MeshChunk::new_from_data(
                IVec3::ZERO,
                self.mesh.size,
                self.mesh.render_size,
                node_id_bits,
                &self.render_nodes,
                self.mesh.to_drop_buffers.len(),
                context,
                descriptor_layout,
                descriptor_pool,
            )?;
            if new_chunk.is_some() {
                self.mesh.chunks.push(new_chunk.unwrap())
            }
        }

        Ok(())
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

    pub fn render(&mut self, buffer: &CommandBuffer, renderer: &ShipRenderer, image_index: usize) {
        renderer.render(buffer, image_index, RENDER_MODE_BUILD, &self.mesh)
    }
}

impl DebugController {
    pub fn update_affected_by_node(
        &mut self,

        rules: &Rules,
        controls: &Controls,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
        total_time: Duration,
    ) -> Result<()> {
        self.affected_by_node_renderer
            .update_rule_index(controls, rules, total_time);

        self.add_text(vec!["Affected by Node".to_owned()], vec3(-1.0, 0.0, 0.0));

        self.add_cube(
            Vec3::ZERO,
            Vec3::ONE * RULES_SIZE as f32,
            vec4(1.0, 0.0, 0.0, 1.0),
        );
        self.add_cube(
            Vec3::ONE * (RULES_SIZE / 2) as f32,
            Vec3::ONE * ((RULES_SIZE / 2) + 1) as f32,
            vec4(0.0, 0.0, 1.0, 1.0),
        );

        let node_id =
            rules.map_rules_index_to_node_id[self.affected_by_node_renderer.rule_index][0];
        if rules.affected_by_node.contains_key(&node_id) {
            let affected_positions: &Vec<IVec3> = &rules.affected_by_node[&node_id];
            let middle_pos = self.affected_by_node_renderer.mesh.size / 2;

            for pos in affected_positions {
                let p = (*pos + middle_pos).as_vec3();
                self.add_cube(p, p + Vec3::ONE, vec4(0.0, 1.0, 0.0, 1.0))
            }
        }

        let node_id_bits = self.get_affected_by_node_node_id_bits(
            self.affected_by_node_renderer.mesh.size,
            rules,
            self.affected_by_node_renderer.rule_index,
        );

        self.affected_by_node_renderer.update_renderer(
            &node_id_bits,
            image_index,
            context,
            descriptor_layout,
            descriptor_pool,
        )?;

        self.text_renderer.push_texts()?;
        self.line_renderer.push_lines()?;

        Ok(())
    }

    fn get_affected_by_node_node_id_bits(
        &mut self,
        size: IVec3,
        rules: &Rules,
        rule_index: usize,
    ) -> Vec<u32> {
        let mut node_debug_node_id_bits = vec![0; size.element_product() as usize];

        let middle_pos = size / 2;
        let middle_index = to_1d_i(middle_pos, size) as usize;
        node_debug_node_id_bits[middle_index] =
            rules.map_rules_index_to_node_id[rule_index][0].into();

        node_debug_node_id_bits
    }
}
