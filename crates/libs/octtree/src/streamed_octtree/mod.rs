use std::collections::VecDeque;

use app::anyhow::{format_err};
use::app::anyhow::Result;

use crate::{Tree, TreeType, octtree_node::OcttreeNode, file::{metadata::{Metadata, self}, load::load_batch}};

#[derive(Clone)]
pub struct StreamedOcttree {
    pub metadata: Metadata,
    pub batches: VecDeque<Batch>,
    pub folder_path: String,
    pub loaded_max_size: usize,
}

#[derive(Clone)]
pub struct Batch {
    index: usize,
    nodes: Vec<OcttreeNode>,
}

impl StreamedOcttree {
    pub fn new(folder_path: &str, loaded_max_size: usize) -> Result<Self> {
        let metadata = Metadata::load(folder_path)?;
        let batches = VecDeque::new();

        Ok(Self { 
            metadata, 
            batches, 
            folder_path: folder_path.to_owned(),
            loaded_max_size,
        })
    }

    fn get_batch(&mut self, index: usize ) -> Result<&Batch> {
        let r =  self.batches.iter().position(|b| b.index == index);
        match r {
            Some(index) => {
                
                let batch = self.batches.swap_remove_front(index).unwrap();
                self.batches.push_front(batch);

                return Ok(&self.batches.front().unwrap())
            },
            None => return Err(format_err!("Batch {index} no loaded.")),
        };
    }

    fn load_batch(&mut self, index: usize) -> Result<&Batch> {
        let nodes = load_batch(&self.folder_path, index, self.metadata.get_batch_metadata(index)?.size as usize)?;

        let batch = Batch{index, nodes};
        self.batches.push_front(batch);

        self.batches.truncate(self.loaded_max_size);

        Ok(&self.batches.front().unwrap())
    }

    pub fn get_loaded_size(&self) -> usize {
        self.batches.len()
    }

    pub fn get_loaded_max_size(&self) -> usize {
        self.loaded_max_size
    }
}


impl Tree for StreamedOcttree {
    fn tree_type(&self) -> TreeType {
        TreeType::Streamed
    }

    fn get_node(&mut self, id: u64) -> Result<OcttreeNode> {
        let batch_index = (id as usize) / self.metadata.batch_size;

        let r = self.get_batch(batch_index);
        let batch = if r.is_ok() {
            r.unwrap()
        } else {
            self.load_batch(batch_index)?
        };

        let r = batch.nodes.binary_search_by(|node| node.get_node_id().cmp(&id));
        match r {
            Ok(i) => Ok(batch.nodes[i]),
            Err(_) => Err(format_err!("Requested Node {:?} not found!", id)),
        }
    }

    fn get_node_by_index(&mut self, index: usize) -> Result<OcttreeNode> {
        todo!()
    }

    fn get_depth(&self) -> u16 {
        self.metadata.depth
    }

    fn get_size(&self) -> u64 {
        self.metadata.size
    }

    fn get_max_size(&self) -> u64 {
        self.metadata.max_size
    }
    
}