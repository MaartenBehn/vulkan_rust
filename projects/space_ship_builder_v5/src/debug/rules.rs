use crate::debug::DebugController;
use crate::math::to_1d_i;
use crate::rules::Rules;
use crate::ship_mesh::{MeshChunk, RenderNode, ShipMesh};
use crate::ship_renderer::{ShipRenderer, RENDER_MODE_BUILD};
use log::info;
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{ivec3, vec3, vec4, IVec3, Vec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::{Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::time::Duration;

pub const RULES_SIZE: i32 = 4;
const NEXT_NODE_SPEED: Duration = Duration::from_millis(100);

pub struct DebugRulesRenderer {
    mesh: ShipMesh,
    render_nodes: Vec<RenderNode>,
    rule_index: usize,
    last_action_time: Duration,
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
            last_action_time: Duration::ZERO,
        })
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

impl DebugController {
    pub fn update_rules(
        &mut self,

        rules: &Rules,
        controls: &Controls,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
        total_time: Duration,
    ) -> Result<()> {
        self.rules_renderer
            .update_rule_index(controls, rules, total_time);

        self.add_text(vec!["RULES".to_owned()], vec3(-1.0, 0.0, 0.0));

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

        let node_id_bits = self.get_rule_node_id_bits_debug(
            self.rules_renderer.mesh.size,
            self.rules_renderer.mesh.render_size,
            rules,
            self.rules_renderer.rule_index,
        );

        self.rules_renderer.update_renderer(
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

    fn get_rule_node_id_bits_debug(
        &mut self,
        size: IVec3,
        rules_size: IVec3,
        rules: &Rules,
        rule_index: usize,
    ) -> Vec<u32> {
        let mut node_debug_node_id_bits = vec![0; size.element_product() as usize];
        let pattern_block_size = size / rules_size;

        let middle_pos = rules_size / 2;
        let middle_index = to_1d_i(middle_pos * pattern_block_size, size) as usize;
        node_debug_node_id_bits[middle_index] =
            rules.map_rules_index_to_node_id[rule_index][0].into();

        let nodes = &rules.node_rules[rule_index];
        for x in 0..rules_size.x {
            for y in 0..rules_size.y {
                for z in 0..rules_size.z {
                    let node_pos = ivec3(x, y, z);
                    let test_pos = node_pos - middle_pos;

                    if !nodes.contains_key(&test_pos) {
                        continue;
                    }

                    let mut pattern_counter = 0;
                    let possible_nodes = &nodes[&test_pos];
                    let node_pos = node_pos * pattern_block_size;

                    'iter: for iz in 0..pattern_block_size.x {
                        for iy in 0..pattern_block_size.y {
                            for ix in 0..pattern_block_size.z {
                                if possible_nodes.len() <= pattern_counter {
                                    break 'iter;
                                }

                                let pattern_pos = ivec3(ix, iy, iz) + node_pos;
                                let index = to_1d_i(pattern_pos, size) as usize;

                                let node_id = possible_nodes[pattern_counter];
                                node_debug_node_id_bits[index] = node_id.into();

                                if node_id.is_none() {
                                    let one_cell_size = Vec3::ONE / pattern_block_size.as_vec3();
                                    let p = pattern_pos.as_vec3() * one_cell_size;
                                    self.add_cube(p, p + one_cell_size, vec4(0.0, 1.0, 0.0, 1.0));
                                }

                                pattern_counter += 1;
                            }
                        }
                    }
                }
            }
        }

        node_debug_node_id_bits
    }
}
