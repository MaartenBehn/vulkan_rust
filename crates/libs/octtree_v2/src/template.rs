
use std::{fs::{OpenOptions, self}, io::Write};

use app::anyhow::{Result, bail};
use speedy::{Readable, Writable};
use serde::{Serialize, Deserialize};

use crate::util::create_dir;

#[derive(Clone, Debug, Default)]
pub struct TemplateTreeBuilder {
    tree: TemplateTree,
}

#[derive(Clone, Debug, Default)]
pub struct TemplateTreeReader {
    tree: TemplateTree,
}

#[derive(Clone, Debug, Default)]
struct TemplateTree {
    path: String,
    metadata: TemplateMetadata,
    pages: Vec<TemplatePage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TemplateMetadata {
    page_size: usize,
    last_page: usize,
    depth: usize,
}

#[derive(Clone, Debug, Default, Readable, Writable)]
struct TemplatePage {
    nr: usize,
    set_counter: usize,
    nodes: Vec<TemplateNode>,
}

#[derive(Clone, Copy, Debug, Default, Readable, Writable)]
pub struct TemplateNode {
    ptr: u64,
    branches: [bool; 8],
    materials: [u8; 8],
}

impl TemplateTreeBuilder {
    pub fn new(path: String, page_size: usize, depth: usize) -> Result<TemplateTreeBuilder> {
        let tree = TemplateTree { 
            path,
            metadata: TemplateMetadata::new(page_size, depth),
            pages: Vec::new(),
        };
        create_dir(&tree.path)?;

        Ok(TemplateTreeBuilder { tree })
    }

    pub fn set_node(&mut self, index: usize, ptr: u64, branches: [bool; 8], materials: [u8; 8]) -> Result<()> {
        let page_size = self.tree.metadata.page_size;
        let page_nr = index / page_size;
        let in_page_index = index % page_size;

        let res = self.tree.get_page_index(page_nr);
        let page_index = if res.is_err() {
            if page_nr < self.tree.metadata.last_page {
                bail!("Unloaded page needed!")
            }

            for i in self.tree.metadata.last_page..(page_nr + 1) {
                self.tree.pages.push(TemplatePage::new(i, self.tree.metadata.page_size));
            }
            self.tree.metadata.last_page = page_nr;
            
            self.tree.pages.len() - 1
        }
        else {
            res.unwrap()
        };
        
        self.tree.pages[page_index].set_node(in_page_index, TemplateNode::new(ptr, branches, materials));
        self.tree.check_page_save(page_index)?;

        Ok(())
    }

    pub fn done(&mut self) -> Result<()> {
        self.tree.save_all_pages()?;
        self.tree.save_metadata()?;

        Ok(())
    }

    pub fn get_depth(&self) -> usize {
        self.tree.metadata.depth
    }
}

impl TemplateTreeReader {
    pub fn new(path: String) -> Result<TemplateTreeReader> {
        let mut tree = TemplateTree { 
            path,
            metadata: TemplateMetadata::default(),
            pages: Vec::new(),
        };
        tree.load_metadata()?;

        Ok(TemplateTreeReader { tree })
    }

    pub fn get_node(&mut self, index: usize) -> Result<TemplateNode> {
        let page_size = self.tree.metadata.page_size;
        let page_nr = index / page_size;
        let in_page_index = index % page_size;

        let res = self.tree.get_page_index(page_nr);
        let page_index = if res.is_err() {
            self.tree.load_page(page_nr)?            
        }
        else {
            res.unwrap()
        };

        let node = self.tree.pages[page_index].nodes[in_page_index];
        Ok(node)
    }

    pub fn get_depth(&self) -> usize {
        self.tree.metadata.depth
    }
}

impl TemplateTree {
    fn get_page_index(&self, nr: usize) -> Result<usize> {
        for (i, page) in self.pages.iter().enumerate() {
            if page.nr == nr{
                return Ok(i);
            }
        }

        bail!("Page not found!")
    }

    fn save_page(&mut self, page_index: usize) -> Result<()> {
        let page = &self.pages[page_index];
        let path = format!("{}/template_{}.bin", self.path, page.nr);
        page.write_to_file(path)?;

        self.pages.swap_remove(page_index);

        Ok(())
    }

    fn load_page(&mut self, page_nr: usize) -> Result<usize> {
        let path = format!("{}/template_{}.bin", self.path, page_nr);
        let page = TemplatePage::read_from_file(path)?;
        self.pages.push(page);

        Ok(self.pages.len() -1)
    }

    fn check_page_save(&mut self, page_index: usize) -> Result<()>{
        if self.pages[page_index].set_counter >= self.metadata.page_size - 1 {
            self.save_page(page_index)?;
        }

        Ok(())
    }

    fn save_all_pages(&mut self) -> Result<()> {
        let len = self.pages.len();
        for i in (0..len).rev() {
            self.save_page(i)?;
        }

        self.pages.clear();

        Ok(())
    }

    fn save_metadata(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.metadata)?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("{}/metadata.json", self.path))
            .unwrap();

        file.write(json.as_bytes())?;

        Ok(())
    }

    fn load_metadata(&mut self) -> Result<()> {
        let json = fs::read_to_string(format!("{}/metadata.json", self.path))?;
        self.metadata = serde_json::from_str(&json)?;

        Ok(())
    }

   
}

impl TemplateMetadata {
    pub fn new(page_size: usize, depth: usize) -> TemplateMetadata {
        TemplateMetadata { 
            page_size, 
            last_page: 0,
            depth,
        }
    }
}

impl TemplatePage {
    fn new(nr: usize, size: usize) -> TemplatePage {
        TemplatePage { 
            nr, 
            set_counter: 0,
            nodes: vec![TemplateNode::default(); size] 
        }
    }

    fn set_node(&mut self, index: usize, node: TemplateNode) {
        self.nodes[index] = node;
        self.set_counter += 1;
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
            num += 1;
        }

        num
    }

    pub fn get_materials(&self) -> [u8; 8] {
        self.materials 
    }
}

