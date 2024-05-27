use crate::math::{get_neighbors, get_packed_index, to_3d_i};
use crate::node::{Node, NodeID, NodeIndex, PatternIndex, EMPYT_PATTERN_INDEX};
use crate::rules::Rules;
use crate::{
    math::{to_1d, to_1d_i, to_3d},
    node::{BlockIndex, BLOCK_INDEX_EMPTY},
};
use index_queue::IndexQueue;
use log::{debug, info};
use octa_force::{anyhow::*, glam::*, log};
use std::cmp::max;

#[cfg(debug_assertions)]
use crate::debug::DebugController;
use crate::ship::mesh::RenderNode;

pub type ChunkIndex = usize;

#[derive(Clone)]
pub struct ShipData {
    pub chunks: Vec<ShipDataChunk>,

    pub blocks_per_chunk: IVec3,
    pub nodes_per_chunk: IVec3,
    pub chunk_pos_mask: IVec3,
    pub in_chunk_pos_mask: IVec3,
    pub node_index_bits: usize,
    pub node_index_mask: usize,

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
    pub nodes: Vec<Vec<(usize, NodeID, usize)>>,
    pub nodes_base: Vec<Vec<(usize, NodeID, usize)>>,
    pub node_id_bits: Vec<u32>,
    pub render_nodes: Vec<RenderNode>,
}

