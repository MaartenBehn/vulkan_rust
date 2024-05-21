use crate::math::to_1d_i;
use crate::rules::Rules;
use crate::ship_mesh::{MeshChunk, RenderNode, ShipMesh};
use crate::ship_renderer::{ShipRenderer, RENDER_MODE_BUILD};
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{ivec3, IVec3};
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
        controls: &Controls,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
        total_time: Duration,
    ) -> Result<()> {
        if controls.t && (self.last_action_time + NEXT_NODE_SPEED) < total_time {
            self.last_action_time = total_time;

            self.rule_index += 1;
            if self.rule_index >= rules.node_rules.len() {
                self.rule_index = 0;
            }
        }

        // Buffers from the last swapchain iteration are being dropped
        self.mesh.to_drop_buffers[image_index].clear();

        if !self.mesh.chunks.is_empty() {
            Self::update_rules_debug(
                &mut self.mesh.chunks[0],
                self.mesh.size,
                self.mesh.render_size,
                rules,
                self.rule_index,
                &self.render_nodes,
                context,
                &mut self.mesh.to_drop_buffers[image_index],
            )?;
        } else {
            let new_chunk = Self::new_rules_debug(
                self.mesh.size,
                self.mesh.render_size,
                rules,
                self.rule_index,
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

    fn new_rules_debug(
        size: IVec3,
        render_size: IVec3,

        rules: &Rules,
        rule_index: usize,
        render_nodes: &Vec<RenderNode>,

        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Option<MeshChunk>> {
        let wave_debug_node_id_bits =
            Self::get_rule_node_id_bits_debug(size, render_size, rules, rule_index);

        MeshChunk::new_from_data(
            IVec3::ZERO,
            size,
            render_size,
            &wave_debug_node_id_bits,
            render_nodes,
            images_len,
            context,
            descriptor_layout,
            descriptor_pool,
        )
    }

    fn get_rule_node_id_bits_debug(
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
                                } else if possible_nodes[pattern_counter].is_none() {
                                    pattern_counter += 1;

                                    if possible_nodes.len() <= pattern_counter {
                                        break 'iter;
                                    }
                                }

                                let pattern_pos = ivec3(ix, iy, iz) + node_pos;
                                let index = to_1d_i(pattern_pos, size) as usize;

                                let node = possible_nodes[pattern_counter];
                                node_debug_node_id_bits[index] = node.into();
                                pattern_counter += 1;
                            }
                        }
                    }
                }
            }
        }

        node_debug_node_id_bits
    }

    fn update_rules_debug(
        chunk: &mut MeshChunk,

        size: IVec3,
        render_size: IVec3,

        rules: &Rules,
        rule_index: usize,
        render_nodes: &Vec<RenderNode>,

        context: &Context,
        to_drop_buffers: &mut Vec<Buffer>,
    ) -> Result<()> {
        let wave_debug_node_id_bits =
            Self::get_rule_node_id_bits_debug(size, render_size, rules, rule_index);

        chunk.update_from_data(
            &wave_debug_node_id_bits,
            render_nodes,
            context,
            to_drop_buffers,
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
