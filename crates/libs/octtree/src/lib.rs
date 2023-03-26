use file::save::save_tree;

use octtree_node::OcttreeNode;
use app::{anyhow::Result};

pub mod octtree_node;
pub mod basic_octtree;
pub mod streamed_octtree;

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

pub enum TreeType {
    Basic,
    Streamed
}

pub trait Tree {
    fn tree_type(&self) -> TreeType;

    fn get_node(&mut self, id: u64) -> Result<OcttreeNode>;
    fn get_node_by_index(&mut self, index: usize) -> Result<OcttreeNode>;

    fn get_depth(&self) -> u16;
    fn get_size(&self) -> u64;
    fn get_max_size(&self) -> u64;

    // Default Funcs
    fn get_child_id(&self, node_id: u64, child_nr: usize, depth: u16) -> u64 {
        let child_size = get_max_tree_size(self.get_depth() - depth - 1);
        return node_id + child_size * (child_nr as u64) + 1;
    }

    fn save(&mut self, folder_path: &str, batch_size: usize) -> Result<()> where Self: Sized {
        save_tree(self, folder_path, batch_size)?;
        Ok(())
    }
} 

pub fn get_max_tree_size(depth: u16) -> u64 {
    ((i64::pow(8, (depth + 1) as u32) - 1) / 7) as u64
}

pub fn get_node_size(depth: u16) -> u64 {
    i64::pow(2, depth as u32) as u64
}

