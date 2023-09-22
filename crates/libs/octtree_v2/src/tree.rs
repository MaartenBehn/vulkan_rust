use std::{collections::HashMap, io::{Write, Read}, fs::File};

use app::anyhow::Result;

use crate::{node::CompressedNode, util, metadata::Metadata, Tree, Node};

#[derive(Clone, Debug, Default)]
pub struct CompressedTree {
    path: String,
    metadata: Metadata,
    pages: HashMap<usize, CompressedPage>,
}

#[derive(Clone, Debug, Default)]
pub struct CompressedPage {
    nodes: Vec<CompressedNode>,
} 

impl Tree for CompressedTree {
    fn new(path: String, page_size: usize, depth: usize) -> Self {
        CompressedTree { 
            path, 
            metadata: Metadata::new(page_size, 0, depth),
            pages: HashMap::new(),
        }
    }

    fn form_disk(path: String) -> Result<Self> {
        let metadata = Metadata::from_file(&path)?;
        Ok(CompressedTree { 
            path, 
            metadata,
            pages: HashMap::new(),
        })
    }

    fn has_page(&self, page_nr: usize) -> bool {
        self.pages.contains_key(&page_nr)
    }

    fn get_all_page_nrs(&self) -> Vec<usize> {
        self.pages.keys().cloned().collect()
    }

    fn add_empty_page(&mut self, page_nr: usize) {
        self.pages.insert(page_nr, CompressedPage::new(self.metadata.page_size));
        self.metadata.page_ammount += 1;
    }

    fn remove_page(&mut self, page_nr: usize) {
        self.pages.remove(&page_nr);
    }

    fn set_node(&mut self, page_nr: usize, in_page_index: usize, node: Node) -> Result<()> {
        self.pages.get_mut(&page_nr).unwrap().nodes[in_page_index] = node.try_into()?;  

        Ok(())
    }

    fn get_node(&self, page_nr: usize, in_page_index: usize) -> Node {
        let node = self.pages.get(&page_nr).unwrap().nodes[in_page_index];
        Node::Compressed(node)
    }

    fn get_metadata(&self) -> Metadata {
        self.metadata
    }

    fn save_metadata(&self) -> Result<()> {
        self.metadata.save(&self.path)
    }

    fn save_page(&self, page_nr: usize) -> Result<()> {
        let page = self.pages.get(&page_nr).unwrap();
        
        let mut file = File::create(format!("{}/page_{}.bin", self.path, page_nr))?;
        let buffer = unsafe { util::slice_as_u8_slice(&page.nodes) };
        file.write_all(buffer)?;

        Ok(())
    }

    fn load_page(&mut self, page_nr: usize) -> Result<()> {
        let mut page = CompressedPage::new(self.metadata.page_size);

        let mut file = File::create(format!("{}/page_{}.bin", self.path, page_nr))?;
        let buffer = unsafe { util::slice_as_u8_slice_mut(&mut page.nodes) };
        file.read(buffer)?;

        Ok(())
    }
}

impl CompressedPage {
    fn new(page_size: usize) -> CompressedPage {
        CompressedPage { nodes: vec![CompressedNode::default(); page_size] }
    }
}
