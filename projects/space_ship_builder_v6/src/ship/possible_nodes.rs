use crate::node::{BlockIndex, NodeID};
use crate::rules::Prio;

#[derive(Clone, Default)]
pub struct PossibleNodes {
    nodes: Vec<(BlockIndex, Vec<(NodeID, Prio)>)>,
}

impl PossibleNodes {
    fn get_block_index(&mut self, block: BlockIndex) -> usize {
        let res = self
            .nodes
            .binary_search_by(|(test_index, _)| test_index.cmp(&block));
        if res.is_ok() {
            res.unwrap()
        } else {
            let new_index = res.err().unwrap();
            self.nodes.insert(new_index, (block, vec![]));
            new_index
        }
    }

    pub fn set_node_ids(&mut self, block: BlockIndex, node_ids: Vec<(NodeID, Prio)>) {
        let index = self.get_block_index(block);
        self.nodes[index].1 = node_ids;
    }

    pub fn has_node_id(&mut self, block: BlockIndex, node_id: &NodeID) -> bool {
        let index = self.get_block_index(block);
        self.nodes[index]
            .1
            .iter()
            .find(|(id, _)| *id == *node_id)
            .is_some()
    }

    pub fn get_node_ids(&mut self, block: BlockIndex) -> &[(NodeID, Prio)] {
        let index = self.get_block_index(block);
        &self.nodes[index].1
    }

    pub fn get_all(&self) -> impl Iterator<Item = &(NodeID, Prio)> {
        self.nodes.iter().flat_map(|(_, ids)| ids)
    }
}
