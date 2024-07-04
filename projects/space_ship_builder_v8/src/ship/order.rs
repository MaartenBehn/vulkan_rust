use crate::rules::block::{BlockIndex, BlockNameIndex};
use crate::ship::data::ChunkIndex;

#[derive(Clone, Debug)]
pub struct NodeOrderController {
    pub block_index_shift_bits: usize,
    pub chunk_index_shift_bits: usize,
    pub chunk_index_shift_bits_with_block: usize,

    pub block_name_mask: usize,
    pub block_mask: usize,
}

impl NodeOrderController {
    pub fn new(num_block_names: usize, block_length: usize) -> Self {
        let block_name_mask_bits = (num_block_names.ilog2() + 1) as usize;
        let block_mask_bits = block_length.trailing_zeros() as usize;
        let block_name_mask = ((1 << block_name_mask_bits) - 1) as usize;
        let block_mask = block_length - 1;

        Self {
            block_index_shift_bits: block_name_mask_bits,
            chunk_index_shift_bits: block_mask_bits,
            chunk_index_shift_bits_with_block: block_name_mask_bits + block_mask_bits,
            block_name_mask,
            block_mask,
        }
    }

    pub fn pack_propergate_order(
        &self,
        block_name_index: BlockNameIndex,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
    ) -> usize {
        block_name_index
            + (block_index << self.block_index_shift_bits)
            + (chunk_index << self.chunk_index_shift_bits_with_block)
    }

    pub fn pack_collapse_order(&self, block_index: BlockIndex, chunk_index: ChunkIndex) -> usize {
        block_index + (chunk_index << self.chunk_index_shift_bits)
    }

    pub fn unpack_propergate_order(&self, order: usize) -> (usize, usize, usize) {
        let block_name_index = order & self.block_name_mask;
        let block_index = (order >> self.block_index_shift_bits) & self.block_mask;
        let chunk_index = order >> self.chunk_index_shift_bits_with_block;

        (block_name_index, block_index, chunk_index)
    }

    pub fn unpack_collapse_order(&self, order: usize) -> (usize, usize) {
        let block_index = order & self.block_mask;
        let chunk_index = order >> self.chunk_index_shift_bits;

        (block_index, chunk_index)
    }
}
