use crate::debug::DebugController;
use crate::math::oct_positions;
use crate::render::parallax::chunk::{ParallaxData, RenderNode};
use crate::render::parallax::renderer::ParallaxRenderer;
use crate::rules::solver::{SolverFunctions};
use crate::rules::Rules;
use crate::world::block_object::collapse::Collapser;
use crate::world::block_object::possible_blocks::PossibleBlocks;
use crate::world::block_object::BlockObject;
use crate::world::data::block::BlockNameIndex;
use index_queue::IndexQueue;
use log::info;
use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::controls::Controls;
use octa_force::glam::{ivec3, vec3, vec4, IVec3, Vec3};
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::iter;
use std::time::{Duration, Instant};

const INPUT_INTERVAL: Duration = Duration::from_millis(100);
const PLACE_INPUT_INTERVAL: Duration = Duration::from_millis(5000);

const CACHE_INDEX_UPDATE_INTERVAL: Duration = Duration::from_millis(1000);

const LOG_SIZE: usize = 10;

#[derive(Clone)]
pub struct LogEntry {
    pub blocks: Vec<PossibleBlocks>,
    pub to_reset: IndexQueue,
    pub was_reset: IndexQueue,
    pub to_propergate: IndexQueue,
    pub collapser: Collapser,
    pub is_collapsed: IndexQueue,
}

pub struct CollapseLogRenderer {
    mesh: ParallaxData,

    last_blocks_names: Vec<BlockNameIndex>,
    block_log: Vec<LogEntry>,
    log_end: usize,

    log_index: usize,
    last_log_index: usize,
    last_input: Instant,
    cache_index: usize,
    last_index_update: Instant,

    pos: IVec3,
    preview_index: usize,

    show_orders: bool,
}

impl CollapseLogRenderer {
    pub fn new(image_len: usize, block_object: &BlockObject) -> Self {
        CollapseLogRenderer {
            mesh: ParallaxData::new_from_block_object(block_object, image_len),
            last_input: Instant::now(),
            last_blocks_names: vec![],
            block_log: vec![],
            log_end: 0,
            log_index: 0,
            last_log_index: 0,
            cache_index: 0,
            last_index_update: Instant::now(),

            pos: IVec3::ZERO,
            preview_index: 0,

            show_orders: true,
        }
    }

    fn update(
        &mut self,
        ship_data: &mut BlockObject,
        controls: &Controls,
        rules: &Rules,
        camera: &Camera,
    ) {
        // Input
        if self.last_input.elapsed() > INPUT_INTERVAL && controls.rigth {
            self.log_index += 1;
            self.last_input = Instant::now();

            info!("Log Index: {}", self.log_index);
        }

        if self.last_input.elapsed() > INPUT_INTERVAL && controls.left {
            self.log_index -= 1;
            self.last_input = Instant::now();

            info!("Log Index: {}", self.log_index);
        }

        let hull_solver = &rules.solvers[1].as_hull().unwrap();
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

        if self.last_input.elapsed() > INPUT_INTERVAL && controls.r {
            self.last_input = Instant::now();
            self.show_orders = !self.show_orders;
        }

        // Update Cache
        if self.last_index_update.elapsed() > CACHE_INDEX_UPDATE_INTERVAL {
            self.cache_index = self.cache_index + 1;
            self.last_index_update = Instant::now();
        }

        // Block Placement
        self.pos = (((camera.position + camera.direction * 3.0) - vec3(1.0, 1.0, 1.0)) / 2.0)
            .round()
            .as_ivec3();

        if controls.e && self.last_input.elapsed() > PLACE_INPUT_INTERVAL {
            self.last_input = Instant::now();

            let block_name_index = ship_data.get_block_name_from_world_block_pos(self.pos);
            let new_block_name_index = if block_name_index == 0 { 1 } else { 0 };

            info!("Place {}", new_block_name_index);
            ship_data.place_block(self.pos, new_block_name_index);
        }

        // Rest
        if ship_data.chunks[0].block_names != self.last_blocks_names {
            self.block_log = vec![LogEntry {
                blocks: ship_data.chunks[0].blocks.to_owned(),
                to_reset: ship_data.to_reset.to_owned(),
                was_reset: ship_data.was_reset.to_owned(),
                to_propergate: ship_data.to_propergate.to_owned(),
                collapser: ship_data.collapser.to_owned(),
                is_collapsed: ship_data.is_collapsed.to_owned(),
            }];
            self.log_index = 0;
            self.log_end = 0;
            self.last_blocks_names = ship_data.chunks[0].block_names.to_owned();
        }

        // Create Log
        while ((self.block_log.len() + self.log_end) <= self.log_index)
            && ship_data.tick(1, rules).0
        {
            let new_log_entry = LogEntry {
                blocks: ship_data.chunks[0].blocks.to_owned(),
                to_reset: ship_data.to_reset.to_owned(),
                was_reset: ship_data.was_reset.to_owned(),
                to_propergate: ship_data.to_propergate.to_owned(),
                collapser: ship_data.collapser.to_owned(),
                is_collapsed: ship_data.is_collapsed.to_owned(),
            };

            self.block_log.push(new_log_entry);

            if self.block_log.len() > LOG_SIZE {
                self.log_end += 1;
                self.block_log.remove(0);
            }

            info!("Added Log {}", self.block_log.len() - 1 + self.log_end);
        }

        if self.log_index >= self.log_end + self.block_log.len() {
            self.log_index = self.log_end + self.block_log.len() - 1;
        }

        if self.log_index < self.log_end {
            self.log_index = self.log_end;
        }

        if self.last_log_index != self.log_index {
            self.print_next_action(ship_data);
        }

        self.last_log_index = self.log_index;
    }

