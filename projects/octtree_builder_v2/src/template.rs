use indicatif::ProgressBar;
use octtree_v2::{
    builder::Builder,
    template::{TemplateNode, TemplateTree},
};

use app::anyhow::Result;

pub fn build_template_tree(path: &str, depth: usize, page_size: usize) -> Result<()> {
    let mut builder = Builder::new(path.to_owned(), page_size, depth)?;

    let mut rng = fastrand::Rng::with_seed(42);
    let bar = ProgressBar::new(u32::MAX as u64);
    fill(&mut builder, 0, 0, 1, &mut rng, &bar)?;
    builder.done()?;

    Ok(())
}

fn fill(
    builder: &mut Builder<TemplateTree>,
    depth: usize,
    index: usize,
    mut ptr: usize,
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
    builder.set_node(index, TemplateNode::new(use_ptr as u64, branch, mats))?;

    let parent_ptr = ptr;
    if num_branches > 0 {
        ptr += num_branches;

        let mut i = 0;
        for b in branch {
            if b {
                ptr = fill(builder, depth + 1, parent_ptr + i, ptr, rng, bar)?;

                i += 1;
            }
        }
    }

    Ok(ptr)
}
