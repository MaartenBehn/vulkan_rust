use indicatif::ProgressBar;
use app::{
    anyhow::Result,
    glam::{ivec3, IVec3},
};

use crate::{
    aabb::AABB,
    builder::Builder,
    node::{bools_to_bits, CompressedNode, CHILD_CONFIG, MAX_PTR},
    reader::Reader,
    template::{TemplateNode, TemplateTree},
    tree::CompressedTree,
};

const FAR_PADDING: usize = 255;

struct Converter {
    reader: Reader<TemplateTree>,
    builder: Builder<CompressedTree>,
}

pub fn convert_template_to_tree(
    reader: Reader<TemplateTree>,
    builder: Builder<CompressedTree>,
) -> Result<()> {
    let mut converter = Converter { reader, builder };

    let root_node = converter.reader.get_node(0).unwrap().try_into()?;
    let bar = ProgressBar::new(u32::MAX as u64);
    converter.convert_node(root_node, 0, 0, 1, 0, ivec3(0, 0, 0), &bar)?;

    converter.builder.done()?;

    Ok(())
}

impl Converter {
    fn convert_node(
        &mut self,
        template_node: TemplateNode,
        depth: usize,
        index: usize,
        mut ptr: usize,
        far_offset: usize,
        pos: IVec3,
        bar: &ProgressBar,
    ) -> Result<(usize, AABB)> {
        bar.set_position(ptr as u64);

        let branches = template_node.get_branches();
        let mats = template_node.get_materials();

        let pos_aabb = AABB::new(pos, pos + IVec3::ONE);

        let branch_bits = bools_to_bits(branches);
        let use_ptr = if far_offset == 0 && branch_bits != 0 {
            ptr - index
        } else if far_offset != 0 {
            self.builder.set_node(
                index + far_offset,
                CompressedNode::new_far_pointer(ptr - index - far_offset),
            )?;
            self.builder.tree.add_aabb(index + far_offset, pos_aabb);
            far_offset
        } else {
            0
        };

        self.builder.set_node(
            index,
            CompressedNode::new(use_ptr, bools_to_bits(branches), mats, far_offset != 0),
        )?;
        self.builder.tree.add_aabb(index, pos_aabb);

        let template_ptr = template_node.get_ptr() as usize;
        let mut child_nodes = Vec::new();
        let mut child_fars = Vec::new();

        let mut num_childen = 0;
        let mut num_far = 0;

        let child_size = i32::pow(2, (self.reader.get_depth() - depth - 1) as u32);
        let mut new_poses = Vec::new();

        for (j, b) in branches.iter().enumerate() {
            if *b {
                let child_index = template_ptr + num_childen;
                let child_node: TemplateNode =
                    self.reader.get_node(child_index).unwrap().try_into()?;
                let child_ptr = child_node.get_ptr() as usize;
                let child_far = if child_ptr != 0 {
                    (child_ptr - child_index + FAR_PADDING) > MAX_PTR
                } else {
                    false
                };

                if child_far {
                    num_far += 1;
                }

                child_nodes.push(child_node);
                child_fars.push(child_far);

                num_childen += 1;

                let new_pos = ivec3(
                    pos[0] + CHILD_CONFIG[j][0] * child_size,
                    pos[1] + CHILD_CONFIG[j][1] * child_size,
                    pos[2] + CHILD_CONFIG[j][2] * child_size,
                );
                new_poses.push(new_pos)
            }
        }

        let parent_ptr = ptr;
        ptr += num_childen + num_far;
        let mut far_counter = 0;
        for (i, child_node) in child_nodes.iter().enumerate() {
            let child_index = parent_ptr + i;
            let child_far_offset = if child_fars[i] {
                let offset = num_childen - i + far_counter;
                far_counter += 1;
                offset
            } else {
                0
            };
            let new_pos = new_poses[i];

            let (child_ptr, child_aabb) = self.convert_node(
                *child_node,
                depth + 1,
                child_index,
                ptr,
                child_far_offset,
                new_pos,
                bar,
            )?;
            ptr = child_ptr;
            self.builder.tree.add_aabb(index, child_aabb);
        }

        Ok((ptr, self.builder.tree.get_aabb(index)))
    }
}
