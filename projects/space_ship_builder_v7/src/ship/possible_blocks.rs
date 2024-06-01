use crate::rules::block::{BlockIndex, BlockNameIndex};

#[derive(Clone, Debug, Default)]
pub struct PossibleBlocks {
    blocks: Vec<(BlockNameIndex, Vec<BlockIndex>)>,
}

impl PossibleBlocks {
    fn get_index(&mut self, block_name_index: BlockNameIndex) -> usize {
        let res = self
            .blocks
            .binary_search_by(|(test_index, _)| test_index.cmp(&block_name_index));
        if res.is_ok() {
            res.unwrap()
        } else {
            let new_index = res.err().unwrap();
            self.blocks.insert(new_index, (block_name_index, vec![]));
            new_index
        }
    }

    pub fn set_blocks(
        &mut self,
        block_name_index: BlockNameIndex,
        block_indices: &[BlockNameIndex],
    ) {
        let index = self.get_index(block_name_index);

        self.blocks[index].1.clear();
        self.blocks[index].1.extend_from_slice(block_indices);
    }

    pub fn get_blocks(&mut self, block_name_index: BlockNameIndex) -> &[BlockNameIndex] {
        let index = self.get_index(block_name_index);

        self.blocks[index].1.as_slice()
    }
}
