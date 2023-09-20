use serde::{Serialize, Deserialize};

use app::anyhow::Result;

use crate::{node::Node, template::TemplateTreeReader};

#[derive(Clone, Debug, Default)]
pub struct Tree {
    path: String,
    metadata: Metadata,
    pages: Vec<Page>,
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

impl Tree {
    pub fn from_template(path: String, page_size: usize, reader: &mut TemplateTreeReader) -> Result<Tree> {
        let mut tree = Tree{
            path,
            metadata: Metadata { page_size, page_ammount: 0, depth: reader.get_depth() },
            pages: Vec::new(),
        };

        tree.convert_node(reader, 0, 0)?;

        Ok(tree)
    }

    fn convert_node(&mut self, reader: &mut TemplateTreeReader, template_index: usize, depth: usize) -> Result<()> {
        let template_node = reader.get_node(template_index)?;
        let num_branches = template_node.get_num_branches();



    } 

    
}