use crate::node::{Node, NodeID};
use crate::rules::block::Block;
use crate::rules::solver::{Solver, SolverCacheIndex};
use crate::rules::Prio::BASE;
use crate::rules::{Prio, Rules};
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
    fn block_check_reset(
        &self,
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
    ) -> Vec<SolverCacheIndex> {
        vec![]
    }

    fn block_check(
        &self,
        ship: &mut ShipData,
        chunk_index: usize,
        node_index: usize,
        world_node_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex> {
        vec![]
    }

    fn get_block(
        &self,
        ship: &mut ShipData,
        block_index: usize,
        chunk_index: usize,
        world_block_pos: IVec3,
        cache: Vec<SolverCacheIndex>,
    ) -> (Block, Prio) {
        (Block::from_single_node_id(NodeID::empty()), BASE)
    }
}
