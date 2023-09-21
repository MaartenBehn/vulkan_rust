use app::anyhow::Result;

use crate::{Tree, util::create_dir, Node};

struct Builder<T: Tree> {
    tree: T
}

impl<T: Tree> Builder<T> {
    pub fn new(path: String, page_size: usize, depth: usize) -> Result<Builder<T>> {
        let tree = T::new(path, page_size, depth);
        create_dir(&path)?;

        Ok(Builder { tree })
    }

    pub fn set_node(&mut self, index: usize, ) -> Result<()> {
        let page_size = self.tree.get_page_size();
        let page_nr = index / page_size;
        let in_page_index = index % page_size;

        if self.tree.pages.contains_key(&page_nr) {
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

        Ok(())
    }
}
