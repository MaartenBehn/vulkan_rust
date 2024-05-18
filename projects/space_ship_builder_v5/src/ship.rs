use crate::math::{get_packed_index, to_3d_i};
use crate::node::{Node, NodeID, PatternIndex, EMPYT_PATTERN_INDEX};
use crate::rules::Rules;
use crate::ship_mesh::RenderNode;
use crate::{
    math::{to_1d, to_1d_i, to_3d},
    node::{BlockIndex, BLOCK_INDEX_EMPTY},
    ship_mesh::ShipMesh,
};
use index_queue::IndexQueue;
use log::{debug, info};
use octa_force::{anyhow::*, glam::*, log};

#[cfg(debug_assertions)]
use crate::debug::DebugController;

pub type ChunkIndex = usize;
pub type WaveIndex = usize;

pub const CHUNK_SIZE: i32 = 32;

pub struct Ship {
    pub chunks: Vec<ShipChunk>,

    pub blocks_per_chunk: IVec3,
    pub nodes_per_chunk: IVec3,
    pub chunk_pos_mask: IVec3,
    pub chunk_voxel_size: IVec3,
    pub in_chunk_pos_mask: IVec3,
    pub node_index_bits: usize,
    pub node_index_mask: usize,

    pub to_propergate: IndexQueue,
    pub to_collapse: IndexQueue,
}

pub struct ShipChunk {
    pub pos: IVec3,
    pub blocks: Vec<BlockIndex>,
    pub nodes: Vec<Option<Vec<NodeID>>>,
    pub node_id_bits: Vec<u32>,
    pub render_nodes: Vec<RenderNode>,
}

impl Ship {
    pub fn new(node_size: i32, rules: &Rules) -> Result<Ship> {
        let blocks_per_chunk = IVec3::ONE * node_size / 2;
        let nodes_per_chunk = IVec3::ONE * node_size;
        let chunk_pos_mask = IVec3::ONE * !(node_size - 1);
        let chunk_voxel_size = IVec3::ONE * node_size;
        let in_chunk_pos_mask = IVec3::ONE * (node_size - 1);
        let node_index_bits = (nodes_per_chunk.element_product().trailing_zeros() + 1) as usize;
        let node_index_mask = (nodes_per_chunk.element_product() - 1) as usize;

        let mut ship = Ship {
            chunks: Vec::new(),

            blocks_per_chunk,
            nodes_per_chunk,
            chunk_pos_mask,
            chunk_voxel_size,
            in_chunk_pos_mask,
            node_index_bits,
            node_index_mask,

            to_propergate: IndexQueue::default(),
            to_collapse: IndexQueue::default(),
        };
        ship.add_chunk(IVec3::ZERO);

        //ship.place_block(ivec3(0, 0, 0), 1, rules)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    pub fn place_block(
        &mut self,
        block_pos: IVec3,
        block_index: BlockIndex,
        rules: &Rules,
    ) -> Result<()> {
        let pos = self.get_node_pos_from_block_pos(block_pos);

        let chunk_index = self.get_chunk_index(pos)?;
        let in_chunk_block_index = self.get_block_index(pos);

        let chunk = &mut self.chunks[chunk_index];

        let old_block_index = chunk.blocks[in_chunk_block_index];
        if old_block_index == block_index {
            return Ok(());
        }

        log::info!("Place: {block_pos:?}");
        chunk.blocks[in_chunk_block_index] = block_index;

        let mut push_propergate = |block_index: BlockIndex, pos: IVec3| -> Result<()> {
            if block_index == BLOCK_INDEX_EMPTY {
                return Ok(());
            }

            for offset in rules.affected_by_block[block_index].iter() {
                let affected_pos = pos + *offset;

                let chunk_index = self.get_chunk_index(affected_pos);
                if chunk_index.is_err() {
                    continue;
                }

                let node_index = self.get_node_index(affected_pos);

                let node_world_index = self.to_world_node_index(chunk_index.unwrap(), node_index);
                self.to_propergate.push_back(node_world_index);
            }

            Ok(())
        };

        push_propergate(old_block_index, pos)?;
        push_propergate(block_index, pos)?;

        Ok(())
    }

    pub fn tick(
        &mut self,
        actions_per_tick: usize,
        rules: &Rules,
    ) -> Result<(bool, Vec<ChunkIndex>)> {
        let mut changed_chunks = Vec::new();
        for _ in 0..actions_per_tick {
            if !self.to_propergate.is_empty() {
                self.propergate(rules)?;
            } else if !self.to_collapse.is_empty() {
                self.collapse()?;
                changed_chunks = vec![0];
            } else {
                return Ok((false, changed_chunks));
            }
        }

        info!("Tick: {actions_per_tick}");

        Ok((true, changed_chunks))
    }

