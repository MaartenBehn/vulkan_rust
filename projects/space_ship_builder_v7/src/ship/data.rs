use crate::math::{get_neighbors, to_3d_i};
use crate::math::{to_1d, to_1d_i, to_3d};
use crate::node::{NodeID, NodeIndex};
use crate::rules::{Prio, Rules};
use crate::ship::mesh::RenderNode;
use crate::ship::order::NodeOrderController;
use crate::ship::possible_blocks::PossibleBlocks;

use crate::rules::block::{BlockNameIndex, BLOCK_INDEX_EMPTY};
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

    pub chunk_pos_mask: IVec3,
    pub in_chunk_pos_mask: IVec3,

    pub order_controller: NodeOrderController,

    pub block_changed: IndexQueue,
    pub to_reset: IndexQueue,
    pub was_reset: IndexQueue,
    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,
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
        let blocks_per_chunk = IVec3::ONE * node_size / 2;
        let block_length = blocks_per_chunk.element_product() as usize;

        let nodes_per_chunk = IVec3::ONE * node_size;
        let nodes_length = nodes_per_chunk.element_product() as usize;

        let nodes_per_chunk_with_padding = IVec3::ONE * (node_size + 2);
        let nodes_length_with_padding = nodes_per_chunk_with_padding.element_product() as usize;

        let chunk_pos_mask = IVec3::ONE * !(node_size - 1);
        let in_chunk_pos_mask = IVec3::ONE * (node_size - 1);

        let node_order_controller = NodeOrderController::new(rules.block_names.len(), nodes_length);

        let mut ship = ShipData {
            chunks: Vec::new(),

            blocks_per_chunk,
            block_length,

            nodes_per_chunk,
            nodes_length,

            nodes_per_chunk_with_padding,
            nodes_length_with_padding,

            chunk_pos_mask,
            in_chunk_pos_mask,

            order_controller: node_order_controller,

            block_changed: IndexQueue::default(),
            to_reset: IndexQueue::default(),
            was_reset: IndexQueue::default(),
            to_propergate: IndexQueue::default(),
            to_collapse: IndexQueue::default(),
        };

        ship
    }

    pub fn place_block(&mut self, block_pos: IVec3, block_index: BlockNameIndex, rules: &Rules) {
        let pos = self.get_node_pos_from_block_pos(block_pos);

        let chunk_index = self.get_chunk_index_from_node_pos(pos);
        let in_chunk_block_index = self.get_block_index(pos);
        let chunk = &mut self.chunks[chunk_index];

        let old_block_index = chunk.block_names[in_chunk_block_index];
        if old_block_index == block_index {
            return;
        }

        info!("Place: {block_pos:?}");
        chunk.block_names[in_chunk_block_index] = block_index;

        self.was_reset = IndexQueue::default();
    }

    pub fn get_block_at_pos(&mut self, block_pos: IVec3) -> BlockNameIndex {
        let pos = self.get_node_pos_from_block_pos(block_pos);

        let chunk_index = self.get_chunk_index_from_node_pos(pos);
        let in_chunk_block_index = self.get_block_index(pos);

        self.chunks[chunk_index].block_names[in_chunk_block_index]
    }

    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        rules: &Rules,
        #[cfg(debug_assertions)] debug: bool,
    ) -> (bool, Vec<ChunkIndex>) {
        let mut changed_chunks = Vec::new();

        (true, changed_chunks)
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

    pub fn get_chunk_index_from_node_pos(&mut self, node_pos: IVec3) -> usize {
        let chunk_pos = self.get_chunk_pos_from_node_pos(node_pos);

        let r = self.chunks.iter().position(|c| c.pos == chunk_pos);
        let index = if r.is_none() {
            self.add_chunk(chunk_pos);
            self.chunks.len() - 1
        } else {
            r.unwrap()
        };

        index
    }

    pub fn get_node_pos_from_block_pos(&self, block_pos: IVec3) -> IVec3 {
        block_pos * 2
    }

    pub fn get_chunk_pos_from_node_pos(&self, node_pos: IVec3) -> IVec3 {
        ((node_pos / self.nodes_per_chunk)
            - ivec3(
                (node_pos.x < 0) as i32,
                (node_pos.y < 0) as i32,
                (node_pos.z < 0) as i32,
            ))
            * self.nodes_per_chunk
    }

    pub fn get_in_chunk_pos(&self, pos: IVec3) -> IVec3 {
        pos & self.in_chunk_pos_mask
    }

    pub fn get_block_index(&self, pos: IVec3) -> usize {
        let in_chunk_index = self.get_in_chunk_pos(pos);
        to_1d_i(in_chunk_index / 2, self.blocks_per_chunk) as usize
    }

    pub fn get_node_index(&self, pos: IVec3) -> usize {
        let in_chunk_pos = self.get_in_chunk_pos(pos);
        to_1d_i(in_chunk_pos, self.nodes_per_chunk) as usize
    }

    pub fn get_world_pos_from_chunk_and_node_index(
        &self,
        chunk_index: usize,
        node_index: usize,
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

    fn get_neighbor_chunk_and_node_index(
        &mut self,
        pos: IVec3,
    ) -> impl Iterator<Item = (usize, usize)> {
        get_neighbors()
            .map(|offset| {
                let neighbor_pos = pos + offset;
                let chunk_index = self.get_chunk_index_from_node_pos(neighbor_pos);
                let node_index = self.get_node_index(neighbor_pos);

                (chunk_index, node_index)
            })
            .into_iter()
    }
}
