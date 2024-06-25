use crate::debug::DebugController;
use crate::math::{oct_positions, to_1d_i};
use crate::node::NodeID;
use crate::rules::block::BlockNameIndex;
use crate::rules::hull::HullSolver;
use crate::rules::Rules;
use crate::ship::data::ShipData;
use crate::ship::mesh::{MeshChunk, RenderNode, ShipMesh};
use crate::ship::possible_blocks::PossibleBlocks;
use crate::ship::renderer::{ShipRenderer, RENDER_MODE_BASE};
use crate::ship::ShipManager;
use index_queue::IndexQueue;
use log::info;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::{ivec3, ivec4, vec3, vec4, IVec3};
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::iter;
use std::time::{Duration, Instant};

const INPUT_INTERVAL: Duration = Duration::from_millis(100);

const CACHE_INDEX_UPDATE_INTERVAL: Duration = Duration::from_millis(1000);

#[derive(Clone)]
pub struct LogEntry {
    pub blocks: Vec<PossibleBlocks>,
    pub to_reset: IndexQueue,
    pub was_reset: IndexQueue,
    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,
}

pub struct CollapseLogRenderer {
    mesh: ShipMesh,

    last_blocks_names: Vec<BlockNameIndex>,
    block_log: Vec<LogEntry>,
    log_index: usize,
    last_input: Instant,
    cache_index: usize,
    last_index_update: Instant,

    build_hull: bool,
    pos: IVec3,
    preview_index: usize,
}

impl CollapseLogRenderer {
    pub fn new(image_len: usize, ship_data: &ShipData) -> Self {
        CollapseLogRenderer {
            mesh: ShipMesh::new(
                image_len,
                ship_data.nodes_per_chunk,
                ship_data.nodes_per_chunk,
            ),
            last_input: Instant::now(),
            last_blocks_names: vec![],
            block_log: vec![],
            log_index: 0,
            cache_index: 0,
            last_index_update: Instant::now(),

            build_hull: true,
            pos: IVec3::ZERO,
            preview_index: 0,
        }
    }

    fn update(
        &mut self,
        ship_data: &mut ShipData,
        controls: &Controls,
        rules: &Rules,
        camera: &Camera,
    ) {
        if ship_data.chunks[0].block_names != self.last_blocks_names {
            self.block_log = vec![LogEntry {
                blocks: ship_data.chunks[0].blocks.to_owned(),
                to_reset: ship_data.to_reset.to_owned(),
                was_reset: ship_data.was_reset.to_owned(),
                to_propergate: ship_data.to_propergate.to_owned(),
                to_collapse: ship_data.to_collapse.to_owned(),
            }];
            self.log_index = 0;
            self.last_blocks_names = ship_data.chunks[0].block_names.to_owned();
        }

        while ship_data.tick(1, rules).0 {
            let new_log_entry = LogEntry {
                blocks: ship_data.chunks[0].blocks.to_owned(),
                to_reset: ship_data.to_reset.to_owned(),
                was_reset: ship_data.was_reset.to_owned(),
                to_propergate: ship_data.to_propergate.to_owned(),
                to_collapse: ship_data.to_collapse.to_owned(),
            };

            if self.block_log.is_empty() || new_log_entry != *self.block_log.last().unwrap() {
                self.block_log.push(new_log_entry);
                info!("Added Log {}", self.block_log.len() - 1);
            }
        }

        if self.last_input.elapsed() > INPUT_INTERVAL && controls.rigth {
            self.log_index = (self.log_index + 1) % self.block_log.len();
            self.last_input = Instant::now();

            info!("Log Index: {}", self.log_index);
        }

        if self.last_input.elapsed() > INPUT_INTERVAL && controls.left {
            self.log_index = if self.log_index == 0 {
                self.block_log.len() - 1
            } else {
                self.log_index - 1
            };
            self.last_input = Instant::now();

            info!("Log Index: {}", self.log_index);
        }

        let hull_solver = rules.solvers[1].to_hull().unwrap();
        if self.last_input.elapsed() > INPUT_INTERVAL && controls.t {
            self.preview_index = (self.preview_index + 1) % hull_solver.multi_blocks.len();
            self.last_input = Instant::now();

            info!("Preview Index: {}", self.preview_index);
        }

        if self.last_input.elapsed() > INPUT_INTERVAL && controls.down {
            self.preview_index = if self.preview_index == 0 {
                hull_solver.multi_blocks.len() - 1
            } else {
                self.preview_index - 1
            };
            self.last_input = Instant::now();

            info!("Preview Index: {}", self.preview_index);
        }

        self.pos = (((camera.position + camera.direction * 3.0) - vec3(1.0, 1.0, 1.0)) / 2.0)
            .round()
            .as_ivec3();

        if controls.e && self.last_input.elapsed() > INPUT_INTERVAL {
            self.last_input = Instant::now();
            self.build_hull = !self.build_hull;

            ship_data.place_block(self.pos, if self.build_hull { 1 } else { 0 }, rules);
        }

        if self.last_index_update.elapsed() < CACHE_INDEX_UPDATE_INTERVAL {
            self.cache_index = self.cache_index + 1;
            self.last_index_update = Instant::now();
        }
    }

