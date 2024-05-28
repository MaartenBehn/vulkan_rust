use crate::math::in_node_positions;
use crate::node::{BlockIndex, Material, Node, NodeID, NODE_INDEX_ANY, NODE_INDEX_EMPTY};
use crate::voxel_loader::VoxelLoader;
use octa_force::{anyhow::Result, glam::IVec3};

const NODE_ID_MAP_INDEX_NONE: usize = NODE_INDEX_EMPTY;
const NODE_ID_MAP_INDEX_ANY: usize = NODE_INDEX_ANY;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Prio {
    ZERO,
    BASE,
}

pub struct Rules {
    pub materials: [Material; 256],
    pub nodes: Vec<Node>,
    pub block_names: Vec<String>,
    pub block_preview_node_ids: Vec<Vec<(IVec3, NodeID)>>,

    pub map_rules_index_to_node_id: Vec<Vec<NodeID>>,

    pub node_rules: Vec<Vec<(IVec3, Vec<NodeID>)>>,
    pub block_rules: Vec<Vec<(Vec<(IVec3, BlockIndex)>, Prio)>>,

    pub affected_by_block: Vec<Vec<IVec3>>,
}

impl Rules {
    pub fn new(voxel_loader: VoxelLoader) -> Result<Self> {
        let mut rules = Rules {
            materials: voxel_loader.load_materials(),
            nodes: vec![],
            block_names: vec!["Empty".to_owned()],
            block_preview_node_ids: vec![vec![]],
            map_rules_index_to_node_id: vec![],
            node_rules: vec![],
            block_rules: vec![],
            affected_by_block: vec![],
        };

        rules.make_hull(&voxel_loader)?;

        Ok(rules)
    }

    fn make_hull(&mut self, voxel_loader: &VoxelLoader) -> Result<()> {
        self.block_names.push("Hull".to_owned());

        let mut nodes = vec![];
        let max_hull_node = 8;
        for i in 1..max_hull_node {
            let node_id = self.add_node(&format!("Hull-{i}"), voxel_loader)?;
            nodes.push(node_id);
        }

        self.block_preview_node_ids
            .push(Self::make_preview(nodes[0]));

        Ok(())
    }
}

// Helper functions
impl Rules {
    fn add_node(&mut self, name: &str, voxel_loader: &VoxelLoader) -> Result<NodeID> {
        let (model_index, rot) = voxel_loader.find_model(name)?;
        let node = voxel_loader.load_node_model(model_index)?;

        self.nodes.push(node);

        Ok(NodeID::new(self.nodes.len() - 1, rot))
    }

    fn make_preview(node_id: NodeID) -> Vec<(IVec3, NodeID)> {
        node_id
            .rot
            .get_all_flipped()
            .into_iter()
            .zip(in_node_positions().into_iter())
            .map(|(r, pos)| (pos, NodeID::new(node_id.index, r)))
            .collect()
    }

    fn add_sides_match_node_rules(&mut self, node_ids: &[NodeID]) {
        for node_id in node_ids.to_owned() {
            for test_id in node_ids {}
        }
    }
}
