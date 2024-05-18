use std::{collections::VecDeque, mem::size_of};

use octa_force::{anyhow::Result, log};

use crate::Tree;

#[derive(Clone, Debug, Default)]
pub struct Reader<T: Tree> {
    max_loaded_pages: usize,
    page_list: VecDeque<usize>,
    pub tree: T,
}

impl<T: Tree> Reader<T> {
    pub fn new(path: String, max_loaded_pages: usize) -> Result<Reader<T>> {
        let tree = T::form_disk(path)?;

        let possible_size =
            (size_of::<T::Node>() * tree.get_page_size() + size_of::<T::Page>()) * max_loaded_pages;
        log::info!(
            "Reader Max Size: {} byte {} MB {} GB",
            possible_size,
            possible_size as f32 / 1000000.0,
            possible_size as f32 / 1000000000.0
        );

        Ok(Reader {
            max_loaded_pages,
            page_list: VecDeque::new(),
            tree,
        })
    }

    pub fn get_page(&mut self, page_nr: usize) -> Result<&T::Page> {
        self.check_page(page_nr)?;
        Ok(self.tree.get_page(page_nr))
    }

    pub fn get_node(&mut self, index: usize) -> Result<T::Node> {
        let page_size = self.tree.get_page_size();
        let page_nr = index / page_size;
        let in_page_index = index % page_size;

        self.check_page(page_nr)?;

        Ok(self.tree.get_node(page_nr, in_page_index))
    }

    fn check_page(&mut self, page_nr: usize) -> Result<()> {
        if !self.tree.has_page(page_nr) {
            self.tree.load_page(page_nr)?;
        } else {
            let mut index = 0;
            for (i, nr) in self.page_list.iter().enumerate() {
                if page_nr == *nr {
                    index = i;
                }
            }
            self.page_list.remove(index);
        }

        self.page_list.push_front(page_nr);

        self.check_clean();

        Ok(())
    }

    pub fn check_clean(&mut self) {
        while self.page_list.len() >= self.max_loaded_pages {
            let page_nr = self.page_list.pop_back().unwrap();
            self.tree.remove_page(page_nr);
        }
    }

    pub fn get_depth(&self) -> usize {
        self.tree.get_depth()
    }
}
