use::app::anyhow::Result;

use crate::{Tree, TreeType, octtree_node::OcttreeNode};


#[derive(Clone)]
pub struct StreamedOcttree {
    

    pub depth: u16,
    pub max_size: u64,
    pub size: u64,
}

impl StreamedOcttree {
    pub fn new(folder_path: &str){
        
    }
}


impl Tree for StreamedOcttree {
    fn tree_type(&self) -> TreeType {
        TreeType::Streamed
    }

    fn get_node(&self, id: u64) -> Result<OcttreeNode> {
        todo!()
    }

    fn get_node_by_index(&self, index: usize) -> Result<OcttreeNode> {
        todo!()
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