    fn print_next_action(&mut self, ship_data: &mut BlockObject) {
        let mut to_reset = self.block_log[self.log_index - self.log_end]
            .to_reset
            .to_owned();
        if !to_reset.is_empty() {
            // Querry Result
            let order = to_reset.pop_front().unwrap();
            let (block_name_index, block_index, chunk_index) =
                ship_data.order_controller.unpack_propergate_order(order);

            let block_pos =
                ship_data.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

            info!("Rest Block Name {} at {}", block_name_index, block_pos);
            return;
        }

        let mut to_propergate = self.block_log[self.log_index - self.log_end]
            .to_propergate
            .to_owned();
        if !to_propergate.is_empty() {
            // Querry Result
            let order = to_propergate.pop_front().unwrap();
            let (block_name_index, block_index, chunk_index) =
                ship_data.order_controller.unpack_propergate_order(order);

            let block_pos =
                ship_data.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

            info!(
                "Propergate Block Name {} at {}",
                block_name_index, block_pos
            );
            return;
        }

        let mut collapser = self.block_log[self.log_index - self.log_end]
            .collapser
            .to_owned();
        if !collapser.is_empty() {
            // Querry Result
            let order = collapser.pop_order();
            let (block_index, chunk_index) =
                ship_data.order_controller.unpack_collapse_order(order);

            let block_pos =
                ship_data.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

            info!("Collapse at {}", block_pos);
            return;
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
            let new_chunk = ParallaxData::new_from_data(
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

    pub fn render(
        &mut self,
        buffer: &CommandBuffer,
        renderer: &ParallaxRenderer,
        image_index: usize,
    ) {
        renderer
            .render_mesh(buffer, image_index, &self.mesh)
            .unwrap()
    }
}

impl DebugController {
    pub fn update_collapse_log_debug(
        &mut self,

        ship_data: &mut BlockObject,
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

        if self.collapse_log_renderer.show_orders {
            self.draw_orders(ship_data);
        }

        let (mut node_id_bits, mut render_nodes) = self.get_collapse_log_node_id_bits(
            self.collapse_log_renderer.mesh.size,
            ship_data,
            rules,
        );

        self.draw_next_action(ship_data, rules, &mut node_id_bits, &mut render_nodes);

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

    fn draw_selected(&mut self, ship_data: &BlockObject) {
        let node_pos = ship_data.get_node_pos_from_block_pos(self.collapse_log_renderer.pos);

        self.add_cube(
            node_pos.as_vec3() - 0.1,
            (node_pos + 2).as_vec3() + 0.1,
            vec4(0.0, 0.0, 0.0, 1.0),
        )
    }

    fn draw_orders(&mut self, ship_data: &BlockObject) {
        self.add_cube(
            Vec3::ZERO,
            ship_data.nodes_per_chunk.as_vec3(),
            vec4(0.0, 0.0, 0.0, 1.0),
        );

        let mut to_reset = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .to_reset
            .to_owned();
        while !to_reset.is_empty() {
            let order = to_reset.pop_front().unwrap();
            let (_, block_index, chunk_index) =
                ship_data.order_controller.unpack_propergate_order(order);
            let pos =
                ship_data.get_world_node_pos_from_chunk_and_block_index(block_index, chunk_index);

            self.add_cube(
                pos.as_vec3() - 0.01,
                pos.as_vec3() + 2.01,
                vec4(0.5, 0.5, 0.0, 1.0),
            );
        }

        let mut was_reset = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .was_reset
            .to_owned();
        while !was_reset.is_empty() {
            let order = was_reset.pop_front().unwrap();
            let (_, block_index, chunk_index) =
                ship_data.order_controller.unpack_propergate_order(order);
            let pos =
                ship_data.get_world_node_pos_from_chunk_and_block_index(block_index, chunk_index);

            self.add_cube(
                pos.as_vec3() - 0.02,
                pos.as_vec3() + 2.02,
                vec4(0.5, 0.2, 0.0, 1.0),
            );
        }

        let mut to_propergate = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .to_propergate
            .to_owned();
        while !to_propergate.is_empty() {
            let order = to_propergate.pop_front().unwrap();
            let (_, block_index, chunk_index) =
                ship_data.order_controller.unpack_propergate_order(order);
            let pos =
                ship_data.get_world_node_pos_from_chunk_and_block_index(block_index, chunk_index);

            self.add_cube(
                pos.as_vec3() - 0.03,
                pos.as_vec3() + 2.03,
                vec4(0.5, 0.0, 0.5, 1.0),
            );
        }

        let mut to_collapse = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .collapser
            .to_owned();
        while !to_collapse.is_empty() {
            let order = to_collapse.pop_order();
            let (block_index, chunk_index) =
                ship_data.order_controller.unpack_collapse_order(order);
            let pos =
                ship_data.get_world_node_pos_from_chunk_and_block_index(block_index, chunk_index);

            self.add_cube(
                pos.as_vec3() - 0.04,
                pos.as_vec3() + 2.04,
                vec4(0.0, 0.5, 0.5, 1.0),
            );
        }

        let mut is_collapsed = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .is_collapsed
            .to_owned();
        while !is_collapsed.is_empty() {
            let order = is_collapsed.pop_front().unwrap();
            let (block_index, chunk_index) =
                ship_data.order_controller.unpack_collapse_order(order);
            let pos =
                ship_data.get_world_node_pos_from_chunk_and_block_index(block_index, chunk_index);

            self.add_cube(
                pos.as_vec3() - 0.05,
                pos.as_vec3() + 2.05,
                vec4(0.0, 0.2, 0.2, 1.0),
            );
        }
    }

    fn draw_next_action(
        &mut self,
        ship_data: &mut BlockObject,
        rules: &Rules,
        node_id_bits: &mut Vec<u32>,
        render_nodes: &mut Vec<RenderNode>,
    ) {
        let mut to_reset = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .to_reset
            .to_owned();
        if !to_reset.is_empty() {
            // Querry Result
            let order = to_reset.pop_front().unwrap();
            let (block_name_index, block_index, chunk_index) =
                ship_data.order_controller.unpack_propergate_order(order);

            let block_pos =
                ship_data.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

            let node_pos = ship_data.get_node_pos_from_block_pos(block_pos);
            self.add_cube(
                node_pos.as_vec3() - 0.12,
                node_pos.as_vec3() + 2.12,
                vec4(0.0, 0.0, 0.0, 1.0),
            );

            if chunk_index != 0 {
                return;
            }

            let cache = rules.solvers[block_name_index as usize].debug_block_check_reset(
                ship_data,
                block_index,
                chunk_index,
                block_pos,
            );

            if cache.is_empty() {
                return;
            }

            let (cache_index, req_results) =
                &cache[self.collapse_log_renderer.preview_index % cache.len()];

            let block =
                rules.solvers[block_name_index as usize].get_block_from_cache_index(*cache_index);

            // Draw Block
            let node_pos = ship_data.get_node_pos_from_block_pos(block_pos);
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

            for (node_id, (index, index_with_padding)) in
                block.node_ids.into_iter().zip(indices.into_iter())
            {
                node_id_bits[index] = node_id.into();
                render_nodes[index_with_padding] = RenderNode(node_id.is_some());
            }

            // Draw Reqs
            for (req_pos, ok) in req_results.into_iter() {
                let pos = ship_data.get_node_pos_from_block_pos(*req_pos);
                self.add_cube(
                    pos.as_vec3() - 0.11,
                    pos.as_vec3() + 2.11,
                    if *ok {
                        vec4(0.0, 1.0, 0.0, 1.0)
                    } else {
                        vec4(1.0, 0.0, 0.0, 1.0)
                    },
                );
            }

            return;
        }

        let mut to_propergate = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .to_propergate
            .to_owned();
        if !to_propergate.is_empty() {
            // Querry Result
            let order = to_propergate.pop_front().unwrap();
            let (block_name_index, block_index, chunk_index) =
                ship_data.order_controller.unpack_propergate_order(order);

            let block_pos =
                ship_data.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

            let node_pos = ship_data.get_node_pos_from_block_pos(block_pos);
            self.add_cube(
                node_pos.as_vec3() - 0.12,
                node_pos.as_vec3() + 2.12,
                vec4(0.0, 0.0, 0.0, 1.0),
            );

            if chunk_index != 0 {
                return;
            }

            let debug_cache = rules.solvers[block_name_index as usize].debug_block_check(
                ship_data,
                block_index,
                chunk_index,
                block_pos,
                &self.collapse_log_renderer.block_log
                    [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
                    .blocks,
            );

            if debug_cache.is_empty() {
                return;
            }

            let (cache_index, req_results) =
                &debug_cache[self.collapse_log_renderer.preview_index % debug_cache.len()];

            let block =
                rules.solvers[block_name_index as usize].get_block_from_cache_index(*cache_index);

            // Draw Block
            let node_pos = ship_data.get_node_pos_from_block_pos(block_pos);
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

            for (node_id, (index, index_with_padding)) in
                block.node_ids.into_iter().zip(indices.into_iter())
            {
                node_id_bits[index] = node_id.into();
                render_nodes[index_with_padding] = RenderNode(node_id.is_some());
            }

            // Draw Reqs
            for (req_pos, ok) in req_results.into_iter() {
                let pos = ship_data.get_node_pos_from_block_pos(*req_pos);
                self.add_cube(
                    pos.as_vec3() - 0.11,
                    pos.as_vec3() + 2.11,
                    if *ok {
                        vec4(0.0, 1.0, 0.0, 1.0)
                    } else {
                        vec4(1.0, 0.0, 0.0, 1.0)
                    },
                );
            }

            return;
        }

        let mut to_collapse = self.collapse_log_renderer.block_log
            [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
            .collapser
            .to_owned();
        if !to_collapse.is_empty() {
            // Querry Result
            let order = to_collapse.pop_order();
            let (block_index, chunk_index) =
                ship_data.order_controller.unpack_collapse_order(order);

            let block_pos =
                ship_data.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

            let node_pos = ship_data.get_node_pos_from_block_pos(block_pos);
            self.add_cube(
                node_pos.as_vec3() - 0.12,
                node_pos.as_vec3() + 2.12,
                vec4(0.0, 0.0, 0.0, 1.0),
            );

            return;
        }
    }

    fn get_collapse_log_node_id_bits(
        &mut self,
        size: IVec3,
        ship_data: &BlockObject,
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

                    // Draw Blocks
                    let caches: Vec<_> = self.collapse_log_renderer.block_log
                        [self.collapse_log_renderer.log_index - self.collapse_log_renderer.log_end]
                        .blocks[block_index]
                        .get_all_caches()
                        .into_iter()
                        .map(|(block_name, cache)| iter::repeat(block_name).zip(cache.into_iter()))
                        .flatten()
                        .collect();

                    if caches.is_empty() {
                        continue;
                    }

                    let (block_name_index, cache_index) =
                        caches[self.collapse_log_renderer.cache_index % caches.len()];

                    let hull_solver = rules.solvers[block_name_index as usize].as_hull().unwrap();

                    let mut block = hull_solver.get_block_from_cache_index(cache_index);

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

                    for (node_id, (index, index_with_padding)) in
                        block.node_ids.into_iter().zip(indices.into_iter())
                    {
                        node_id_bits[index] = node_id.into();
                        render_nodes[index_with_padding] = RenderNode(node_id.is_some());
                    }
                }
            }
        }

        (node_id_bits, render_nodes)
    }
}
