use std::fs::File;

use crate::octtree_node::OcttreeNode;

use app::anyhow::Result;
use std::io::Read;

use super::util;

pub fn load_batch(folder_path: &str, index: usize, size: usize) -> Result<Vec<OcttreeNode>> {
    let path = format!("{folder_path}/{index}.nodes");
    let mut file = File::open(path)?;

    let mut nodes: Vec<OcttreeNode> = vec![OcttreeNode::default(); size];

    let buffer = unsafe { util::vec_as_u8_slice_mut(&mut nodes) };

    file.read(buffer)?;

    Ok(nodes)
}
