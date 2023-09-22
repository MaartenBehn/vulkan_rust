use app::anyhow::Result;

use crate::{Tree, Node};

#[derive(Clone, Debug, Default)]
pub struct Reader<T: Tree> {
    tree: T,
}

impl<T: Tree> Reader<T> {
    pub fn new(path: String) -> Result<Reader<T>> {
        let tree = T::form_disk(path)?;
        Ok(Reader { tree })
    }

    pub fn get_node(&mut self, index: usize) -> Result<Node> {
        let page_size = self.tree.get_metadata().page_size;
        let page_nr = index / page_size;
        let in_page_index = index % page_size;

        if !self.tree.has_page(page_nr) {
            self.tree.load_page(page_nr)?            
        }
        
        Ok(self.tree.get_node(page_nr, in_page_index))
    }

    pub fn get_depth(&self) -> usize {
        self.tree.get_metadata().depth
    }
}