impl ShipData {
    pub fn new(node_size: i32) -> ShipData {
        let blocks_per_chunk = IVec3::ONE * node_size / 2;
        let nodes_per_chunk = IVec3::ONE * node_size;
        let chunk_pos_mask = IVec3::ONE * !(node_size - 1);
        let in_chunk_pos_mask = IVec3::ONE * (node_size - 1);
        let node_index_bits = (nodes_per_chunk.element_product().trailing_zeros() + 1) as usize;
        let node_index_mask = (nodes_per_chunk.element_product() - 1) as usize;

        let mut ship = ShipData {
            chunks: Vec::new(),

            blocks_per_chunk,
            nodes_per_chunk,
            chunk_pos_mask,
            in_chunk_pos_mask,
            node_index_bits,
            node_index_mask,

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

        let mut push_blocks_changed = |block_index: BlockIndex, pos: IVec3| {
            if block_index == BLOCK_INDEX_EMPTY {
                return;
            }

            for offset in rules.affected_by_block[block_index].iter() {
                let affected_pos = pos + *offset;

                let chunk_index = self.get_chunk_index_from_node_pos(affected_pos);

                let node_index = self.get_node_index(affected_pos);

                let node_world_index = self.to_world_node_index(chunk_index, node_index);
                self.block_changed.push_back(node_world_index);
            }
        };

        push_blocks_changed(old_block_index, pos);
        push_blocks_changed(block_index, pos);

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
                #[cfg(debug_assertions)]
                self.block_changed(rules, debug);

                #[cfg(not(debug_assertions))]
                self.block_changed(rules);
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

    fn block_changed(&mut self, rules: &Rules, #[cfg(debug_assertions)] debug: bool) {
        let node_world_index = self.block_changed.pop_front().unwrap();
        let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
        let pos = self.pos_from_world_node_index(chunk_index, node_index);

        let mut new_base_possible_node_ids = Vec::new();
        for (i, (node_ids, block_reqs)) in rules
            .map_rules_index_to_node_id
            .iter()
            .zip(rules.block_rules.iter())
            .enumerate()
        {
            let node_id = node_ids[0];

            let mut block_accepted = false;
            let mut block_prio = 0;

            // Go over all possible nodes
            for (req, prio) in block_reqs {
                let mut check_performed = false;
                let mut accepted = true;
                // Go over all offsets of the requirement
                for (offset, id) in req.iter() {
                    let test_pos = pos + *offset;

                    // If the offset does not aling with the node just ignore it.
                    if (test_pos % 2) != IVec3::ZERO {
                        continue;
                    }

                    let test_chunk_index = self.get_chunk_index_from_node_pos(test_pos);
                    let test_block_index = self.get_block_index(test_pos);

                    let index = self.chunks[test_chunk_index].blocks[test_block_index].to_owned();

                    // Check if the Block at the pos is in the allowed id.
                    accepted &= *id == index;
                    check_performed = true;
                }

                if accepted && check_performed {
                    block_accepted = true;
                    block_prio = max(block_prio, *prio);
                    break;
                }
            }

            if block_accepted {
                new_base_possible_node_ids.push((i, node_id.to_owned(), block_prio));
            }
        }

        let old_possible_node_ids = self.chunks[chunk_index].nodes_base[node_index].to_owned();
        if new_base_possible_node_ids != old_possible_node_ids {
            for node_world_index in self.get_neighbor_world_node_index(pos) {
                self.block_changed.push_back(node_world_index);
                self.to_collapse.push_back(node_world_index);
            }

            self.to_reset.push_back(node_world_index);
            self.was_reset.push_back(node_world_index);
            self.to_collapse.push_back(node_world_index);

            #[cfg(debug_assertions)]
            if debug {
                let node_index_plus_padding =
                    self.node_index_to_node_index_plus_padding(node_index);
                self.chunks[chunk_index].render_nodes[node_index_plus_padding] = RenderNode(true);
            }

            self.chunks[chunk_index].nodes_base[node_index] = new_base_possible_node_ids;
        }
    }

    fn propergate_node_world_index(
        &mut self,
        pos: IVec3,
        chunk_index: ChunkIndex,
        node_index: usize,
        rules: &Rules,
        reset_nodes: bool,
    ) -> Vec<(usize, NodeID, usize)> {
        let mut new_possible_node_ids = Vec::new();

        let possible_node_ids = if reset_nodes {
            self.chunks[chunk_index].nodes_base[node_index].to_owned()
        } else {
            self.chunks[chunk_index].nodes[node_index].to_owned()
        };

        for (i, node_id, prio) in possible_node_ids.iter() {
            let node_req = &rules.node_rules[*i];

            let mut node_accepted = true;
            for (offset, req_ids) in node_req.iter() {
                let test_pos = pos + *offset;

                let test_chunk_index = self.get_chunk_index_from_node_pos(test_pos);
                let test_node_index = self.get_node_index(test_pos);

                let mut req_ids_contains_empty = false;
                let mut req_ids_contains_any = false;
                for req_node in req_ids {
                    if req_node.is_empty() {
                        req_ids_contains_empty = true;
                    }
                    if req_node.is_any() {
                        req_ids_contains_any = true;
                    }
                    if req_ids_contains_empty && req_ids_contains_any {
                        break;
                    }
                }

                let test_nodes = if reset_nodes {
                    &self.chunks[test_chunk_index].nodes_base[test_node_index]
                } else {
                    &self.chunks[test_chunk_index].nodes[test_node_index]
                };

                let mut found = false;
                if test_nodes.is_empty() && req_ids_contains_empty {
                    found = true;
                } else if req_ids_contains_any {
                    found = test_nodes.iter().any(|(_, node, _)| !node.is_empty())
                } else {
                    for (_, test_id, _) in test_nodes {
                        if req_ids.contains(&test_id) {
                            found = true;
                            break;
                        }
                    }
                }

                node_accepted &= found;
            }

            if node_accepted {
                new_possible_node_ids.push((*i, node_id.to_owned(), *prio));
            }
        }

        return new_possible_node_ids;
    }

    fn reset(&mut self, rules: &Rules) {
        let node_world_index = self.to_reset.pop_front().unwrap();
        let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
        let pos = self.pos_from_world_node_index(chunk_index, node_index);

        let new_possible_node_ids =
            self.propergate_node_world_index(pos, chunk_index, node_index, rules, true);

        let old_possible_node_ids = self.chunks[chunk_index].nodes[node_index].to_owned();
        if new_possible_node_ids != old_possible_node_ids {
            for node_world_index in self.get_neighbor_world_node_index(pos) {
                if !self.was_reset.contains(node_world_index) {
                    self.to_reset.push_back(node_world_index);
                } else {
                    self.to_propergate.push_back(node_world_index);
                }
            }

            self.was_reset.push_back(node_world_index);
            self.to_propergate.push_back(node_world_index);
            self.to_collapse.push_back(node_world_index);

            self.chunks[chunk_index].nodes[node_index] = new_possible_node_ids;
        }
    }

    fn propergate(&mut self, rules: &Rules) {
        let node_world_index = self.to_propergate.pop_front().unwrap();
        let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
        let pos = self.pos_from_world_node_index(chunk_index, node_index);

        let new_possible_node_ids =
            self.propergate_node_world_index(pos, chunk_index, node_index, rules, false);

        let old_possible_node_ids = self.chunks[chunk_index].nodes[node_index].to_owned();
        if new_possible_node_ids != *old_possible_node_ids {
            for node_world_index in self.get_neighbor_world_node_index(pos) {
                self.to_propergate.push_back(node_world_index);
            }

            self.to_collapse.push_back(node_world_index);

            self.chunks[chunk_index].nodes[node_index] = new_possible_node_ids;
        }
    }

    fn collapse(&mut self) -> usize {
        let node_world_index = self.to_collapse.pop_front().unwrap();
        let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
        let node_index_plus_padding = self.node_index_to_node_index_plus_padding(node_index);

        let possible_node_ids = &self.chunks[chunk_index].nodes[node_index];

        let (_, node_id, _) = possible_node_ids
            .iter()
            .max_by(|(_, _, prio1), (_, _, prio2)| prio1.cmp(prio2))
            .unwrap_or(&(0, NodeID::empty(), 0))
            .to_owned();
        self.chunks[chunk_index].node_id_bits[node_index] = node_id.into();
        self.chunks[chunk_index].render_nodes[node_index_plus_padding] =
            RenderNode(!node_id.is_empty());

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
            let node_world_index = block_changed.pop_front().unwrap();
            let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
            let pos = self.pos_from_world_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3(),
                pos.as_vec3() + Vec3::ONE,
                vec4(1.0, 0.0, 1.0, 1.0),
            );
        }

        let mut to_reset = self.to_reset.to_owned();

        while !to_reset.is_empty() {
            let node_world_index = to_reset.pop_front().unwrap();
            let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
            let pos = self.pos_from_world_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3(),
                pos.as_vec3() + Vec3::ONE,
                vec4(1.0, 0.0, 0.0, 1.0),
            );
        }

