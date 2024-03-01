use std::collections::HashMap;

use octa_force::anyhow::Result;
use speedy::{Readable, Writable};

use crate::{metadata::Metadata, Tree, Page};

#[derive(Clone, Debug, Default)]
pub struct TemplateTree {
    path: String,
    metadata: Metadata,
    pages: HashMap<usize, TemplatePage>,
}

#[derive(Clone, Debug, Default, Readable, Writable)]
pub struct TemplatePage {
    pub nodes: Vec<TemplateNode>,
}

#[derive(Clone, Copy, Debug, Default, Readable, Writable)]
pub struct TemplateNode {
    ptr: u64,
    branches: [bool; 8],
    materials: [u8; 8],
}

impl Tree for TemplateTree {
    type Page = TemplatePage;
    type Node = TemplateNode;

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

    fn remove_page(&mut self, page_nr: usize) {
        self.pages.remove(&page_nr);
    }

    fn set_node(&mut self, page_nr: usize, in_page_index: usize, node: Self::Node) -> Result<()> {
        self.pages.get_mut(&page_nr).unwrap().nodes[in_page_index] = node.try_into()?;  

        Ok(())
    }

    fn get_node(&self, page_nr: usize, in_page_index: usize) -> Self::Node {
        self.pages.get(&page_nr).unwrap().nodes[in_page_index]
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

    fn add_page(&mut self, page_nr: usize, page: Self::Page) {
        self.pages.insert(page_nr, page);
        self.metadata.page_ammount += 1;
    }

    fn get_page(&mut self, page_nr: usize) -> &Self::Page {
        self.pages.get(&page_nr).unwrap()
    }

    fn add_empty_page(&mut self, page_nr: usize) {
        self.add_page(page_nr, Page::new(self.metadata.page_size))
    }

    fn get_depth(&self) -> usize {
        self.metadata.depth
    }

    fn get_page_size(&self) -> usize {
        self.metadata.page_size
    }

    fn get_page_ammount(&self) ->usize {
        self.metadata.page_ammount
    }
}

impl Page for TemplatePage {
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

