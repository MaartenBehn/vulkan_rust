use std::{fs::{File, self}, io::Write};
use json::{self, object};

use indicatif::ProgressBar;
use octtree_node::OcttreeNode;
use app::{anyhow::Result, log};

pub mod octtree_node;
pub mod basic_octtree;
pub mod streamed_octtree;

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

    fn get_node(&self, id: u64) -> Result<OcttreeNode>;
    fn get_node_by_index(&self, index: usize) -> Result<OcttreeNode>;

    fn get_depth(&self) -> u16;
    fn get_size(&self) -> u64;
    fn get_max_size(&self) -> u64;

    // Default Funcs
    fn get_child_id(&self, node_id: u64, child_nr: usize, depth: u16) -> u64 {
        let child_size = get_max_tree_size(self.get_depth() - depth - 1);
        return node_id + child_size * (child_nr as u64) + 1;
    }

    fn save_to_file(&self, folder_path: &str, nodes_per_file: u64) -> Result<()> {

        log::info!("Writing Tree:");

        let bar = ProgressBar::new(self.get_size());

        let _ = fs::remove_dir_all(folder_path);
        fs::create_dir(folder_path)?;

        let mut node_in_file = 0;
        let mut file_start_id = 0;
        let mut file: Option<File> = None;
        let mut start_ranges: Vec<u64> = Vec::new();
        let mut end_ranges: Vec<u64> = Vec::new();
        let mut file_counter = 0;

        for i in 0..self.get_size() {

            bar.set_position(i);

            let node = &self.get_node_by_index(i as usize)?;

            if node_in_file == 0 {
                file_start_id = node.get_node_id();
                
                file = Some(File::create(format!("{folder_path}/{file_counter}.nodes"))?);
                file_counter += 1;
            }
            
            let data = unsafe { 
                any_as_u8_slice(node) 
            };

            file.as_ref().unwrap().write(data)?;

            node_in_file += 1;
            if node_in_file >= nodes_per_file || i >= self.get_size() - 1{
                node_in_file = 0;

                if file.is_some() {
                    file = None;

                    let file_end_id = self.get_node_by_index((i - 1) as usize)?.get_node_id();
                    
                    start_ranges.push(file_start_id);
                    end_ranges.push(file_end_id);
                }   
            }
        }

        // Metadata
        {
            let metadata = object! {
                "depth": self.get_depth(),
                "size": self.get_size(),
                "max_size": self.get_max_size(),
                "start_ranges": start_ranges,
                "end_ranges": end_ranges,
            };
            let mut file = File::create(format!("{folder_path}/metadata.json"))?;
            file.write(json::stringify_pretty(metadata, 4).as_bytes())?;
        }

        Ok(())
    }
} 

pub fn get_max_tree_size(depth: u16) -> u64 {
    ((i64::pow(8, (depth + 1) as u32) - 1) / 7) as u64
}

pub fn get_node_size(depth: u16) -> u64 {
    i64::pow(2, depth as u32) as u64
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
    )
}