use std::collections::HashMap;

use app::anyhow::Result;
use speedy::{Readable, Writable};

use crate::{metadata::Metadata, Tree, Node};

#[derive(Clone, Debug, Default)]
pub struct TemplateTree {
    path: String,
    metadata: Metadata,
    pages: HashMap<usize, TemplatePage>,
}

#[derive(Clone, Debug, Default, Readable, Writable)]
pub struct TemplatePage {
    nodes: Vec<TemplateNode>,
}

#[derive(Clone, Copy, Debug, Default, Readable, Writable)]
pub struct TemplateNode {
    ptr: u64,
    branches: [bool; 8],
    materials: [u8; 8],
}

impl Tree for TemplateTree {
    fn new(path: String, page_size: usize, depth: usize) -> Self {
        TemplateTree { 
            path, 
            metadata: Metadata::new(page_size, 0, depth),
            pages: HashMap::new(),
        }
    }

    fn form_disk(path: String) -> Result<Self> {
        let metadata = Metadata::from_file(&path)?;
        Ok(TemplateTree { 
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
        self.pages.insert(page_nr, TemplatePage::new(self.metadata.page_size));
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
        Node::Template(node)
    }

    fn get_metadata(&self) -> Metadata {
        self.metadata
    }

    fn save_metadata(&self) -> Result<()> {
        self.metadata.save(&self.path)
    }

    fn save_page(&self, page_nr: usize) -> Result<()> {
        let page = self.pages.get(&page_nr).unwrap();
        let path = format!("{}/template_{}.bin", self.path, page_nr);
        page.write_to_file(path)?;

        Ok(())
    }

    fn load_page(&mut self, page_nr: usize) -> Result<()> {
        let path = format!("{}/template_{}.bin", self.path, page_nr);
        let page = TemplatePage::read_from_file(path)?;
        self.pages.insert(page_nr, page);

        Ok(())
    }
}

impl TemplatePage {
    fn new(size: usize) -> TemplatePage {
        TemplatePage { 
            nodes: vec![TemplateNode::default(); size] 
        }
    }
}


impl TemplateNode {
    pub fn new(ptr: u64, branches: [bool; 8], materials: [u8; 8]) -> TemplateNode {
        TemplateNode { ptr, branches, materials }
    }

    pub fn get_ptr(&self) -> u64 {
        self.ptr 
    }

    pub fn get_branches(&self) -> [bool; 8] {
        self.branches 
    }

    pub fn get_num_branches(&self) -> usize {
        let mut num = 0;
        for branch in self.branches {
            if branch {
                num += 1;
            }
        }

        num
    }

    pub fn get_materials(&self) -> [u8; 8] {
        self.materials 
    }
}

