use octa_force::glam::IVec3;
use crate::math::oct_positions;
use crate::ship::data::ShipData;

pub trait Solver {

    fn push_block_affected_nodes(&mut self, ship: &mut ShipData, block_pos: IVec3);
    fn block_check(&mut self, ship: &mut ShipData, node_pos: IVec3, node_index: usize, chunk_index: usize);
    fn node_check(&mut self, ship: &mut ShipData, node_pos: IVec3, node_index: usize, chunk_index: usize);
}

pub fn push_in_block_affected_nodes(ship: &mut ShipData, block_pos: IVec3) {
    for offset in oct_positions() {
        let affected_pos = block_pos + offset;

        let chunk_index = ship.get_chunk_index_from_node_pos(affected_pos);
        let node_index = ship.get_node_index(affected_pos);
        let node_world_index = ship.to_world_node_index(chunk_index, node_index);
        ship.block_changed.push_back(node_world_index);
    }
}