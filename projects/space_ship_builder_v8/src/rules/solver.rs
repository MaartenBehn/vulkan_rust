use crate::rules::empty::EmptySolver;
use crate::rules::hull::HullSolver;
use crate::rules::stone::StoneSolver;
use crate::rules::Prio;
use crate::world::block_object::possible_blocks::PossibleBlocks;
use crate::world::block_object::{BlockObject, ChunkIndex};
use crate::world::data::block::{Block, BlockIndex};
use enum_as_inner::EnumAsInner;
use octa_force::glam::IVec3;

pub type SolverCacheIndex = usize;

#[enum_delegate::implement(SolverFunctions)]
#[derive(EnumAsInner)]
pub enum Solver {
    Empty(EmptySolver),
    Hull(HullSolver),
    Stone(StoneSolver),
}

#[enum_delegate::register]
pub trait SolverFunctions {
    fn block_check_reset(
        &self,
        block_object: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex>;

    fn debug_block_check_reset(
        &self,
        block_object: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        world_block_pos: IVec3,
    ) -> Vec<(SolverCacheIndex, Vec<(IVec3, bool)>)> {
        vec![]
    }

    fn block_check(
        &self,
        ship: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        world_block_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex>;

    fn debug_block_check(
        &self,
        block_object: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        world_block_pos: IVec3,
        cache: &[PossibleBlocks],
    ) -> Vec<(SolverCacheIndex, Vec<(IVec3, bool)>)> {
        vec![]
    }

    fn get_block(
        &self,
        ship: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        world_block_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> (Block, Prio, usize);

    fn get_block_from_cache_index(&self, index: usize) -> Block;
}
