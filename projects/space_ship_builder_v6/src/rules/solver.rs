use crate::rules::hull::HullSolver;
use crate::ship::Ship;
use octa_force::glam::IVec3;

pub trait Solver {
    fn block_check(&mut self, ship: &Ship, node_pos: IVec3, node_index: usize, chunk_index: usize);
    fn node_check(&mut self, ship: &Ship, node_pos: IVec3, node_index: usize, chunk_index: usize);
}
