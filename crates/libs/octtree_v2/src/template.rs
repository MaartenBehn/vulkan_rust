
use std::{fs::OpenOptions, io::Write};

use app::anyhow::{Result, bail};
use speedy::{Readable, Writable};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Default)]
pub struct TemplateTree {
    path: String,
    metadata: TemplateMetadata,
    pages: Vec<TemplatePage>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TemplateMetadata {
    page_size: usize,
    page_ammount: usize,
}

#[derive(Clone, Debug, Default, Readable, Writable)]
pub struct TemplatePage {
    nr: usize,
    set_counter: usize,
    nodes: Vec<TemplateNode>,
}

#[derive(Clone, Copy, Debug, Default, Readable, Writable)]
pub struct TemplateNode {
    ptr: u64,
    branches: [bool; 8],
    materials: [u16; 8],
}

impl TemplateTree {
    pub fn new(path: String, page_size: usize) -> TemplateTree {
        TemplateTree { 
            path,
            metadata: TemplateMetadata::new(page_size),
            pages: Vec::new(),
        }
    }

    pub fn set_node(&mut self, index: usize, node: TemplateNode) -> Result<()> {
        let page_size = self.metadata.page_size;
        let page_nr = index / page_size;
        let in_page_index = index % page_size;

        let res = self.get_page_index(page_nr);
        let page_index = if res.is_err() {
            if page_nr < self.metadata.page_ammount {
                bail!("Unloaded page needed!")
            }

            for i in self.metadata.page_ammount..(page_nr + 1) {
                self.pages.push(TemplatePage::new(i, self.metadata.page_size));
            }
            self.metadata.page_ammount = page_nr + 1;
            
            self.pages.len() - 1
        }
        else {
            res.unwrap()
        };
        
        self.pages[page_index].set_node(in_page_index, node);
        self.check_page_save(page_index)?;

        Ok(())
    }

    pub fn get_page_index(&self, nr: usize) -> Result<usize> {
        for (i, page) in self.pages.iter().enumerate() {
            if page.nr == nr{
                return Ok(i);
            }
        }

        bail!("Page not found!")
    }

    pub fn check_page_save(&mut self, page_index: usize) -> Result<()>{
        if self.pages[page_index].set_counter >= self.metadata.page_size {
            self.save_page(page_index)?;
        }

        Ok(())
    }

    pub fn save_page(&mut self, page_index: usize) -> Result<()> {
        let page = &self.pages[page_index];
        let path = format!("{}/template_{}.bin", self.path, page.nr);
        page.write_to_file(path)?;

        self.pages.swap_remove(page_index);

        Ok(())
    }

    pub fn save_all_pages(&mut self) -> Result<()> {
        let len = self.pages.len();
        for i in (0..len).rev() {
            self.save_page(i)?;
        }

        self.pages.clear();

        Ok(())
    }

    pub fn save_metadata(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.metadata)?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("{}/metadata.json", self.path))
            .unwrap();

        file.write(json.as_bytes())?;

        Ok(())
    }
}

impl TemplateMetadata {
    pub fn new(page_size: usize) -> TemplateMetadata {
        TemplateMetadata { 
            page_size, 
            page_ammount: 0 
        }
    }
}

impl TemplatePage {
    pub fn new(nr: usize, size: usize) -> TemplatePage {
        TemplatePage { 
            nr, 
            set_counter: 0,
            nodes: vec![TemplateNode::default(); size] 
        }
    }

    pub fn set_node(&mut self, index: usize, node: TemplateNode) {
        self.nodes[index] = node;
        self.set_counter += 1;
    }
}


impl TemplateNode {
    pub fn new(ptr: u64, branches: [bool; 8], materials: [u16; 8]) -> TemplateNode {
        TemplateNode { ptr, branches, materials }
    }
}

