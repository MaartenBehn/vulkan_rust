pub mod node;
pub mod tree;
pub mod util;
pub mod aabb;
pub mod template;
pub mod converter;
pub mod metadata;
pub mod builder;
pub mod reader;

use octa_force::anyhow::Result;

pub trait Tree {
    type Page;
    type Node;

    fn new(path: String, page_size: usize, depth: usize) -> Self;
    fn form_disk(path: String) -> Result<Self> where Self: Sized;

    fn add_empty_page(&mut self, page_nr: usize);
    fn add_page(&mut self, page_nr: usize, page: Self::Page);
    fn get_page(&mut self, page_nr: usize) -> &Self::Page;
    fn remove_page(&mut self, page_nr: usize);

    fn has_page(&self, page_nr: usize) -> bool;
    fn get_all_page_nrs(&self) -> Vec<usize>;
    
    fn set_node(&mut self, page_nr: usize, in_page_index: usize, node: Self::Node) -> Result<()>;
    fn get_node(&self, page_nr: usize, in_page_index: usize) -> Self::Node;

    fn get_depth(&self) -> usize;
    fn get_page_size(&self) -> usize;
    fn get_page_ammount(&self) ->usize;
    fn save_metadata(&self) -> Result<()>;

    fn save_page(&self, page_nr: usize) -> Result<()>;
    fn load_page(&mut self, page_nr: usize) -> Result<()>;
}

pub trait Page {
    fn new(page_size: usize) -> Self;
}