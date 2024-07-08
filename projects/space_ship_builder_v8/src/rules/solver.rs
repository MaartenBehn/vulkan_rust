use crate::rules::hull::HullSolver;
use crate::rules::Prio;
use crate::world::block_object::possible_blocks::PossibleBlocks;
use crate::world::block_object::{BlockObject, ChunkIndex};
use crate::world::data::block::{Block, BlockIndex};
use octa_force::anyhow::{bail, Result};
use octa_force::glam::IVec3;
use std::any::Any;

pub type SolverCacheIndex = usize;

pub trait ToAny: 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> ToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait Solver: ToAny {
    fn block_check_reset(
        &self,
        block_object: &mut BlockObject,
        block_index: BlockIndex,
        chunk_index: ChunkIndex,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex>;

    fn debug_block_check_reset(
        &self,
        _: &mut BlockObject,
        _: BlockIndex,
        _: ChunkIndex,
        _: IVec3,
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
        _: &mut BlockObject,
        _: BlockIndex,
        _: ChunkIndex,
        _: IVec3,
        _: &[PossibleBlocks],
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

    fn to_hull(&self) -> Result<&HullSolver> {
        let a = self.as_any();
        match a.downcast_ref::<HullSolver>() {
            Some(hull_solver) => Ok(hull_solver),
            None => bail!("Not Hull Solver"),
        }
    }
}
