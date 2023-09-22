use app::glam::{ivec3, IVec3};
use indicatif::ProgressBar;
use octtree_v2::{
    builder::Builder,
    node::CHILD_CONFIG,
    template::{TemplateNode, TemplateTree},
    Node,
};

use app::anyhow::Result;

pub fn build_template_tree(path: &str, depth: usize, page_size: usize) -> Result<()> {
    let mut builder = Builder::new(path.to_owned(), page_size, depth)?;

    let mut rng = fastrand::Rng::with_seed(42);
    let bar = ProgressBar::new(u32::MAX as u64);
    fill(&mut builder, 0, 0, 1, ivec3(0, 0, 0), &mut rng, &bar)?;
    builder.done()?;

    Ok(())
}

fn fill(
    builder: &mut Builder<TemplateTree>,
    depth: usize,
    index: usize,
    mut ptr: usize,
    pos: IVec3,
    rng: &mut fastrand::Rng,
    bar: &ProgressBar,
) -> Result<usize> {
    bar.set_position(ptr as u64);

    let mut branch = [false; 8];
    let mut mats = [0; 8];
    let mut num_branches = 0;
    for i in 0..8 {
        let rand_float = rng.f32();

        mats[i] = rng.u8(1..u8::MAX) * (rand_float < 0.8) as u8;
        branch[i] = rand_float < 0.8 && depth < (builder.get_depth() - 1);
        if branch[i] {
            num_branches += 1;
        }
    }

    let use_ptr = ptr * (num_branches != 0) as usize;
    builder.set_node(
        index,
        Node::Template(TemplateNode::new(use_ptr as u64, branch, mats)),
    )?;

    let parent_ptr = ptr;
    if num_branches > 0 {
        ptr += num_branches;

        let child_size = i32::pow(2, (builder.get_depth() - depth - 1) as u32);

        let mut i = 0;
        for (j, b) in branch.iter().enumerate() {
            if *b {
                let new_pos = ivec3(
                    pos[0] + CHILD_CONFIG[j][0] * child_size,
                    pos[1] + CHILD_CONFIG[j][1] * child_size,
                    pos[2] + CHILD_CONFIG[j][2] * child_size,
                );

                ptr = fill(builder, depth + 1, parent_ptr + i, ptr, new_pos, rng, bar)?;

                i += 1;
            }
        }
    }

    Ok(ptr)
}
