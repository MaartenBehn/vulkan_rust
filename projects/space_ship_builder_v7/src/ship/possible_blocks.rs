use crate::rules::block::BlockNameIndex;
use crate::rules::solver::SolverCacheIndex;

#[derive(Clone, Debug, Default)]
pub struct PossibleBlocks {
    blocks: Vec<(BlockNameIndex, Vec<SolverCacheIndex>)>,
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

    pub fn set_cache(&mut self, block_name_index: BlockNameIndex, cache: &[SolverCacheIndex]) {
        let index = self.get_index(block_name_index);

        self.blocks[index].1.clear();
        self.blocks[index].1.extend_from_slice(cache);
    }

    pub fn get_cache(&mut self, block_name_index: BlockNameIndex) -> &[SolverCacheIndex] {
        let index = self.get_index(block_name_index);

        self.blocks[index].1.as_slice()
    }
}
