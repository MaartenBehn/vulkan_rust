use std::{collections::HashMap, fs::{File, OpenOptions, self}, io::{Write, Read}};

use serde::{Serialize, Deserialize};

use app::anyhow::{Result, bail};

use crate::{node::Node, util::{self, create_dir}};

pub struct TreeBuilder {
    tree: Tree,
    set_counters: HashMap<usize, usize>
}

#[derive(Clone, Debug, Default)]
pub struct Tree {
    path: String,
    metadata: Metadata,
    pages: HashMap<usize, Page>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    page_size: usize,
    page_ammount: usize,
    depth: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Page {
    nodes: Vec<Node>,
}

impl TreeBuilder {
    pub fn new(path: String, page_size: usize) -> Result<TreeBuilder> {
        create_dir(&path)?;

        Ok(TreeBuilder {
            tree: Tree {
                path,
                metadata: Metadata { page_size, page_ammount: 0, depth: 0 },
                pages: HashMap::new(),
            },
            set_counters: HashMap::new(),
        })
    }

    pub fn set_node(&mut self, index: usize, node: Node) -> Result<()> {
        let page_size = self.tree.metadata.page_size;
        let page_nr = index / page_size;
        let in_page_index = index % page_size;

        if !self.tree.pages.contains_key(&page_nr) {
            if page_nr < self.tree.metadata.page_ammount {
                bail!("Unloaded page needed!");
            }

            for i in self.tree.metadata.page_ammount..(page_nr + 1) {
                self.tree.pages.insert(i, Page::new(page_size));
                self.set_counters.insert(i, 0);
            }
            self.tree.metadata.page_ammount = page_nr + 1;
        }
        self.tree.pages.get_mut(&page_nr).unwrap().nodes[in_page_index] = node;
        *self.set_counters.get_mut(&page_nr).unwrap() += 1;

        self.check_page_save(page_nr)?;

        Ok(())
    }

    fn check_page_save(&mut self, page_nr: usize) -> Result<()>{
        if self.set_counters[&page_nr] >= self.tree.metadata.page_size {
            self.tree.save_page(page_nr)?;
        }

        Ok(())
    }

    fn done(&mut self) -> Result<()> {
        self.tree.save_all_pages()?;
        self.tree.save_metadata()?;

        Ok(())
    }
}

impl Tree {
     fn save_page(&mut self, page_nr: usize) -> Result<()> {
        let page = self.pages.get(&page_nr).unwrap();
        
        let mut file = File::create(format!("{}/page_{}.bin", self.path, page_nr))?;
        let buffer = unsafe { util::slice_as_u8_slice(&page.nodes) };
        file.write_all(buffer)?;

        self.pages.remove(&page_nr);

        Ok(())
    }

    fn load_page(&mut self, page_nr: usize) -> Result<()> {
        let mut page = Page::new(self.metadata.page_size);

        let mut file = File::create(format!("{}/page_{}.bin", self.path, page_nr))?;
        let buffer = unsafe { util::slice_as_u8_slice_mut(&mut page.nodes) };
        file.read(buffer)?;

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


impl Page {
    fn new(page_size: usize) -> Page {
        Page { nodes: vec![Node::default(); page_size] }
    }
}



