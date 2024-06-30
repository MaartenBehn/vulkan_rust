use crate::math::{get_neighbors, oct_positions, to_3d_i};
use crate::math::{to_1d, to_1d_i, to_3d};
use crate::node::{NodeID, NodeIndex};
use crate::rules::{Prio, Rules};
use crate::ship::mesh::RenderNode;
use crate::ship::order::NodeOrderController;
use crate::ship::possible_blocks::PossibleBlocks;

use crate::rules::block::{BlockIndex, BlockNameIndex, BLOCK_INDEX_EMPTY};
use crate::rules::empty::EMPTY_BLOCK_NAME_INDEX;
use crate::rules::solver::SolverCacheIndex;
use crate::ship::collapse::Collapser;
use index_queue::IndexQueue;
use log::{debug, info};
use octa_force::{anyhow::*, glam::*, log};

pub type ChunkIndex = usize;
pub type CacheIndex = usize;

#[derive(Clone)]
pub struct ShipData {
    pub chunks: Vec<ShipDataChunk>,

    pub blocks_per_chunk: IVec3,
    pub block_length: usize,

    pub nodes_per_chunk: IVec3,
    pub nodes_length: usize,

    pub nodes_per_chunk_with_padding: IVec3,
    pub nodes_length_with_padding: usize,

    pub chunk_block_pos_mask: IVec3,
    pub in_chunk_block_pos_mask: IVec3,

    pub order_controller: NodeOrderController,

    pub to_reset: IndexQueue,
    pub was_reset: IndexQueue,
    pub to_propergate: IndexQueue,

    pub collapser: Collapser,
    pub is_collapsed: IndexQueue,
}

#[derive(Clone)]
pub struct ShipDataChunk {
    pub pos: IVec3,
    pub block_names: Vec<BlockNameIndex>,
    pub blocks: Vec<PossibleBlocks>,
    pub node_id_bits: Vec<u32>,
    pub render_nodes: Vec<RenderNode>,
}

impl ShipData {
    pub fn new(node_size: i32, rules: &Rules) -> ShipData {
        let block_size = node_size / 2;
        let blocks_per_chunk = IVec3::ONE * block_size;
        let block_length = blocks_per_chunk.element_product() as usize;

        let nodes_per_chunk = IVec3::ONE * node_size;
        let nodes_length = nodes_per_chunk.element_product() as usize;

        let nodes_per_chunk_with_padding = IVec3::ONE * (node_size + 2);
        let nodes_length_with_padding = nodes_per_chunk_with_padding.element_product() as usize;

        let chunk_pos_mask = IVec3::ONE * !(block_size - 1);
        let in_chunk_pos_mask = IVec3::ONE * (block_size - 1);

        let node_order_controller = NodeOrderController::new(rules.block_names.len(), nodes_length);

        let mut ship = ShipData {
            chunks: Vec::new(),

            blocks_per_chunk,
            block_length,

            nodes_per_chunk,
            nodes_length,

            nodes_per_chunk_with_padding,
            nodes_length_with_padding,

            chunk_block_pos_mask: chunk_pos_mask,
            in_chunk_block_pos_mask: in_chunk_pos_mask,

            order_controller: node_order_controller,

            to_reset: IndexQueue::default(),
            was_reset: IndexQueue::default(),
            to_propergate: IndexQueue::default(),
            collapser: Collapser::new(),
            is_collapsed: IndexQueue::default(),
        };

        //ship.place_block(IVec3::ZERO, 1, rules);

        ship
    }

    pub fn place_block(
        &mut self,
        world_block_pos: IVec3,
        new_block_name_index: BlockNameIndex,
        rules: &Rules,
    ) {
        let chunk_index = self.get_chunk_index_from_world_block_pos(world_block_pos);
        let block_index = self.get_block_index_from_world_block_pos(world_block_pos);
        let chunk = &mut self.chunks[chunk_index];

        let old_block_name_index = chunk.block_names[block_index];
        if old_block_name_index == new_block_name_index {
            return;
        }

        info!("Place: {world_block_pos:?}");
        chunk.block_names[block_index] = new_block_name_index;

        let old_order = self.order_controller.pack_propergate_order(
            old_block_name_index,
            block_index,
            chunk_index,
        );
        let new_order = self.order_controller.pack_propergate_order(
            new_block_name_index,
            block_index,
            chunk_index,
        );
        self.to_reset.push_back(old_order);
        self.to_reset.push_back(new_order);

        self.was_reset = IndexQueue::default();
        self.is_collapsed = IndexQueue::default();

        let collapse_order = self
            .order_controller
            .pack_collapse_order(block_index, chunk_index);
        let neighbor_cache_len = self.chunks[chunk_index].blocks[block_index].get_num_caches();

        self.collapser
            .push_order(collapse_order, neighbor_cache_len);
    }

