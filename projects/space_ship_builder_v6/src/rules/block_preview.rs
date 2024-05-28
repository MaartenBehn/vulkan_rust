use crate::node::NodeID;

#[derive(Copy, Clone, Default, Debug)]
pub struct BlockPreview {
    node_ids: [NodeID; 8],
}

impl BlockPreview {
    pub fn from_single_node_id(node_id: NodeID) -> Self {
        let mut node_ids = [NodeID::empty(); 8];
        for (i, r) in node_id.rot.get_all_flipped().into_iter().enumerate() {
            node_ids[i] = NodeID::new(node_id.index, r);
        }

        Self { node_ids }
    }
}