        let mut to_propergate = self.to_propergate.to_owned();

        while !to_propergate.is_empty() {
            let node_world_index = to_propergate.pop_front().unwrap();
            let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
            let pos = self.pos_from_world_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3() + Vec3::ONE * 0.01,
                pos.as_vec3() + Vec3::ONE * 0.99,
                vec4(0.0, 1.0, 0.0, 1.0),
            );
        }

        let mut to_collapse = self.to_collapse.to_owned();

        while !to_collapse.is_empty() {
            let node_world_index = to_collapse.pop_front().unwrap();
            let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
            let pos = self.pos_from_world_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3() + Vec3::ONE * 0.02,
                pos.as_vec3() + Vec3::ONE * 0.98,
                vec4(0.0, 0.0, 1.0, 1.0),
            );
        }
    }

    // Math
    pub fn block_length(&self) -> usize {
        self.blocks_per_chunk.element_product() as usize
    }
    pub fn node_length(&self) -> usize {
        self.nodes_per_chunk.element_product() as usize
    }
    pub fn node_size_plus_padding(&self) -> IVec3 {
        self.nodes_per_chunk + 2
    }
    pub fn node_length_plus_padding(&self) -> usize {
        Self::node_size_plus_padding(self).element_product() as usize
    }

    pub fn add_chunk(&mut self, chunk_pos: IVec3) {
        let chunk = ShipDataChunk {
            pos: chunk_pos,
            blocks: vec![BLOCK_INDEX_EMPTY; self.block_length()],
            nodes: vec![Vec::new(); self.node_length()],
            nodes_base: vec![Vec::new(); self.node_length()],
            node_id_bits: vec![0; self.node_length()],
            render_nodes: vec![RenderNode(false); self.node_length_plus_padding()],
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

    pub fn to_world_node_index(&self, chunk_index: usize, node_index: usize) -> usize {
        node_index + (chunk_index << self.node_index_bits)
    }

    pub fn from_world_node_index(&self, node_world_index: usize) -> (usize, usize) {
        (
            node_world_index >> self.node_index_bits,
            node_world_index & self.node_index_mask,
        )
    }

    pub fn pos_from_world_node_index(&self, chunk_index: usize, node_index: usize) -> IVec3 {
        let chunk_pos = self.chunks[chunk_index].pos;
        let node_pos = to_3d_i(node_index as i32, self.nodes_per_chunk);

        chunk_pos + node_pos
    }

    pub fn node_index_to_node_index_plus_padding(&self, node_index: usize) -> usize {
        let node_pos = to_3d_i(node_index as i32, self.nodes_per_chunk);
        to_1d_i(node_pos + IVec3::ONE, self.node_size_plus_padding()) as usize
    }

    pub fn block_world_pos_from_in_chunk_block_index(
        &self,
        block_index: usize,
        chunk_pos: IVec3,
    ) -> IVec3 {
        to_3d_i(block_index as i32, self.blocks_per_chunk) + chunk_pos
    }

    fn get_neighbor_world_node_index(&mut self, pos: IVec3) -> impl Iterator<Item = usize> {
        get_neighbors()
            .map(|offset| {
                let neighbor_pos = pos + offset;
                let chunk_index = self.get_chunk_index_from_node_pos(neighbor_pos);
                let node_index = self.get_node_index(neighbor_pos);
                let node_world_index = self.to_world_node_index(chunk_index, node_index);

                node_world_index
            })
            .into_iter()
    }
}
