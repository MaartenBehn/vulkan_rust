use crate::node::NodeID;

pub type BlockNameIndex = usize;
pub const BLOCK_INDEX_EMPTY: BlockNameIndex = 0;

pub type BlockIndex = usize;

#[derive(Copy, Clone, Default, Debug)]
pub struct Block {
    pub node_ids: [NodeID; 8],
}

impl Block {
    pub fn from_node_ids(node_ids: [NodeID; 8]) -> Self {
        Self { node_ids }
    }

    pub fn from_single_node_id(node_id: NodeID) -> Self {
        let mut node_ids = [NodeID::empty(); 8];
        for (i, r) in node_id.rot.get_all_flipped().into_iter().enumerate() {
            node_ids[i] = NodeID::new(node_id.index, r);
        }

        Self { node_ids }
    }
}
