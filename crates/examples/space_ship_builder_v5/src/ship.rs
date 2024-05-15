use crate::debug::DebugController;
#[cfg(debug_assertions)]
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
use log::debug;
use octa_force::{anyhow::*, glam::*, log};
use std::iter;

pub type ChunkIndex = usize;
pub type WaveIndex = usize;

pub const CHUNK_SIZE: i32 = 16;
pub const VOXELS_PER_NODE: i32 = 4;
pub const VOXELS_PER_BLOCK: i32 = 8;

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
}

pub struct ShipChunk {
    pub pos: IVec3,
    pub blocks: Vec<BlockIndex>,
    pub nodes: Vec<Option<Vec<NodeID>>>,
    pub node_id_bits: Vec<u32>,
    pub node_voxels: Vec<RenderNode>,
}

impl Ship {
    pub fn new(block_size: i32, rules: &Rules) -> Result<Ship> {
        let blocks_per_chunk = IVec3::ONE * block_size;
        let nodes_per_chunk = IVec3::ONE * block_size * 2;
        let chunk_pos_mask = IVec3::ONE * !((block_size * VOXELS_PER_BLOCK) - 1);
        let chunk_voxel_size = IVec3::ONE * (block_size * VOXELS_PER_BLOCK);
        let in_chunk_pos_mask = IVec3::ONE * ((block_size * VOXELS_PER_BLOCK) - 1);
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
        let pos = self.get_voxel_pos_from_block_pos(block_pos);

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
            for offset in rules.affected_by_block[old_block_index].iter() {
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
        for _ in 0..actions_per_tick {
            if self.to_propergate.is_empty() {
                return Ok((false, vec![0]));
            }

            let node_world_index = self.to_propergate.pop_front().unwrap();
            let (chunk_index, node_index) = self.from_world_node_index(node_world_index);
            let pos = self.pos_from_World_node_index(chunk_index, node_index);

            debug!("Node: {node_index}");

            let mut new_possible_node_ids = Vec::new();
            for (node_id, reqs) in rules
                .map_rules_index_to_node_id
                .iter()
                .zip(rules.node_rules.iter())
            {
                let mut accepted = true;
                for (offset, ids) in reqs.iter() {
                    let test_pos = pos + *offset;

                    let test_chunk_index = self.get_chunk_index(test_pos);
                    let test_node_index = self.get_node_index(test_pos);

                    let mut found = false;
                    if test_chunk_index.is_err() {
                        for id in ids.iter() {
                            if id.is_none() {
                                found = true;
                                break;
                            }
                        }
                    } else {
                        let test_ids = self.chunks[test_chunk_index.unwrap()].nodes
                            [test_node_index]
                            .to_owned();

                        if test_ids.is_none() {
                            found = true;
                        } else {
                            for test_id in test_ids.unwrap().iter() {
                                found = ids.contains(&test_id);
                            }
                        }
                    };

                    accepted &= found
                }

                if accepted {
                    new_possible_node_ids.push(node_id.to_owned());
                }
            }

            let possible_node_ids = self.chunks[chunk_index].nodes[node_index].take();

            let mut push_propergate = |node_id: NodeID| -> Result<()> {
                for offset in rules.affected_by_node[&node_id].iter() {
                    let affected_pos = pos + *offset;

                    let chunk_index = self.get_chunk_index(affected_pos);
                    if chunk_index.is_err() {
                        continue;
                    }

                    let node_index = self.get_node_index(affected_pos);

                    let node_world_index =
                        self.to_world_node_index(chunk_index.unwrap(), node_index);
                    self.to_propergate.push_back(node_world_index);
                }

                Ok(())
            };

            if possible_node_ids.is_none() {
                for node_id in new_possible_node_ids.iter() {
                    push_propergate(node_id.to_owned())?;
                }
            } else {
                let old_possible_node_ids = possible_node_ids.unwrap();
                if old_possible_node_ids.len() != new_possible_node_ids.len() {
                    for node_id in old_possible_node_ids.iter() {
                        push_propergate(node_id.to_owned())?;
                    }

                    for node_id in new_possible_node_ids.iter() {
                        push_propergate(node_id.to_owned())?;
                    }
                }
            }
            self.chunks[chunk_index].nodes[node_index] = Some(new_possible_node_ids);
        }

        debug!("Tick: {actions_per_tick}");

        Ok((true, vec![0]))
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
    }

    // Math
    pub fn block_length(&self) -> usize {
        self.blocks_per_chunk.element_product() as usize
    }
    pub fn node_size(&self) -> IVec3 {
        self.blocks_per_chunk * 2
    }
    pub fn node_length(&self) -> usize {
        Self::node_size(self).element_product() as usize
    }
    pub fn node_size_plus_padding(&self) -> IVec3 {
        Self::node_size(self) + 2
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
            node_voxels: vec![RenderNode(false); self.node_length_plus_padding()],
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

    pub fn get_voxel_pos_from_block_pos(&self, pos: IVec3) -> IVec3 {
        pos * VOXELS_PER_BLOCK
    }

    pub fn get_chunk_pos(&self, pos: IVec3) -> IVec3 {
        pos & self.chunk_pos_mask
            - self.chunk_voxel_size
                * ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
    }

    pub fn get_in_chunk_pos(&self, pos: IVec3) -> IVec3 {
        pos & self.in_chunk_pos_mask
    }

    pub fn get_block_index(&self, pos: IVec3) -> usize {
        let in_chunk_index = self.get_in_chunk_pos(pos);
        to_1d_i(in_chunk_index / VOXELS_PER_BLOCK, self.blocks_per_chunk) as usize
    }

    pub fn get_node_index(&self, pos: IVec3) -> usize {
        let in_chunk_index = self.get_in_chunk_pos(pos);
        to_1d_i(in_chunk_index / VOXELS_PER_NODE, self.nodes_per_chunk) as usize
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

    pub fn pos_from_World_node_index(&self, chunk_index: usize, node_index: usize) -> IVec3 {
        let chunk_pos = self.chunks[chunk_index].pos;
        let node_pos = to_3d_i(node_index as i32, VOXELS_PER_NODE * self.nodes_per_chunk);

        chunk_pos + node_pos
    }
}
