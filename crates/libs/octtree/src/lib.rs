use app::log;
use octtree_node::OcttreeNode;
use rand::Rng;

use crate::{sparce_tree::CreateSparceOcttreeData};

pub mod octtree_node;
mod sphere;
mod sparce_tree;
mod file;

const OCTTREE_CONFIG: [[u64; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 0],
    [0, 1, 1],
    [1, 0, 0],
    [1, 0, 1],
    [1, 1, 0],
    [1, 1, 1],
];

pub enum OcttreeFill {
    Sphere,
    SpareseTree,
}

#[derive(Clone)]
pub struct Octtree {
    pub nodes: Vec<OcttreeNode>,

    pub depth: u16,
    pub max_size: u64,
    pub size: u64,
}

impl Octtree{
    pub fn new(depth: u16, mut seed: u64, fill_kind: OcttreeFill) -> Self {
        let mut octtree = Octtree{
            nodes: Vec::new(),
            depth: depth,
            max_size: Self::get_max_tree_size(depth),
            size: 0,
        };

        if seed == 0 {
            let mut seed_rng= rand::thread_rng();
            seed = seed_rng.gen();
        }

        log::info!("Octtree Seed: {:?}", seed);
        
        match fill_kind {
            OcttreeFill::Sphere => {
                log::info!("Building Sphere.");
                octtree.inital_fill_sphere(0, 0, [0, 0, 0]); 
            },
            OcttreeFill::SpareseTree => {
                log::info!("Building Sparse Octtree:");
                octtree.inital_fill_sparse_tree(0, 0, [0, 0, 0], true, &mut CreateSparceOcttreeData::new(seed, octtree.max_size));
            },
        }

        octtree.size = octtree.nodes.len() as u64;

        return octtree;
    }

    pub fn get_max_tree_size(depth: u16) -> u64 {
        ((i64::pow(8, (depth + 1) as u32) - 1) / 7) as u64
    }
    
    pub fn get_child_id(&self, node_id: u64, child_nr: usize, depth: u16) -> u64 {
        let child_size = Self::get_max_tree_size(self.depth - depth - 1);
        return node_id + child_size * (child_nr as u64) + 1;
    }

    pub fn get_node_size(depth: u16) -> u64 {
        i64::pow(2, depth as u32) as u64
    }
}

