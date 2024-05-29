mod block_preview;
mod empty;
mod hull;
pub mod solver;

use crate::node::{Material, Node, NodeID, NODE_INDEX_ANY, NODE_INDEX_EMPTY};
use crate::rules::block_preview::BlockPreview;
use crate::rules::solver::Solver;
use crate::voxel_loader::VoxelLoader;
use octa_force::anyhow::Result;

const NODE_ID_MAP_INDEX_NONE: usize = NODE_INDEX_EMPTY;
const NODE_ID_MAP_INDEX_ANY: usize = NODE_INDEX_ANY;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Prio {
    ZERO,
    BASE,

    HULL0,
    HULL1,
    HULL2,
    HULL3,
    HULL4,
    HULL5,
    HULL6,
    HULL7,
    HULL8,
    HULL9,
    HULL10,
}

pub struct Rules {
    pub materials: [Material; 256],
    pub nodes: Vec<Node>,

    pub block_names: Vec<String>,
    pub block_previews: Vec<BlockPreview>,

    pub duplicate_node_ids: Vec<Vec<Vec<NodeID>>>,

    pub solvers: Vec<Box<dyn Solver>>,
}

impl Rules {
    pub fn new(voxel_loader: VoxelLoader) -> Result<Self> {
        let mut rules = Rules {
            materials: voxel_loader.load_materials(),
            nodes: vec![],
            block_names: vec![],
            block_previews: vec![],
            duplicate_node_ids: vec![],
            solvers: vec![],
        };

        rules.make_empty();
        rules.make_hull(&voxel_loader)?;

        Ok(rules)
    }

    pub fn get_duplicate_node_id(&mut self, node_id: NodeID) -> NodeID {
        let node = &self.nodes[node_id.index];

        while self.duplicate_node_ids.len() <= node_id.index {
            self.duplicate_node_ids.push(vec![])
        }

        let mut new_node_id = None;
        for ids in self.duplicate_node_ids[node_id.index].iter_mut() {
            if ids.contains(&node_id) {
                new_node_id = Some(ids[0]);
                break;
            }

            if node.is_duplicate_node_id(node_id.rot, node, ids[0].rot) {
                ids.push(node_id);
                new_node_id = Some(ids[0]);
                break;
            }
        }

        if new_node_id.is_none() {
            self.duplicate_node_ids[node_id.index].push(vec![node_id]);
            new_node_id = Some(node_id);
        }

        new_node_id.unwrap()
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
}
