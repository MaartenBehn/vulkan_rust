use crate::math::{get_neighbors, to_3d_i};
use crate::node::{NodeID, NodeIndex, BLOCK_INDEX_EMPTY};
use crate::rules::{Prio, Rules};
use crate::{
    math::{to_1d, to_1d_i, to_3d},
    node::BlockIndex,
};
use index_queue::IndexQueue;
use log::{debug, info};
use octa_force::{anyhow::*, glam::*, log};
use std::cmp::max;

#[cfg(debug_assertions)]
use crate::debug::DebugController;
use crate::ship::mesh::RenderNode;
use crate::ship::node_order::NodeOrderController;
use crate::ship::possible_nodes::{NodeData, PossibleNodes};

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
    pub blocks: Vec<BlockIndex>,
    pub nodes: Vec<PossibleNodes>,
    pub base_nodes: Vec<PossibleNodes>,
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

    pub fn place_block(&mut self, block_pos: IVec3, block_index: BlockIndex, rules: &Rules) {
        let pos = self.get_node_pos_from_block_pos(block_pos);

        let chunk_index = self.get_chunk_index_from_node_pos(pos);
        let in_chunk_block_index = self.get_block_index(pos);
        let chunk = &mut self.chunks[chunk_index];

        let old_block_index = chunk.blocks[in_chunk_block_index];
        if old_block_index == block_index {
            return;
        }

        info!("Place: {block_pos:?}");
        chunk.blocks[in_chunk_block_index] = block_index;

        rules.solvers[old_block_index].push_block_affected_nodes(self, pos);
        rules.solvers[block_index].push_block_affected_nodes(self, pos);

        self.was_reset = IndexQueue::default();
    }

    pub fn get_block_at_pos(&mut self, block_pos: IVec3) -> BlockIndex {
        let pos = self.get_node_pos_from_block_pos(block_pos);

        let chunk_index = self.get_chunk_index_from_node_pos(pos);
        let in_chunk_block_index = self.get_block_index(pos);

        self.chunks[chunk_index].blocks[in_chunk_block_index]
    }

    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        rules: &Rules,
        #[cfg(debug_assertions)] debug: bool,
    ) -> (bool, Vec<ChunkIndex>) {
        let mut changed_chunks = Vec::new();

        for _ in 0..actions_per_tick {
            if !self.block_changed.is_empty() {
                self.block_changed(rules, debug);
            } else if !self.to_reset.is_empty() {
                self.reset(rules);
            } else if !self.to_propergate.is_empty() {
                self.propergate(rules);
            } else if !self.to_collapse.is_empty() {
                let changed_chunk = self.collapse();

                if !changed_chunks.contains(&changed_chunk) {
                    changed_chunks.push(changed_chunk)
                }
            } else {
                return (false, changed_chunks);
            }
        }

        info!("Full Tick: {actions_per_tick}");

        (true, changed_chunks)
    }

    fn block_changed(&mut self, rules: &Rules, debug: bool) {
        let order = self.block_changed.pop_front().unwrap();
        let (block_index, chunk_index, node_index) =
            self.order_controller.unpack_order_with_block(order);
        let pos = self.get_world_pos_from_chunk_and_node_index(chunk_index, node_index);

        let new_base_node_ids =
            rules.solvers[block_index].block_check(self, node_index, chunk_index, pos);

        let old_base_node_ids =
            self.chunks[chunk_index].base_nodes[node_index].get_node_ids(block_index);
        if old_base_node_ids != new_base_node_ids.as_slice() {
            for (chunk_index, node_index) in self.get_neighbor_chunk_and_node_index(pos) {
                self.block_changed
                    .push_back(self.order_controller.pack_order_with_block(
                        block_index,
                        node_index,
                        chunk_index,
                    ));
                self.to_collapse
                    .push_back(self.order_controller.pack_order(node_index, chunk_index));
            }

            self.to_reset.push_back(order);
            self.was_reset.push_back(order);
            self.to_collapse
                .push_back(self.order_controller.pack_order(node_index, chunk_index));

            self.chunks[chunk_index].base_nodes[node_index]
                .set_node_ids(block_index, new_base_node_ids);

            if debug {
                let node_index_plus_padding =
                    self.node_index_to_node_index_plus_padding(node_index);
                self.chunks[chunk_index].render_nodes[node_index_plus_padding] = RenderNode(true);
            }
        }
    }

    fn reset(&mut self, rules: &Rules) {
        let order = self.to_reset.pop_front().unwrap();
        let (block_index, chunk_index, node_index) =
            self.order_controller.unpack_order_with_block(order);
        let pos = self.get_world_pos_from_chunk_and_node_index(chunk_index, node_index);

        let new_node_ids =
            rules.solvers[block_index].node_check_reset(self, node_index, chunk_index, pos);

        let old_node_ids = self.chunks[chunk_index].nodes[node_index].get_node_ids(block_index);
        if new_node_ids != old_node_ids {
            for (chunk_index, node_index) in self.get_neighbor_chunk_and_node_index(pos) {
                let neighbor_order = self.order_controller.pack_order_with_block(
                    block_index,
                    node_index,
                    chunk_index,
                );

                if !self.was_reset.contains(neighbor_order) {
                    self.to_reset.push_back(neighbor_order);
                } else {
                    self.to_propergate.push_back(neighbor_order);
                }
            }

            self.was_reset.push_back(order);
            self.to_propergate.push_back(order);
            self.to_collapse
                .push_back(self.order_controller.pack_order(node_index, chunk_index));

            self.chunks[chunk_index].nodes[node_index].set_node_ids(block_index, new_node_ids);
        }
    }

    fn propergate(&mut self, rules: &Rules) {
        let order = self.to_propergate.pop_front().unwrap();
        let (block_index, chunk_index, node_index) =
            self.order_controller.unpack_order_with_block(order);
        let pos = self.get_world_pos_from_chunk_and_node_index(chunk_index, node_index);

        let new_node_ids =
            rules.solvers[block_index].node_check(self, node_index, chunk_index, pos);

        let old_node_ids = self.chunks[chunk_index].nodes[node_index].get_node_ids(block_index);
        if new_node_ids != old_node_ids {
            for (chunk_index, node_index) in self.get_neighbor_chunk_and_node_index(pos) {
                let neighbor_order = self.order_controller.pack_order_with_block(
                    block_index,
                    node_index,
                    chunk_index,
                );
                self.to_propergate.push_back(neighbor_order);
            }

            self.to_collapse
                .push_back(self.order_controller.pack_order(node_index, chunk_index));

            self.chunks[chunk_index].nodes[node_index].set_node_ids(block_index, new_node_ids);
        }
    }

    fn collapse(&mut self) -> usize {
        let order = self.to_collapse.pop_front().unwrap();
        let (chunk_index, node_index) = self.order_controller.unpack_order(order);
        let node_index_plus_padding = self.node_index_to_node_index_plus_padding(node_index);

        let data = self.chunks[chunk_index].nodes[node_index]
            .get_all()
            .max_by(|data1, data2| data1.prio.cmp(&data2.prio))
            .unwrap_or(&NodeData::default())
            .to_owned();

        self.chunks[chunk_index].node_id_bits[node_index] = data.id.into();
        self.chunks[chunk_index].render_nodes[node_index_plus_padding] =
            RenderNode(!data.id.is_empty());

        chunk_index
    }

    #[cfg(debug_assertions)]
    pub fn show_debug(&self, debug_controller: &mut DebugController) {
        for chunk in self.chunks.iter() {
            debug_controller.add_cube(
                (chunk.pos * self.nodes_per_chunk).as_vec3(),
                ((chunk.pos + IVec3::ONE) * self.nodes_per_chunk).as_vec3(),
                vec4(1.0, 0.0, 0.0, 1.0),
            );
        }

        let mut block_changed = self.block_changed.to_owned();
        while !block_changed.is_empty() {
            let order = block_changed.pop_front().unwrap();
            let (_, chunk_index, node_index) = self.order_controller.unpack_order_with_block(order);
            let pos = self.get_world_pos_from_chunk_and_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3() + vec3(0.0, 0.0, 0.0),
                pos.as_vec3() + Vec3::ONE * 0.1 + vec3(0.0, 0.0, 0.0),
                vec4(1.0, 0.0, 1.0, 1.0),
            );
        }

        let mut to_reset = self.to_reset.to_owned();
        while !to_reset.is_empty() {
            let order = to_reset.pop_front().unwrap();
            let (_, chunk_index, node_index) = self.order_controller.unpack_order_with_block(order);
            let pos = self.get_world_pos_from_chunk_and_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3() + vec3(0.1, 0.0, 0.0),
                pos.as_vec3() + Vec3::ONE * 0.1 + vec3(0.1, 0.0, 0.0),
                vec4(1.0, 1.0, 0.0, 1.0),
            );
        }

        let mut to_propergate = self.to_propergate.to_owned();
        while !to_propergate.is_empty() {
            let order = to_propergate.pop_front().unwrap();
            let (_, chunk_index, node_index) = self.order_controller.unpack_order_with_block(order);
            let pos = self.get_world_pos_from_chunk_and_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3() + vec3(0.2, 0.0, 0.0),
                pos.as_vec3() + Vec3::ONE * 0.1 + vec3(0.2, 0.0, 0.0),
                vec4(0.0, 1.0, 0.0, 1.0),
            );
        }

        let mut to_collapse = self.to_collapse.to_owned();
        while !to_collapse.is_empty() {
            let order = to_collapse.pop_front().unwrap();
            let (chunk_index, node_index) = self.order_controller.unpack_order(order);
            let pos = self.get_world_pos_from_chunk_and_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3() + vec3(0.3, 0.0, 0.0),
                pos.as_vec3() + Vec3::ONE * 0.1 + vec3(0.3, 0.0, 0.0),
                vec4(0.0, 0.0, 1.0, 1.0),
            );
        }
    }

    pub fn add_chunk(&mut self, chunk_pos: IVec3) {
        let chunk = ShipDataChunk {
            pos: chunk_pos,
            blocks: vec![BLOCK_INDEX_EMPTY; self.block_length],
            nodes: vec![PossibleNodes::default(); self.nodes_length],
            base_nodes: vec![PossibleNodes::default(); self.nodes_length],
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
