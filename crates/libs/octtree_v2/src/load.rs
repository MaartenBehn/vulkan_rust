use std::fs::File;

use app::anyhow::Result;
use std::io::Read;

use crate::node::Node;

use super::util;

pub fn load_page(folder_path: &str, page_nr: usize, page_size: usize) -> Result<Vec<Node>> {
    let path = format!("{folder_path}/{page_nr}.nodes");
    let mut file = File::open(path)?;

    let mut nodes: Vec<Node> = vec![Node::default(); page_size];

    let buffer = unsafe { util::vec_as_u8_slice_mut(&mut nodes) };

    file.read(buffer)?;

    Ok(nodes)
}
