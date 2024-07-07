mod basic_blocks;
pub mod empty;
pub mod hull;
pub mod req_tree;
pub mod solver;
pub mod stone;

use crate::math::oct_positions;
use crate::math::rotation::Rot;
use crate::rules::solver::Solver;
use crate::world::data::block::{Block, BlockNameIndex};
use crate::world::data::node::{Material, Node, NodeID};
use crate::world::data::voxel_loader::VoxelLoader;
use octa_force::anyhow::{bail, Result};
use octa_force::glam::{IVec3, UVec3};

const BLOCK_MODEL_IDENTIFIER: &str = "B";
const FOLDER_MODEL_IDENTIFIER: &str = "F";
const BLOCK_TYPE_IDENTIFIER: &str = "Block";
const REQ_TYPE_IDENTIFIER: &str = "Req";

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Debug)]
pub enum Prio {
    #[default]
    Zero,
    Empty,
    Basic(usize),
    Multi(usize),
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
        rules.make_stone(voxel_loader)?;

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

    pub fn get_block_name_index(&self, name: &str) -> BlockNameIndex {
        self.block_names
            .iter()
            .position(|test_name| test_name == name)
            .unwrap()
    }
}

// Helper functions
impl Rules {
    pub(crate) fn load_node(&mut self, name: &str, voxel_loader: &VoxelLoader) -> Result<NodeID> {
        let (model_index, rot) = voxel_loader.find_model_by_name(name)?;
        let node = voxel_loader.load_node_model(model_index)?;

        let id = self.add_node(node, rot);
        let dup_id = self.get_duplicate_node_id(id);

        Ok(dup_id)
    }

    #[allow(unused)]
    fn load_block_from_multi_node_by_name(
        &mut self,
        name: &str,
        voxel_loader: &VoxelLoader,
    ) -> Result<Block> {
        let (model_index, _) = voxel_loader.find_model_by_name(name)?;
        let (size, nodes) = voxel_loader.load_multi_node_model(model_index)?;

        if size != (IVec3::ONE * 2) {
            bail!("{} not multi block Size of [2, 2, 2]", size)
        }

        let mut node_ids = vec![];
        for node in nodes {
            let id = self.add_node(node, Rot::IDENTITY);
            let dup_id = self.get_duplicate_node_id(id);
            node_ids.push(dup_id);
        }

        Ok(Block::from_node_ids_slice(&node_ids))
    }

    fn load_block_from_block_model_by_index(
        &mut self,
        index: usize,
        voxel_loader: &VoxelLoader,
    ) -> Result<Block> {
        let (model_index, _) = voxel_loader.find_model_by_index(index)?;
        let (size, nodes) = voxel_loader.load_multi_node_model(model_index)?;

        if size != (IVec3::ONE * 2) {
            bail!("{} not multi block Size of [2, 2, 2]", size)
        }

        let mut node_ids = vec![];
        for node in nodes {
            let id = self.add_node(node, Rot::IDENTITY);
            let dup_id = self.get_duplicate_node_id(id);
            node_ids.push(dup_id);
        }

        Ok(Block::from_node_ids_slice(&node_ids))
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

impl Prio {
    pub fn is_less(&self, other: &Prio) -> bool {
        match self {
            Prio::Basic(a) => match other {
                Prio::Basic(b) => a < b,
                _ => self < other,
            },
            Prio::Multi(a) => match other {
                Prio::Multi(b) => a < b,
                _ => self < other,
            },
            _ => self < other,
        }
    }
}
