use crate::node::{Node, NodeID};
use crate::rules::block_preview::BlockPreview;
use crate::rules::solver::{push_in_block_affected_nodes, Solver};
use crate::rules::{Prio, Rules};
use crate::ship::data::ShipData;
use crate::ship::possible_nodes::NodeData;
use octa_force::glam::IVec3;

pub struct EmptySolver {}

impl Rules {
    pub fn make_empty(&mut self) {
        self.block_names.push("Empty".to_owned());
        self.block_previews.push(BlockPreview::default());
        self.solvers.push(Box::new(EmptySolver {}));
        self.nodes.push(Node::default());
    }
}

impl Solver for EmptySolver {
    fn push_block_affected_nodes(&self, ship: &mut ShipData, block_pos: IVec3) {}

    fn block_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData> {
        vec![]
    }

    fn node_check_reset(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData> {
        vec![]
    }

    fn node_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData> {
        vec![]
    }
}
