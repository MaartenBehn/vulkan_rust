use crate::rules::block::Block;
use crate::rules::hull::HullSolver;
use crate::rules::Prio;
use crate::ship::data::ShipData;
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
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex>;

    fn block_check(
        &self,
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex>;

    fn get_block(
        &self,
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> (Block, Prio);

    fn to_hull(&self) -> &HullSolver {
        let a = self.as_any();
        match a.downcast_ref::<HullSolver>() {
            Some(hull_solver) => hull_solver,
            None => panic!(),
        }
    }
}
