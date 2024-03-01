use std::{collections::HashMap, io::{Write, Read}, fs::File};

use octa_force::{anyhow::Result, glam::IVec3};

use crate::{node::CompressedNode, util, metadata::Metadata, Tree, Page, aabb::AABB};

#[derive(Clone, Debug, Default)]
pub struct CompressedTree {
    path: String,
    metadata: Metadata,
    pages: HashMap<usize, CompressedPage>,
}

#[derive(Clone, Debug, Default)]
pub struct CompressedPage {
    pub nodes: Vec<CompressedNode>,
} 

impl Tree for CompressedTree {
    type Page = CompressedPage;
    type Node = CompressedNode;

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
        
        let mut file = File::create(format!("{}/page_{}.bin", self.path, page_nr))?;
        let buffer = unsafe { util::slice_as_u8_slice(&page.nodes) };
        file.write_all(buffer)?;

        Ok(())
    }

    fn load_page(&mut self, page_nr: usize) -> Result<()> {
        let mut page = CompressedPage::new(self.metadata.page_size);

        let mut file = File::open(format!("{}/page_{}.bin", self.path, page_nr))?;
        let buffer = unsafe { util::slice_as_u8_slice_mut(&mut page.nodes) };
        file.read(buffer)?;

        self.pages.insert(page_nr, page);

        Ok(())
    }

    fn add_page(&mut self, page_nr: usize, page: Self::Page) {
        self.pages.insert(page_nr, page);
        self.metadata.page_ammount += 1;
        
        for _ in self.metadata.aabbs.len()..(page_nr + 1) {
            self.metadata.aabbs.push(AABB::default());
        }
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

impl CompressedTree {
    pub fn get_aabbs(&self) -> Vec<AABB> {
        self.metadata.aabbs.clone()
    }

    pub fn get_aabb(&self, index: usize) -> AABB {
        let page_nr = index / self.get_page_size();
        self.metadata.aabbs[page_nr]
    }

    pub fn add_aabb(&mut self, index: usize, aabb: AABB) {
        let page_nr = index / self.get_page_size();

        let mut a = self.metadata.aabbs[page_nr];
        if a.min == IVec3::ZERO && a.max == IVec3::ZERO {
            a = aabb;
        } else {
            a.min = a.min.min(aabb.min);
            a.max = a.max.max(aabb.max);
        }
        self.metadata.aabbs[page_nr] = a;
    }
}

impl Page for CompressedPage {
    fn new(page_size: usize) -> CompressedPage {
        CompressedPage { nodes: vec![CompressedNode::default(); page_size] }
    }
}
