use std::{fs::{File, self}, io::BufReader, mem::size_of};

use crate::octtree_node::OcttreeNode;

use app::anyhow::{ensure, Result};
use byteorder::{LittleEndian, ReadBytesExt}; // 1.2.7
use std::{
    io::{self, Read},
};

impl OcttreeNode {
    fn from_reader(mut rdr: impl Read) -> io::Result<Self> {
        let node_id_0 = rdr.read_u32::<LittleEndian>()?;
        let node_id_1 = rdr.read_u32::<LittleEndian>()?;
        let mat_id    = rdr.read_u32::<LittleEndian>()?;
        let bit_field = rdr.read_u32::<LittleEndian>()?;

        Ok(OcttreeNode::from_data(node_id_0, node_id_1, mat_id, bit_field))
    }
}

pub fn load_batch(folder_path: &str, index: usize, size: u64) -> Result<Vec<OcttreeNode>> {

    let path = format!("{folder_path}/{index}.nodes");
    let file = File::open(path)?;

    let mut nodes: Vec<OcttreeNode> = Vec::new();
    for _ in 0..size {
        nodes.push(OcttreeNode::from_reader(&file)?);
    }

    Ok(nodes)
}