    fn update_renderer(
        &mut self,

        node_id_bits: &Vec<u32>,
        render_nodes: &Vec<RenderNode>,

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
                &render_nodes,
                context,
                &mut self.mesh.to_drop_buffers[image_index],
            )?;
        } else {
            let new_chunk = MeshChunk::new_from_data(
                IVec3::ZERO,
                self.mesh.size,
                self.mesh.render_size,
                node_id_bits,
                render_nodes,
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

    pub fn render(&mut self, buffer: &CommandBuffer, renderer: &ShipRenderer, image_index: usize) {
        renderer.render(buffer, image_index, RENDER_MODE_BASE, &self.mesh)
    }
}

impl DebugController {
    pub fn update_collapse_log_debug(
        &mut self,

        ship_data: &mut ShipData,
        controls: &Controls,
        rules: &Rules,
        camera: &Camera,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.collapse_log_renderer
            .update(ship_data, controls, rules, camera);

        self.draw_selected(ship_data);

        let (node_id_bits, render_nodes) = self.get_collapse_log_node_id_bits(
            self.collapse_log_renderer.mesh.size,
            ship_data,
            rules,
        );

        self.collapse_log_renderer.update_renderer(
            &node_id_bits,
            &render_nodes,
            image_index,
            context,
            descriptor_layout,
            descriptor_pool,
        )?;

        self.text_renderer.push_texts()?;
        self.line_renderer.push_lines()?;

        Ok(())
    }

    fn draw_selected(&mut self, ship_data: &ShipData) {
        let node_pos = ship_data.get_node_pos_from_block_pos(self.collapse_log_renderer.pos);

        self.add_cube(
            node_pos.as_vec3() - 0.1,
            (node_pos + 2).as_vec3() + 0.1,
            vec4(0.0, 0.0, 0.0, 1.0),
        )
    }

    fn get_collapse_log_node_id_bits(
        &mut self,
        size: IVec3,
        ship_data: &ShipData,
        rules: &Rules,
    ) -> (Vec<u32>, Vec<RenderNode>) {
        let mut node_id_bits = vec![0; size.element_product() as usize];
        let mut render_nodes = vec![RenderNode(false); (size + 2).element_product() as usize];

        for x in 0..(size.x / 2) {
            for y in 0..(size.y / 2) {
                for z in 0..(size.z / 2) {
                    let node_pos = ship_data.get_node_pos_from_block_pos(ivec3(x, y, z));
                    let block_index =
                        ship_data.get_block_index_from_world_block_pos(ivec3(x, y, z));

                    if self.collapse_log_renderer.block_log.is_empty() {
                        continue;
                    }

                    {
                        let hull_slover = rules.solvers[1].to_hull().unwrap();

                        let order = ship_data.order_controller.pack_propergate_order(
                            hull_slover.block_name_index,
                            block_index,
                            0,
                        );
                        let to_reset = &self.collapse_log_renderer.block_log
                            [self.collapse_log_renderer.log_index]
                            .to_reset;
                        if to_reset.contains(order) {
                            self.add_cube(
                                node_pos.as_vec3() - 0.01,
                                (node_pos + 2).as_vec3() + 0.01,
                                vec4(0.0, 1.0, 0.0, 1.0),
                            )
                        }

                        let to_propergate = &self.collapse_log_renderer.block_log
                            [self.collapse_log_renderer.log_index]
                            .to_propergate;
                        if to_propergate.contains(order) {
                            self.add_cube(
                                node_pos.as_vec3() - 0.02,
                                (node_pos + 2).as_vec3() + 0.02,
                                vec4(0.0, 0.0, 1.0, 1.0),
                            )
                        }

                        let collapse_order = ship_data
                            .order_controller
                            .pack_collapse_order(block_index, 0);
                        let to_collapse = &self.collapse_log_renderer.block_log
                            [self.collapse_log_renderer.log_index]
                            .to_collapse;
                        if to_collapse.contains(collapse_order) {
                            self.add_cube(
                                node_pos.as_vec3() - 0.03,
                                (node_pos + 2).as_vec3() + 0.03,
                                vec4(1.0, 0.0, 0.0, 1.0),
                            )
                        }
                    }

                    {
                        let caches: Vec<_> = self.collapse_log_renderer.block_log
                            [self.collapse_log_renderer.log_index]
                            .blocks[block_index]
                            .get_all_caches()
                            .into_iter()
                            .map(|(block_name, cache)| {
                                iter::repeat(block_name).zip(cache.into_iter())
                            })
                            .flatten()
                            .collect();

                        if caches.is_empty() {
                            continue;
                        }

                        let (block_name_index, cache_index) =
                            caches[self.collapse_log_renderer.cache_index % caches.len()];

                        let mut block = None;

                        let hull_solver = rules.solvers[block_name_index].to_hull();
                        if hull_solver.is_ok() {
                            block =
                                Some(hull_solver.unwrap().get_block_from_cache_index(cache_index));
                        }

                        let indices: Vec<_> = oct_positions()
                            .into_iter()
                            .map(|offset| {
                                let pos = node_pos + offset;
                                let index = ship_data.get_node_index_from_node_pos(pos);
                                let index_with_padding =
                                    ship_data.get_node_index_with_padding_from_node_pos(pos);
                                (index, index_with_padding)
                            })
                            .collect();

                        if block.is_some() {
                            let block = block.unwrap();

                            for (node_id, (index, index_with_padding)) in
                                block.node_ids.into_iter().zip(indices.into_iter())
                            {
                                node_id_bits[index] = node_id.into();
                                render_nodes[index_with_padding] = RenderNode(node_id.is_some());
                            }
                        }
                    }
                }
            }
        }

        (node_id_bits, render_nodes)
    }
}

impl PartialEq for LogEntry {
    fn eq(&self, other: &Self) -> bool {
        self.blocks == other.blocks
            && index_queue_eg(&self.to_reset, &other.to_reset)
            && index_queue_eg(&self.was_reset, &other.was_reset)
            && index_queue_eg(&self.to_propergate, &other.to_propergate)
            && index_queue_eg(&self.to_collapse, &other.to_collapse)
    }
}

fn index_queue_eg(q1: &IndexQueue, q2: &IndexQueue) -> bool {
    let mut q1_copy = q1.to_owned();
    while !q1_copy.is_empty() {
        let elem = q1_copy.pop_front();
        if !q2.contains(elem.unwrap()) {
            return false;
        }
    }

    let mut q2_copy = q1.to_owned();
    while !q2_copy.is_empty() {
        let elem = q2_copy.pop_front();
        if !q1.contains(elem.unwrap()) {
            return false;
        }
    }

    true
}
