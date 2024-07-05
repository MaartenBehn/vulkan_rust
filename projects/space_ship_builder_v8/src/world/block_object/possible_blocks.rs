use crate::rules::solver::SolverCacheIndex;
use crate::world::data::block::BlockNameIndex;
use octa_force::puffin_egui::puffin;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PossibleBlocks {
    blocks: Vec<(BlockNameIndex, Vec<SolverCacheIndex>)>,
}

impl PossibleBlocks {
    fn get_index(&mut self, block_name_index: BlockNameIndex) -> usize {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

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
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let index = self.get_index(block_name_index);

        self.blocks[index].1.clear();
        self.blocks[index].1.extend_from_slice(cache);
    }

    pub fn set_all_caches_with_one(
        &mut self,
        block_name_index: BlockNameIndex,
        index: SolverCacheIndex,
    ) {
        self.blocks = vec![(block_name_index, vec![index])];
    }

    pub fn get_cache(&mut self, block_name_index: BlockNameIndex) -> &[SolverCacheIndex] {
        let index = self.get_index(block_name_index);

        self.blocks[index].1.as_slice()
    }

    pub fn get_all_caches(&mut self) -> Vec<(BlockNameIndex, Vec<SolverCacheIndex>)> {
        self.blocks.to_owned()
    }

    pub fn get_num_caches(&self) -> usize {
        let mut num = 0;

        for (_, cache) in self.blocks.iter() {
            num += cache.len();
        }

        num
    }
}
