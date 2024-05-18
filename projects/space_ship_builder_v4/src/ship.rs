use crate::generator::Generator;
#[cfg(debug_assertions)]
use crate::math::{get_packed_index, to_3d_i};
use crate::node::{Node, NodeID, PatternIndex, EMPYT_PATTERN_INDEX};
use crate::ship_mesh::RenderNode;
use crate::{
    math::{to_1d, to_1d_i, to_3d},
    node::{BlockIndex, BLOCK_INDEX_EMPTY},
    ship_mesh::ShipMesh,
};
use octa_force::{anyhow::*, glam::*, log};
use std::collections::VecDeque;

pub type ChunkIndex = usize;
pub type WaveIndex = usize;

pub const CHUNK_SIZE: i32 = 16;

pub struct Ship {
    pub chunks: Vec<ShipChunk>,
    pub block_size: i32,
}

pub struct ShipChunk {
    pub pos: IVec3,
    pub blocks: Vec<BlockIndex>,
    pub node_id_bits: Vec<u32>,
    pub node_voxels: Vec<RenderNode>,
}

impl Ship {
    pub fn new(block_size: i32, generator: &mut Generator) -> Result<Ship> {
        let mut ship = Ship {
            chunks: Vec::new(),
            block_size,
        };
        ship.add_chunk(IVec3::ZERO);

        ship.place_block(ivec3(0, 0, 0), 1, generator)?;
        //ship.fill_all(0, node_controller)?;

        Ok(ship)
    }

    pub fn add_chunk(&mut self, pos: IVec3) {
        let chunk = ShipChunk {
            pos,
            blocks: vec![BLOCK_INDEX_EMPTY; self.block_length()],
            node_id_bits: vec![0; self.node_length()],
            node_voxels: vec![RenderNode(false); self.node_length_plus_padding()],
        };

        self.chunks.push(chunk)
    }

    pub fn has_chunk(&self, chunk_pos: IVec3) -> bool {
        chunk_pos == IVec3::ZERO
    }

    pub fn get_chunk_index(&self, chunk_pos: IVec3) -> Result<usize> {
        if !self.has_chunk(chunk_pos) {
            bail!("Chunk not found!");
        }

        Ok(0)
    }

    pub fn place_block(
        &mut self,
        block_pos: IVec3,
        block_index: BlockIndex,
        generator: &mut Generator,
    ) -> Result<()> {
        let chunk_pos = self.get_chunk_pos_of_block_pos(block_pos);
        let in_chunk_pos = self.get_in_chunk_pos_of_node_pos(block_pos);
        let chunk_index = self.get_chunk_index(chunk_pos)?;
        let in_chunk_index = to_1d_i(in_chunk_pos, IVec3::ONE * self.block_size) as usize;

        let chunk = &mut self.chunks[chunk_index];

        if chunk.blocks[in_chunk_index] == block_index {
            return Ok(());
        }

        log::info!("Place: {block_pos:?}");
        chunk.blocks[in_chunk_index] = block_index;

        let node_pos = Self::get_node_pos_of_block_pos(block_pos);
        for offset in Self::get_nodes_offsets_of_block() {
            let pos = node_pos + offset;
            let chunk_pos = self.get_chunk_pos_of_node_pos(pos);
            let chunk_index_result = self.get_chunk_index(chunk_pos);
            if chunk_index_result.is_err() {
                continue;
            }

            let node_id = generator.generate_node(self, pos);

            let node_index = to_1d_i(pos, IVec3::ONE * self.node_size()) as usize;
            let chunk_index = chunk_index_result.unwrap();
            let node_id_bits: u32 = node_id.into();
            let node_index_plus_padding =
                to_1d_i(pos + IVec3::ONE, IVec3::ONE * self.node_size_plus_padding()) as usize;

            self.chunks[chunk_index].node_id_bits[node_index] = node_id_bits;
            self.chunks[chunk_index].node_voxels[node_index_plus_padding] =
                RenderNode(node_id_bits != 0);
        }

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
        (self.block_size * self.block_size * self.block_size) as usize
    }
    pub fn node_size(&self) -> i32 {
        self.block_size * 2
    }
    pub fn node_length(&self) -> usize {
        (Self::node_size(self) * Self::node_size(self) * Self::node_size(self)) as usize
    }
    pub fn node_size_plus_padding(&self) -> i32 {
        Self::node_size(self) + 2
    }
    pub fn node_length_plus_padding(&self) -> usize {
        (Self::node_size_plus_padding(self)
            * Self::node_size_plus_padding(self)
            * Self::node_size_plus_padding(self)) as usize
    }

    pub fn get_chunk_pos_of_block_pos(&self, pos: IVec3) -> IVec3 {
        (pos / self.block_size) - ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
    }

    pub fn get_in_chunk_pos_of_block_pos(&self, pos: IVec3) -> IVec3 {
        pos % self.block_size
    }

    pub fn get_chunk_pos_of_node_pos(&self, pos: IVec3) -> IVec3 {
        (pos / self.node_size()) - ivec3((pos.x < 0) as i32, (pos.y < 0) as i32, (pos.z < 0) as i32)
    }

    pub fn get_in_chunk_pos_of_node_pos(&self, pos: IVec3) -> IVec3 {
        pos % self.node_size()
    }

    pub fn pos_in_bounds(pos: IVec3, size: IVec3) -> bool {
        pos.cmpge(IVec3::ZERO).all() && pos.cmplt(size).all()
    }

    pub fn get_node_pos_of_block_pos(pos: IVec3) -> IVec3 {
        pos * 2
    }

    pub fn get_block_pos_of_node_pos(pos: IVec3) -> IVec3 {
        pos / 2
    }

    pub fn get_config(pos: IVec3) -> usize {
        let c = (pos % 2).abs();
        (c.x + (c.y << 1) + (c.z << 2)) as usize
    }
}
