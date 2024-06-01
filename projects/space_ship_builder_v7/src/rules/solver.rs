use crate::rules::block::BlockNameIndex;
use crate::rules::hull::HullSolver;
use crate::ship::data::ShipData;
use octa_force::glam::IVec3;
use std::any::Any;

pub trait ToAny: 'static {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> ToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait Solver: ToAny {
    fn block_check(
        &self,
        ship: &mut ShipData,
        chunk_index: usize,
        block_index: usize,
        world_block_pos: IVec3,
    ) -> Vec<BlockNameIndex>;

    fn to_hull(&self) -> &HullSolver {
        let a = self.as_any();
        match a.downcast_ref::<HullSolver>() {
            Some(hull_solver) => hull_solver,
            None => panic!(),
        }
    }
}
