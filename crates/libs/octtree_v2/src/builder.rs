use std::collections::HashMap;

use app::anyhow::{Result, bail};

use crate::{Tree, util::create_dir, Node};

pub struct Builder<T: Tree> {
    tree: T,
    set_counters: HashMap<usize, usize>,
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

    pub fn set_node(&mut self, index: usize, node: Node) -> Result<()> {
        let metadata = self.tree.get_metadata();

        let page_nr = index / metadata.page_size;
        let in_page_index = index % metadata.page_size;

        if !self.tree.has_page(page_nr) {
            if page_nr < metadata.page_ammount {
                bail!("Unloaded page needed!");
            }

            for i in metadata.page_ammount..(page_nr + 1) {
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
        if self.set_counters[&page_nr] >= self.tree.get_metadata().page_size {
            self.tree.save_page(page_nr)?;
            self.tree.remove_page(page_nr);
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
        self.tree.get_metadata().depth
    }
}