    fn propergate(&mut self, rules: &Rules) -> Result<()> {
        let node_world_index = self.to_propergate.pop_front().unwrap();
        let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
        let pos = self.pos_from_world_node_index(chunk_index, node_index);

        //debug!("Propergate: {node_world_index}");

        let mut new_possible_node_ids = Vec::new();

        // Go over all node ids and their block requirements
        for (node_id, reqs) in rules
            .map_rules_index_to_node_id
            .iter()
            .zip(rules.block_rules.iter())
        {
            for (req, prio) in reqs {
                let mut accepted = true;
                let mut check_performed = false;
                // Go over all offsets of the requirement
                for (offset, id) in req.iter() {
                    let test_pos = pos + *offset;

                    // If the offset does not aling with the node just ignore it.
                    if (test_pos % 2) != IVec3::ZERO {
                        continue;
                    }

                    let test_chunk_index = self.get_chunk_index(test_pos);
                    let test_block_index = self.get_block_index(test_pos);

                    let mut found = false;
                    if test_chunk_index.is_err() {
                        // If the block is in chunk that does not exist it is always Air.

                        found = *id == BLOCK_INDEX_EMPTY
                    } else {
                        // If the chuck exists.

                        let index = self.chunks[test_chunk_index.unwrap()].blocks[test_block_index]
                            .to_owned();

                        // Check if the Block at the pos is in the allowed block ids.
                        found = *id == index;
                    };

                    accepted &= found;
                    check_performed = true;
                }

                if accepted && check_performed {
                    new_possible_node_ids.push(node_id.to_owned());
                }
            }
        }

        let old_possible_node_ids = self.chunks[chunk_index].nodes[node_index]
            .take()
            .unwrap_or(Vec::new());

        let mut push_propergate = |node_id: NodeID| -> Result<()> {
            for offset in rules.affected_by_node[&node_id].iter() {
                let affected_pos = pos + *offset;

                let chunk_index = self.get_chunk_index(affected_pos);
                if chunk_index.is_err() {
                    continue;
                }

                let node_index = self.get_node_index(affected_pos);

                let node_world_index = self.to_world_node_index(chunk_index.unwrap(), node_index);
                self.to_propergate.push_back(node_world_index);
            }

            Ok(())
        };

        if old_possible_node_ids != new_possible_node_ids {
            for node_id in old_possible_node_ids.iter() {
                push_propergate(node_id.to_owned())?;
            }

            for node_id in new_possible_node_ids.iter() {
                push_propergate(node_id.to_owned())?;
            }

            self.to_collapse.push_back(node_world_index);
        }
        self.chunks[chunk_index].nodes[node_index] = Some(new_possible_node_ids);

        Ok(())
    }

    fn collapse(&mut self) -> Result<()> {
        let node_world_index = self.to_collapse.pop_front().unwrap();
        let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
        let node_index_plus_padding = self.node_index_to_node_index_plus_padding(node_index);

        //debug!("Collapse: {node_world_index}");

        let possible_node_ids = self.chunks[chunk_index].nodes[node_index]
            .take()
            .unwrap_or(Vec::new());

        let node_id = possible_node_ids
            .first()
            .unwrap_or(&NodeID::none())
            .to_owned();
        self.chunks[chunk_index].node_id_bits[node_index] = node_id.into();
        self.chunks[chunk_index].render_nodes[node_index_plus_padding] =
            RenderNode(!node_id.is_none());

        self.chunks[chunk_index].nodes[node_index] = Some(possible_node_ids);
        Ok(())
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

        let mut to_propergate = self.to_propergate.to_owned();

        while !to_propergate.is_empty() {
            let node_world_index = to_propergate.pop_front().unwrap();
            let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
            let pos = self.pos_from_world_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3(),
                pos.as_vec3() + Vec3::ONE,
                vec4(0.0, 0.0, 1.0, 1.0),
            );
        }

        let mut to_collapse = self.to_collapse.to_owned();

        while !to_collapse.is_empty() {
            let node_world_index = to_collapse.pop_front().unwrap();
            let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
            let pos = self.pos_from_world_node_index(chunk_index, node_index);

            debug_controller.add_cube(
                pos.as_vec3(),
                pos.as_vec3() + Vec3::ONE,
                vec4(0.0, 1.0, 0.0, 1.0),
            );
        }
    }

    pub fn on_rules_changed(&mut self) -> Result<()> {
        for chunk_index in 0..self.chunks.len() {
            for node_index in 0..self.node_length() {
                let node_world_index = self.to_world_node_index(chunk_index, node_index);
                self.to_propergate.push_back(node_world_index);
            }
        }

        std::prelude::rust_2015::Ok(())
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
        let chunk = ShipChunk {
            pos: chunk_pos,
            blocks: vec![BLOCK_INDEX_EMPTY; self.block_length()],
            nodes: vec![None; self.node_length()],
            node_id_bits: vec![0; self.node_length()],
            render_nodes: vec![RenderNode(false); self.node_length_plus_padding()],
        };

        self.chunks.push(chunk)
    }

    pub fn has_chunk(&self, chunk_pos: IVec3) -> bool {
        chunk_pos == IVec3::ZERO
    }

    pub fn get_chunk_index(&self, pos: IVec3) -> Result<usize> {
        let chunk_pos = self.get_chunk_pos(pos);

        if !self.has_chunk(chunk_pos) {
            bail!("Chunk not found!");
        }

        Ok(0)
    }

    pub fn get_node_pos_from_block_pos(&self, pos: IVec3) -> IVec3 {
        pos * 2
    }

    pub fn get_chunk_pos(&self, pos: IVec3) -> IVec3 {
        (pos & self.chunk_pos_mask)
            - self.chunk_voxel_size
                * ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
    }

    pub fn get_in_chunk_pos(&self, pos: IVec3) -> IVec3 {
        pos & self.in_chunk_pos_mask
    }

    pub fn get_block_index(&self, pos: IVec3) -> usize {
        let in_chunk_index = self.get_in_chunk_pos(pos);
        to_1d_i(in_chunk_index / 2, self.blocks_per_chunk) as usize
    }

    pub fn get_node_index(&self, pos: IVec3) -> usize {
        let in_chunk_index = self.get_in_chunk_pos(pos);
        to_1d_i(in_chunk_index, self.nodes_per_chunk) as usize
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
}
