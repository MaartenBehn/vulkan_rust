use crate::node::{BlockIndex, NodeIndex};
use crate::ship::data::ChunkIndex;

#[derive(Clone, Debug)]
pub struct NodeOrderController {
    pub node_index_shift_bits: usize,
    pub chunk_index_shift_bits: usize,
    pub chunk_index_shift_bits_with_block: usize,

    pub block_mask: usize,
    pub node_mask: usize,
}

impl NodeOrderController {
    pub fn new(num_blocks: usize, nodes_length: usize) -> Self {
        let block_mask_bits = num_blocks.ilog2() as usize;
        let node_mask_bits = nodes_length.trailing_zeros() as usize;
        let block_mask = ((1 << block_mask_bits) - 1) as usize;
        let node_mask = nodes_length - 1;

        Self {
            node_index_shift_bits: block_mask_bits,
            chunk_index_shift_bits: node_mask_bits,
            chunk_index_shift_bits_with_block: block_mask_bits + node_mask_bits,
            block_mask,
            node_mask,
        }
    }

    pub fn pack_order_with_block(
        &self,
        block_index: BlockIndex,
        node_index: NodeIndex,
        chunk_index: ChunkIndex,
    ) -> usize {
        block_index
            + (node_index << self.node_index_shift_bits)
            + (chunk_index << self.chunk_index_shift_bits_with_block)
    }

    pub fn unpack_order_with_block(&self, order: usize) -> (usize, usize, usize) {
        let block_index = order & self.block_mask;
        let node_index = (order >> self.node_index_shift_bits) & self.node_mask;
        let chunk_index = order >> self.chunk_index_shift_bits_with_block;

        (block_index, chunk_index, node_index)
    }

    pub fn pack_order(&self, node_index: NodeIndex, chunk_index: ChunkIndex) -> usize {
        node_index + (chunk_index << self.chunk_index_shift_bits)
    }

    pub fn unpack_order(&self, order: usize) -> (usize, usize) {
        let node_index = order & self.node_mask;
        let chunk_index = order >> self.chunk_index_shift_bits;

        (chunk_index, node_index)
    }
}