    pub fn get_block_name_from_world_block_pos(
        &mut self,
        world_block_pos: IVec3,
    ) -> BlockNameIndex {
        let chunk_index = self.get_chunk_index_from_world_block_pos(world_block_pos);
        let in_chunk_block_index = self.get_block_index_from_world_block_pos(world_block_pos);

        self.chunks[chunk_index].block_names[in_chunk_block_index]
    }

    pub fn get_cache_from_world_block_pos(
        &mut self,
        world_block_pos: IVec3,
        block_name_index: BlockNameIndex,
    ) -> &[SolverCacheIndex] {
        let chunk_index = self.get_chunk_index_from_world_block_pos(world_block_pos);
        let in_chunk_block_index = self.get_block_index_from_world_block_pos(world_block_pos);

        self.chunks[chunk_index].blocks[in_chunk_block_index].get_cache(block_name_index)
    }

    pub fn tick(&mut self, actions_per_tick: usize, rules: &Rules) -> (bool, Vec<ChunkIndex>) {
        let mut changed_chunks = Vec::new();

        for _ in 0..actions_per_tick {
            if !self.to_reset.is_empty() {
                self.reset(rules);
            } else if !self.to_propergate.is_empty() {
                self.propergate(rules);
            } else if !self.collapser.is_empty() {
                let changed_chunk = self.collapse(rules);

                if !changed_chunks.contains(&changed_chunk) {
                    changed_chunks.push(changed_chunk)
                }
            } else {
                return (false, changed_chunks);
            }
        }

        (true, changed_chunks)
    }

    fn reset(&mut self, rules: &Rules) {
        let order = self.to_reset.pop_front().unwrap();
        let (block_name_index, block_index, chunk_index) =
            self.order_controller.unpack_propergate_order(order);
        let world_block_pos =
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

        let new_cache = rules.solvers[block_name_index].block_check_reset(
            self,
            block_index,
            chunk_index,
            world_block_pos,
        );
        let old_cache = self.chunks[chunk_index].blocks[block_index].get_cache(block_name_index);
        if new_cache != old_cache {
            self.chunks[chunk_index].blocks[block_index].set_cache(block_name_index, &new_cache);

            self.to_propergate.push_back(order);
            self.was_reset.push_back(order);

            for offset in get_neighbors() {
                let neighbor_world_pos = world_block_pos + offset;
                let neighbor_chunk_index =
                    self.get_chunk_index_from_world_block_pos(neighbor_world_pos);
                let neighbor_block_index =
                    self.get_block_index_from_world_block_pos(neighbor_world_pos);
                let neighbor_order = self.order_controller.pack_propergate_order(
                    block_name_index,
                    neighbor_block_index,
                    neighbor_chunk_index,
                );

                if !self.was_reset.contains(neighbor_order) {
                    self.to_reset.push_back(neighbor_order);
                }
            }
        }
    }

    fn propergate(&mut self, rules: &Rules) {
        let order = self.to_propergate.pop_front().unwrap();
        let (block_name_index, block_index, chunk_index) =
            self.order_controller.unpack_propergate_order(order);
        let world_block_pos =
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

        let old_cache = self.chunks[chunk_index].blocks[block_index]
            .get_cache(block_name_index)
            .to_owned();
        let new_cache = rules.solvers[block_name_index].block_check(
            self,
            block_index,
            chunk_index,
            world_block_pos,
            old_cache.to_owned(),
        );

        if new_cache != old_cache {
            self.chunks[chunk_index].blocks[block_index].set_cache(block_name_index, &new_cache);

            for offset in get_neighbors() {
                let neighbor_world_pos = world_block_pos + offset;
                let neighbor_chunk_index =
                    self.get_chunk_index_from_world_block_pos(neighbor_world_pos);
                let neighbor_block_index =
                    self.get_block_index_from_world_block_pos(neighbor_world_pos);

                let collapse_order = self
                    .order_controller
                    .pack_collapse_order(neighbor_block_index, neighbor_chunk_index);
                if self.is_collapsed.contains(collapse_order) {
                    continue;
                }

                let propergate_order = self.order_controller.pack_propergate_order(
                    block_name_index,
                    neighbor_block_index,
                    neighbor_chunk_index,
                );
                self.to_propergate.push_back(propergate_order);
            }
        }
    }

