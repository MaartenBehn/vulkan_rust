use crate::node::Node;
use crate::rules::block::{Block, BlockNameIndex};
use crate::rules::solver::Solver;
use crate::rules::Rules;
use crate::ship::data::ShipData;
use octa_force::glam::IVec3;

pub struct EmptySolver {}

impl Rules {
    pub fn make_empty(&mut self) {
        self.block_names.push("Empty".to_owned());
        self.solvers.push(Box::new(EmptySolver {}));
        self.nodes.push(Node::default());
    }
}

impl Solver for EmptySolver {
    fn block_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<BlockNameIndex> {
        vec![]
    }
}
