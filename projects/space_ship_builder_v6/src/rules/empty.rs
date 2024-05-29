use octa_force::glam::IVec3;
use crate::rules::block_preview::BlockPreview;
use crate::rules::Rules;
use crate::rules::solver::{push_in_block_affected_nodes, Solver};
use crate::ship::data::ShipData;

pub struct EmptySolver {}


impl Rules {
    pub fn make_empty(&mut self) {
        self.block_names.push("Empty".to_owned());
        self.block_previews.push(BlockPreview::default());
        self.solvers.push(Box::new(EmptySolver{}));
    }
}

impl Solver for EmptySolver {
    fn push_block_affected_nodes(&mut self, ship: &mut ShipData, block_pos: IVec3) {
        push_in_block_affected_nodes(ship, block_pos);
    }

    fn block_check(&mut self, ship: &mut ShipData, node_pos: IVec3, node_index: usize, chunk_index: usize) {

    }

    fn node_check(&mut self, ship: &mut ShipData, node_pos: IVec3, node_index: usize, chunk_index: usize) {

    }
}