use std::{fs::{File, self}, io::BufReader, mem::{size_of, self}};

use crate::octtree_node::OcttreeNode;

use app::anyhow::{ensure, Result};
use std::{io::{self, Read}};

use super::util;

pub fn load_batch(folder_path: &str, index: usize, size: usize) -> Result<Vec<OcttreeNode>> {

    let path = format!("{folder_path}/{index}.nodes");
    let mut file = File::open(path)?;

    let mut nodes: Vec<OcttreeNode> = vec![OcttreeNode::default(); size];

    let buffer = unsafe{ util::vec_as_u8_slice_mut(&mut nodes) };

    file.read(buffer)?;
    
    Ok(nodes)
}
