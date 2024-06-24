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
use log::info;
use octa_force::anyhow::Result;
use octa_force::controls::Controls;
use octa_force::glam::{ivec3, IVec3};
use octa_force::vulkan::{CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};
use std::iter;
use std::time::{Duration, Instant};

const INPUT_INTERVAL: Duration = Duration::from_millis(100);

const CACHE_INDEX_UPDATE_INTERVAL: Duration = Duration::from_millis(1000);

pub struct CollapseLogRenderer {
    mesh: ShipMesh,

    last_blocks_names: Vec<BlockNameIndex>,
    block_log: Vec<Vec<PossibleBlocks>>,
    log_index: usize,
    last_input: Instant,
    cache_index: usize,
    last_index_update: Instant,
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
        }
    }

    pub fn on_enable(&self, ship_manager: &mut ShipManager) {
        ship_manager.update_actions_per_tick = false;
        ship_manager.actions_per_tick = 1;
    }

    pub fn on_disable(&self, ship_manager: &mut ShipManager) {
        ship_manager.update_actions_per_tick = true;
        ship_manager.actions_per_tick = 4;
    }

    fn update(&mut self, ship_data: &ShipData, controls: &Controls) {
        if ship_data.chunks[0].block_names != self.last_blocks_names {
            self.block_log = vec![];
            self.log_index = 0;
            self.last_blocks_names = ship_data.chunks[0].block_names.to_owned();
        }

        if self.block_log.is_empty()
            || ship_data.chunks[0].blocks != *self.block_log.last().unwrap()
        {
            self.block_log.push(ship_data.chunks[0].blocks.to_owned());
        }

        if self.last_input.elapsed() > INPUT_INTERVAL && controls.t {
            self.log_index = self.cache_index + 1 % self.block_log.len();
            self.last_input = Instant::now();

            info!("Log Index: {}", self.log_index);
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

        ship_data: &ShipData,
        controls: &Controls,
        rules: &Rules,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.collapse_log_renderer.update(ship_data, controls);

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
                    let index = ship_data.get_block_index_from_world_block_pos(ivec3(x, y, z));

                    if self.collapse_log_renderer.block_log.is_empty() {
                        continue;
                    }

                    let caches: Vec<_> = self.collapse_log_renderer.block_log
                        [self.collapse_log_renderer.log_index][index]
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

                    let mut block = None;

                    let hull_solver = rules.solvers[block_name_index].to_hull();
                    if hull_solver.is_ok() {
                        block = Some(hull_solver.unwrap().get_block_from_cache_index(cache_index));
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

        (node_id_bits, render_nodes)
    }
}
