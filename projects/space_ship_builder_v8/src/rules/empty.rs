use crate::node::{Node, NodeID};
use crate::rules::block::Block;
use crate::rules::solver::{Solver, SolverCacheIndex};
use crate::rules::Prio::Empty;
use crate::rules::{Prio, Rules};
use crate::ship::data::ShipData;
use octa_force::glam::IVec3;

pub const EMPTY_BLOCK_NAME_INDEX: usize = 0;

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
        _: &mut ShipData,
        _: usize,
        _: usize,
        _: IVec3,
    ) -> Vec<SolverCacheIndex> {
        vec![]
    }

    fn block_check(
        &self,
        _: &mut ShipData,
        _: usize,
        _: usize,
        _: IVec3,
        _: Vec<SolverCacheIndex>,
    ) -> Vec<SolverCacheIndex> {
        vec![]
    }

    fn get_block(
        &self,
        _: &mut ShipData,
        _: usize,
        _: usize,
        _: IVec3,
        _: Vec<SolverCacheIndex>,
    ) -> (Block, Prio, usize) {
        (Block::from_single_node_id(NodeID::empty()), Empty, 0)
    }

    fn get_block_from_cache_index(&self, _: usize) -> Block {
        Block::from_single_node_id(NodeID::empty())
    }
}
