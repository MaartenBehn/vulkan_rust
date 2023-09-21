use app::anyhow::Result;
use indicatif::ProgressBar;

use crate::{template::{TemplateTreeReader, TemplateNode}, tree::TreeBuilder, node::{bools_to_bits, Node, MAX_PTR}};

const FAR_PADDING: usize = 255;

struct Converter {
    reader: TemplateTreeReader, 
    builder: TreeBuilder,
}

pub fn convert_template_to_tree(reader: TemplateTreeReader, builder: TreeBuilder) -> Result<()> {
    let mut converter = Converter{
        reader,
        builder,
    };

    let root_node = converter.reader.get_node(0).unwrap();
    let bar = ProgressBar::new(u32::MAX as u64);
    converter.convert_node(root_node, 0, 0, 1, 0, &bar)?;

    Ok(())
}

impl Converter {
    fn convert_node(&mut self, template_node: TemplateNode, depth: usize, index: usize, mut ptr: usize, far_offset: usize, bar: &ProgressBar) -> Result<usize> {
        bar.set_position(ptr as u64);

        let branches = template_node.get_branches();
        let mats = template_node.get_materials();
    
        let use_ptr = if far_offset == 0 {
            ptr - index
        } else {
            self.builder.set_node(index + far_offset, Node::new_far_pointer(ptr - index - far_offset))?;
            far_offset
        };
        self.builder.set_node(index, Node::new(use_ptr, bools_to_bits(branches), mats, far_offset != 0))?;

        let template_ptr = template_node.get_ptr() as usize;
        let mut child_nodes = Vec::new();
        let mut child_fars = Vec::new();

        let mut num_childen = 0;
        let mut num_far = 0;
        for b in branches {
            if b {
                let child_index = template_ptr + num_childen;
                let child_node = self.reader.get_node(child_index).unwrap();
                let child_ptr = child_node.get_ptr() as usize;
                let child_far = if child_ptr != 0 {(child_ptr - child_index + FAR_PADDING) > MAX_PTR} else {false};

                if child_far {
                    num_far += 1;
                }

                child_nodes.push(child_node);
                child_fars.push(child_far);
                

                num_childen += 1;

            }
        }

        let parent_ptr = ptr;
        ptr += num_childen + num_far;
        for (i, child_node) in child_nodes.iter().enumerate() {
            let child_index = parent_ptr + i;   
            let child_far_offset = num_childen * child_fars[i] as usize;

            ptr = self.convert_node(*child_node, depth + 1, child_index, ptr, child_far_offset, bar)?;
        }

        Ok(ptr)
    } 
}
