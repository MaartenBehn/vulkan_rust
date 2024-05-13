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
use octa_force::{anyhow::*, glam::*, log};

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

    pub to_propergate: IndexQueue,
}

pub struct ShipChunk {
    pub pos: IVec3,
    pub blocks: Vec<BlockIndex>,
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

        let mut ship = Ship {
            chunks: Vec::new(),

            blocks_per_chunk,
            nodes_per_chunk,
            chunk_pos_mask,
            chunk_voxel_size,
            in_chunk_pos_mask,
            node_index_bits,

            to_propergate: IndexQueue::default(),
        };
        ship.add_chunk(IVec3::ZERO);

        ship.place_block(ivec3(0, 0, 0), 1, rules)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    pub fn add_chunk(&mut self, chunk_pos: IVec3) {
        let chunk = ShipChunk {
            pos: chunk_pos,
            blocks: vec![BLOCK_INDEX_EMPTY; self.block_length()],
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

                let node_world_index = self.get_world_node_index(chunk_index.unwrap(), node_index);
                self.to_propergate.push_back(node_world_index);
            }

            Ok(())
        };

        push_propergate(old_block_index, pos)?;
        push_propergate(block_index, pos)?;

        Ok(())
    }

    pub fn get_nodes_offsets_of_block() -> [IVec3; 8] {
        [
            ivec3(0, 0, 0),
            ivec3(1, 0, 0),
            ivec3(0, 1, 0),
            ivec3(1, 1, 0),
            ivec3(0, 0, 1),
            ivec3(1, 0, 1),
            ivec3(0, 1, 1),
            ivec3(1, 1, 1),
        ]
    }

    pub fn tick(&mut self, actions_per_tick: usize) -> Result<(bool, Vec<ChunkIndex>)> {
        Ok((true, vec![0]))
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

    pub fn get_world_node_index(&self, chunk_index: usize, node_index: usize) -> usize {
        node_index + (chunk_index << self.node_index_bits)
    }
}
