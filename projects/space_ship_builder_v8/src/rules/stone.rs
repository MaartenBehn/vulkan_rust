use crate::math::get_neighbors_without_zero;
use crate::rules::basic_blocks::BasicBlocks;
use crate::rules::empty::EMPTY_BLOCK_NAME_INDEX;
use crate::rules::req_tree::BroadReqTree;
use crate::rules::solver::{Solver, SolverCacheIndex};
use crate::rules::{Prio, Rules};
use crate::world::block_object::possible_blocks::PossibleBlocks;
use crate::world::block_object::BlockObject;
use crate::world::data::block::Block;
use crate::world::data::node::NodeID;
use crate::world::data::voxel_loader::VoxelLoader;
use log::{debug, info};
use octa_force::anyhow::bail;
use octa_force::puffin_egui::puffin;
use octa_force::{
    anyhow::Result,
    glam::{IVec3, Mat4},
};

const STONE_BLOCK_NAME: &str = "Stone";
const STONE_BASE_NAME_PART: &str = "Stone-Base";

pub struct StoneSolver {
    pub block_name_index: usize,
    pub basic_blocks: BasicBlocks,
}

impl Rules {
    pub fn make_stone(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        info!("Making Stone");

        let stone_block_name_index = self.block_names.len();
        self.block_names.push(STONE_BLOCK_NAME.to_owned());

        let basic_blocks = BasicBlocks::new(self, voxel_loader, STONE_BASE_NAME_PART, 1)?;
        let stone_solver = StoneSolver {
            block_name_index: stone_block_name_index,
            basic_blocks,
        };

        self.solvers.push(Box::new(stone_solver));

        info!("Making Stone Done");
        Ok(())
    }
}

impl Solver for StoneSolver {
    fn block_check_reset(
        &self,
        block_object: &mut BlockObject,
        _: usize,
        _: usize,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex> {
        #[cfg(debug_assertions)]
        puffin::profile_function!();

        let mut cache = vec![];
        cache.append(&mut self.basic_blocks.get_possible_blocks(
            block_object,
            world_block_pos,
            self.block_name_index,
        ));

        cache
    }

    fn block_check(
        &self,
        _: &mut BlockObject,
        _: usize,
        _: usize,
        _: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex> {
        cache
    }

    fn get_block(
        &self,
        _: &mut BlockObject,
        _: usize,
        _: usize,
        _: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> (Block, Prio, usize) {
        let mut best_block = Block::from_single_node_id(NodeID::empty());
        let mut best_prio = Prio::Empty;
        let mut best_index = 0;

        for index in cache {
            let (_, block, prio) = &self.basic_blocks.get_block(index);
            if best_prio < *prio {
                best_block = *block;
                best_prio = *prio;
                best_index = index;
            }
        }

        (best_block, best_prio, best_index)
    }

    fn get_block_from_cache_index(&self, index: usize) -> Block {
        self.basic_blocks.get_block(index).1
    }
}
