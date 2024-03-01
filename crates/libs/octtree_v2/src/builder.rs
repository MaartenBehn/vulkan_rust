use std::collections::HashMap;

use octa_force::anyhow::{Result, bail};

use crate::{Tree, util::create_dir};

pub struct Builder<T: Tree> {
    pub tree: T,
    pub set_counters: HashMap<usize, usize>,
}

impl<T: Tree> Builder<T> {
    pub fn new(path: String, page_size: usize, depth: usize) -> Result<Builder<T>> {
        create_dir(&path)?;
        let tree = T::new(path, page_size, depth);

        Ok(Builder { 
            tree,
            set_counters: HashMap::new(),
        })
    }

    pub fn set_node(&mut self, index: usize, node: T::Node) -> Result<()> {
        let page_nr = index / self.tree.get_page_size();
        let in_page_index = index % self.tree.get_page_size();

        if !self.tree.has_page(page_nr) {
            if page_nr < self.tree.get_page_ammount() {
                bail!("Unloaded page needed!");
            }

            for i in self.tree.get_page_ammount()..(page_nr + 1) {
                self.tree.add_empty_page(i);
                self.set_counters.insert(i, 0);
            }
        }
        self.tree.set_node(page_nr, in_page_index, node)?;
        *self.set_counters.get_mut(&page_nr).unwrap() += 1;

        self.check_page_save(page_nr)?;

        Ok(())
    }

    pub fn check_page_save(&mut self, page_nr: usize) -> Result<()>{
        if self.set_counters[&page_nr] >= self.tree.get_page_size() {
            self.tree.save_page(page_nr)?;
            self.tree.remove_page(page_nr);
            self.set_counters.remove(&page_nr);
        }

        Ok(())
    }

    pub fn done(&mut self) -> Result<()> {
        for page_nr in self.tree.get_all_page_nrs() {
            self.tree.save_page(page_nr)?;
            self.tree.remove_page(page_nr);
        }

        self.tree.save_metadata()?;

        Ok(())
    }

    pub fn get_depth(&self) -> usize {
        self.tree.get_depth()
    }
}