    fn collapse(&mut self, rules: &Rules) -> ChunkIndex {
        let order = self.collapser.pop_order();
        let (block_index, chunk_index) = self.order_controller.unpack_collapse_order(order);
        let world_block_pos =
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index);

        // Get best Block
        let mut best_block = None;
        let mut best_prio = Prio::ZERO;
        let mut best_block_name_index = EMPTY_BLOCK_NAME_INDEX;
        let mut best_cache_index = 0;
        for (block_name_index, solver) in rules.solvers.iter().enumerate() {
            let old_cache = self.chunks[chunk_index].blocks[block_index]
                .get_cache(block_name_index)
                .to_owned();
            let (block, prio, cache_index) =
                solver.get_block(self, block_index, chunk_index, world_block_pos, old_cache);

            if best_prio < prio {
                best_prio = prio;
                best_block = Some(block);
                best_block_name_index = block_name_index;
                best_cache_index = cache_index;
            }
        }

        // Update Cache to the chosen index
        self.chunks[chunk_index].blocks[block_index]
            .set_all_caches_with_one(best_block_name_index, best_cache_index);

        // Set node_id and render nodes
        let node_pos = self.get_node_pos_from_block_index(block_index);
        let indices: Vec<_> = oct_positions()
            .into_iter()
            .map(|offset| {
                let pos = node_pos + offset;
                let index = self.get_node_index_from_node_pos(pos);
                let index_with_padding = self.get_node_index_with_padding_from_node_pos(pos);
                (index, index_with_padding)
            })
            .collect();

        if best_block.is_some() {
            let block = best_block.unwrap();
            for (node_id, (index, index_with_padding)) in
                block.node_ids.into_iter().zip(indices.into_iter())
            {
                self.chunks[chunk_index].node_id_bits[index] = node_id.into();
                self.chunks[chunk_index].render_nodes[index_with_padding] =
                    RenderNode(node_id.is_some());
            }
        } else {
            for (index, index_with_padding) in indices.into_iter() {
                self.chunks[chunk_index].node_id_bits[index] = NodeID::empty().into();
                self.chunks[chunk_index].render_nodes[index_with_padding] = RenderNode(false);
            }
        }

        self.is_collapsed.push_back(order);

        for offset in get_neighbors() {
            let neighbor_world_pos = world_block_pos + offset;
            let neighbor_chunk_index =
                self.get_chunk_index_from_world_block_pos(neighbor_world_pos);
            let neighbor_block_index =
                self.get_block_index_from_world_block_pos(neighbor_world_pos);

            let collapse_order = self
                .order_controller
                .pack_collapse_order(neighbor_block_index, neighbor_chunk_index);
            if self.is_collapsed.contains(collapse_order) {
                continue;
            }

            let propergate_order = self.order_controller.pack_propergate_order(
                best_block_name_index,
                neighbor_block_index,
                neighbor_chunk_index,
            );
            self.to_propergate.push_back(propergate_order);

            if self.get_block_name_from_world_block_pos(neighbor_world_pos)
                == EMPTY_BLOCK_NAME_INDEX
            {
                continue;
            }

            let collapse_order = self
                .order_controller
                .pack_collapse_order(neighbor_block_index, neighbor_chunk_index);
            let neighbor_cache_len =
                self.chunks[neighbor_chunk_index].blocks[neighbor_block_index].get_num_caches();

            self.collapser
                .push_order(collapse_order, neighbor_cache_len);
        }

