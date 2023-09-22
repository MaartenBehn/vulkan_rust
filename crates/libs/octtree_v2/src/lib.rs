pub mod node;
pub mod tree;
pub mod util;
pub mod aabb;
pub mod template;
pub mod converter;
pub mod metadata;
pub mod builder;
pub mod reader;

use app::anyhow::{Result, Error, bail};
use metadata::Metadata;
use node::CompressedNode;
use template::TemplateNode;

pub trait Tree {
    fn new(path: String, page_size: usize, depth: usize) -> Self;
    fn form_disk(path: String) -> Result<Self> where Self: Sized;

    fn add_empty_page(&mut self, page_nr: usize);
    fn has_page(&self, page_nr: usize) -> bool;
    fn get_all_page_nrs(&self) -> Vec<usize>;
    fn remove_page(&mut self, page_nr: usize);
    
    fn set_node(&mut self, page_nr: usize, in_page_index: usize, node: Node) -> Result<()>;
    fn get_node(&self, page_nr: usize, in_page_index: usize) -> Node;

    fn get_metadata(&self) -> Metadata;
    fn save_metadata(&self) -> Result<()>;

    fn save_page(&self, page_nr: usize) -> Result<()>;
    fn load_page(&mut self, page_nr: usize) -> Result<()>;
}

pub enum Node {
    Compressed(CompressedNode),
    Template(TemplateNode)
}

impl TryInto<CompressedNode> for Node {
    type Error = Error;

    fn try_into(self) -> std::result::Result<CompressedNode, Self::Error> {
        if let Node::Compressed(n) = self { Ok(n) } else { bail!("Wrong Node Type!") }
    }
}

impl TryInto<TemplateNode> for Node {
    type Error = Error;

    fn try_into(self) -> std::result::Result<TemplateNode, Self::Error> {
        if let Node::Template(n) = self { Ok(n) } else { bail!("Wrong Node Type!") }
    }
}