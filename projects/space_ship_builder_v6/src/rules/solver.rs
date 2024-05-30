use crate::math::oct_positions;
use crate::node::{BlockIndex, NodeID};
use crate::rules::Prio;
use crate::ship::data::{CacheIndex, ShipData};
use octa_force::glam::IVec3;
use crate::ship::possible_nodes::NodeData;

pub trait Solver {
    fn push_block_affected_nodes(&self, ship: &mut ShipData, block_pos: IVec3);
    fn block_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData>;
    fn node_check_reset(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData>;
    fn node_check(
        &self,
        ship: &mut ShipData,
        node_index: usize,
        chunk_index: usize,
        world_node_pos: IVec3,
    ) -> Vec<NodeData>;
}

pub fn push_in_block_affected_nodes(
    ship: &mut ShipData,
    block_pos: IVec3,
    block_index: BlockIndex,
) {
    for offset in oct_positions() {
        let affected_pos = block_pos + offset;

        let chunk_index = ship.get_chunk_index_from_node_pos(affected_pos);
        let node_index = ship.get_node_index(affected_pos);
        let order =
            ship.order_controller
                .pack_order_with_block(block_index, node_index, chunk_index);
        ship.block_changed.push_back(order);
    }
}