        chunk_index
    }

    pub fn add_chunk(&mut self, chunk_pos: IVec3) {
        let chunk = ShipDataChunk {
            pos: chunk_pos,
            block_names: vec![BLOCK_INDEX_EMPTY; self.block_length],
            blocks: vec![PossibleBlocks::default(); self.block_length],
            node_id_bits: vec![0; self.nodes_length],
            render_nodes: vec![RenderNode(false); self.nodes_length_with_padding],
        };

        self.chunks.push(chunk)
    }

    pub fn has_chunk(&self, chunk_pos: IVec3) -> bool {
        self.chunks.iter().find(|c| c.pos == chunk_pos).is_some()
    }

    pub fn get_chunk_index_from_world_block_pos(&mut self, world_block_pos: IVec3) -> usize {
        let chunk_pos = self.get_chunk_pos_from_world_block_pos(world_block_pos);

        let r = self.chunks.iter().position(|c| c.pos == chunk_pos);
        let index = if r.is_none() {
            self.add_chunk(chunk_pos);
            self.chunks.len() - 1
        } else {
            r.unwrap()
        };

        index
    }

    pub fn get_chunk_pos_from_world_block_pos(&self, world_block_pos: IVec3) -> IVec3 {
        ((world_block_pos / self.blocks_per_chunk)
            - ivec3(
                (world_block_pos.x < 0) as i32,
                (world_block_pos.y < 0) as i32,
                (world_block_pos.z < 0) as i32,
            ))
            * self.nodes_per_chunk
    }

    pub fn get_chunk_pos_from_world_node_pos(&self, world_block_pos: IVec3) -> IVec3 {
        ((world_block_pos / self.nodes_per_chunk)
            - ivec3(
                (world_block_pos.x < 0) as i32,
                (world_block_pos.y < 0) as i32,
                (world_block_pos.z < 0) as i32,
            ))
            * self.nodes_per_chunk
    }

    pub fn get_block_index_from_world_block_pos(&self, world_block_pos: IVec3) -> usize {
        let block_pos = self.get_block_pos_from_world_block_pos(world_block_pos);
        to_1d_i(block_pos, self.blocks_per_chunk) as usize
    }

    pub fn get_block_pos_from_world_block_pos(&self, pos: IVec3) -> IVec3 {
        pos & self.in_chunk_block_pos_mask
    }

    pub fn get_world_block_pos_from_chunk_and_block_index(
        &self,
        block_index: usize,
        chunk_index: usize,
    ) -> IVec3 {
        let chunk_pos = self.chunks[chunk_index].pos / 2;
        let block_pos = to_3d_i(block_index as i32, self.blocks_per_chunk);

        chunk_pos + block_pos
    }

    pub fn get_world_node_pos_from_chunk_and_block_index(
        &self,
        block_index: usize,
        chunk_index: usize,
    ) -> IVec3 {
        self.get_node_pos_from_block_pos(
            self.get_world_block_pos_from_chunk_and_block_index(block_index, chunk_index),
        )
    }

    pub fn get_node_pos_from_block_pos(&self, block_pos: IVec3) -> IVec3 {
        block_pos * 2
    }

    pub fn get_node_pos_from_block_index(&self, block_index: usize) -> IVec3 {
        let block_pos = to_3d_i(block_index as i32, self.blocks_per_chunk);
        let node_pos = self.get_node_pos_from_block_pos(block_pos);
        node_pos
    }

    pub fn get_node_index_from_node_pos(&self, node_pos: IVec3) -> usize {
        to_1d_i(node_pos, self.nodes_per_chunk) as usize
    }

    pub fn get_node_index_with_padding_from_node_pos(&self, node_pos: IVec3) -> usize {
        to_1d_i(node_pos + 1, self.nodes_per_chunk_with_padding) as usize
    }

    pub fn get_world_node_pos_from_chunk_and_node_index(
        &self,
        node_index: usize,
        chunk_index: usize,
    ) -> IVec3 {
        let chunk_pos = self.chunks[chunk_index].pos;
        let node_pos = to_3d_i(node_index as i32, self.nodes_per_chunk);

        chunk_pos + node_pos
    }

    pub fn node_index_to_node_index_plus_padding(&self, node_index: usize) -> usize {
        let node_pos = to_3d_i(node_index as i32, self.nodes_per_chunk);
        to_1d_i(node_pos + IVec3::ONE, self.nodes_per_chunk_with_padding) as usize
    }

    pub fn block_world_pos_from_in_chunk_block_index(
        &self,
        block_index: usize,
        chunk_pos: IVec3,
    ) -> IVec3 {
        to_3d_i(block_index as i32, self.blocks_per_chunk) + chunk_pos
    }

    /*
    fn get_neighbor_chunk_and_node_index(
        &mut self,
        pos: IVec3,
    ) -> impl Iterator<Item = (usize, usize)> {
        get_neighbors()
            .map(|offset| {
                let neighbor_pos = pos + offset;
                let chunk_index = self.get_chunk_index_from_node_pos(neighbor_pos);
                let node_index = self.get_node_index_from_node_pos(neighbor_pos);

                (chunk_index, node_index)
            })
            .into_iter()
    }
     */
}
