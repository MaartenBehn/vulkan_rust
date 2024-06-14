pub mod block;
pub mod empty;
pub mod hull;
pub mod solver;

use crate::math::oct_positions;
use crate::node::{Material, Node, NodeID, NodeIndex, NODE_INDEX_ANY, NODE_INDEX_EMPTY};
use crate::rotation::Rot;
use crate::rules::block::Block;
use crate::rules::solver::Solver;
use crate::voxel_loader::VoxelLoader;
use octa_force::anyhow::{bail, Result};
use octa_force::glam::{uvec3, UVec3};
use std::ops::Mul;

const NODE_ID_MAP_INDEX_NONE: usize = NODE_INDEX_EMPTY;
const NODE_ID_MAP_INDEX_ANY: usize = NODE_INDEX_ANY;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Debug)]
pub enum Prio {
    #[default]
    ZERO,
    BASE,

    HULL_BASE0,
    HULL_BASE1,
    HULL_BASE2,
    HULL_BASE3,
    HULL_BASE4,
    HULL_BASE5,
    HULL_BASE6,
    HULL_BASE7,
    HULL_BASE8,

    HULL_FILL0,
    HULL_FILL1,
    HULL_FILL2,
    HULL_FILL3,
    HULL_FILL4,
    HULL_FILL5,
    HULL_FILL6,
    HULL_FILL7,
    HULL_FILL8,
    HULL_FILL9,
}

pub struct Rules {
    pub materials: [Material; 256],
    pub nodes: Vec<Node>,
    pub duplicate_node_ids: Vec<Vec<Vec<NodeID>>>,

    pub block_names: Vec<String>,
    pub solvers: Vec<Box<dyn Solver>>,
}

impl Rules {
    pub fn new(voxel_loader: &VoxelLoader) -> Result<Self> {
        let mut rules = Rules {
            materials: voxel_loader.load_materials(),
            nodes: vec![],
            duplicate_node_ids: vec![vec![vec![NodeID::default()]]],
            block_names: vec![],
            solvers: vec![],
        };

        rules.make_empty();
        rules.make_hull(voxel_loader)?;

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

    pub fn add_node(&mut self, node: Node, rot: Rot) -> NodeID {
        let rots = Rot::IDENTITY.get_all_permutations();

        let mut id = None;
        for (i, test_node) in self.nodes.iter().enumerate() {
            for test_rot in rots.iter() {
                if node.is_duplicate_node_id(rot, test_node, *test_rot) {
                    id = Some(NodeID::new(i, *test_rot));
                }
            }
        }

        if id.is_none() {
            id = Some(NodeID::new(self.nodes.len(), rot));
            self.nodes.push(node);
        }

        id.unwrap()
    }
}

// Helper functions
impl Rules {
    pub(crate) fn load_node(&mut self, name: &str, voxel_loader: &VoxelLoader) -> Result<NodeID> {
        let (model_index, rot) = voxel_loader.find_model(name)?;
        let node = voxel_loader.load_node_model(model_index)?;

        let id = self.add_node(node, rot);
        let dup_id = self.get_duplicate_node_id(id);

        Ok(dup_id)
    }

    fn load_multi_node(
        &mut self,
        name: &str,
        voxel_loader: &VoxelLoader,
    ) -> Result<(UVec3, Vec<NodeID>)> {
        let (model_index, rot) = voxel_loader.find_model(name)?;
        let (size, nodes) = voxel_loader.load_multi_node_model(model_index)?;

        let mut node_ids = vec![];
        for node in nodes {
            let id = self.add_node(node, rot);
            let dup_id = self.get_duplicate_node_id(id);
            node_ids.push(dup_id);
        }

        Ok((size, node_ids))
    }

    fn load_block_from_node_folder(
        &mut self,
        name: &str,
        voxel_loader: &VoxelLoader,
    ) -> Result<Block> {
        let (size, nodes) = voxel_loader.load_node_folder_models(name)?;
        if size != UVec3::ONE * 4 {
            bail!("Node folder size is {} not (4, 4, 4)", size);
        }

        let mut node_ids = vec![];

        for offset in oct_positions() {
            let mut found = false;
            for (node, rot, pos) in nodes.iter() {
                if offset.as_uvec3() == *pos / 4 {
                    found = true;

                    let id = self.add_node(node.to_owned(), *rot);
                    let dup_id = self.get_duplicate_node_id(id);
                    node_ids.push(dup_id);
                    break;
                }
            }

            if !found {
                bail!("Offset {} is not in node folder!", offset)
            }
        }

        Ok(Block::from_node_ids(node_ids.try_into().unwrap()))
    }
}
