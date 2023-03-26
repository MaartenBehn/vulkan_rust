use app::{log, anyhow::format_err};
use app::anyhow::Result;
use rand::Rng;

use crate::{octtree_node::OcttreeNode, basic_octtree::sparce_tree::CreateSparceOcttreeData, Tree, TreeType, get_max_tree_size};

mod sphere;
mod sparce_tree;

pub enum InitalFill {
    Sphere,
    SpareseTree,
}

#[derive(Clone)]
pub struct BasicOcttree {
    nodes: Vec<OcttreeNode>,

    depth: u16,
    max_size: u64,
    size: u64,
}

impl BasicOcttree {
    pub fn new(depth: u16, mut seed: u64, fill_kind: InitalFill) -> Self {
        let mut octtree = BasicOcttree{
            nodes: Vec::new(),
            depth: depth,
            max_size: get_max_tree_size(depth),
            size: 0,
        };

        if seed == 0 {
            let mut seed_rng= rand::thread_rng();
            seed = seed_rng.gen();
        }

        log::info!("Octtree Seed: {:?}", seed);
        
        match fill_kind {
            InitalFill::Sphere => {
                log::info!("Building Sphere.");
                octtree.inital_fill_sphere(0, 0, [0, 0, 0]); 
            },
            InitalFill::SpareseTree => {
                log::info!("Building Sparse Octtree:");
                octtree.inital_fill_sparse_tree(0, 0, [0, 0, 0], true, &mut CreateSparceOcttreeData::new(seed, octtree.max_size));
            },
        }

        octtree.size = octtree.nodes.len() as u64;

        return octtree;
    }
}

impl Tree for BasicOcttree {
    fn tree_type(&self) -> TreeType {
        TreeType::Basic
    }

    fn get_node(&self, id: u64) -> Result<OcttreeNode> {

        let r = self.nodes.binary_search_by(|node| node.get_node_id().cmp(&id));
        match r {
            Ok(_) => Ok(self.nodes[r.unwrap()]),
            Err(_) => Err(format_err!("Requested Node {:?} not found!", id)),
        }
    }

    fn get_node_by_index(&self, index: usize) -> Result<OcttreeNode> {
        Ok(self.nodes[index])
    }

    fn get_depth(&self) -> u16 {
        self.depth
    }

    fn get_size(&self) -> u64 {
        self.size
    }

    fn get_max_size(&self) -> u64 {
        self.max_size
    }

    
}