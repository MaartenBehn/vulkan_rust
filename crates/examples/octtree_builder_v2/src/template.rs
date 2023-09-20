use app::glam::{ivec3, IVec3};
use indicatif::ProgressBar;
use octtree_v2::node::CHILD_CONFIG;
use octtree_v2::template::TemplateTreeBuilder;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use app::anyhow::Result;

pub fn build_template_tree(path: &str, depth: usize, page_size: usize) -> Result<()> {
    let mut builder = TemplateTreeBuilder::new(path.to_owned(), page_size, depth)?;

    let mut rng = StdRng::seed_from_u64(0);
    let bar = ProgressBar::new(u32::MAX as u64);
    fill(&mut builder, 0, 0, 1, ivec3(0, 0, 0), &mut rng, &bar)?;
    builder.done()?;

    Ok(())
}

fn fill(
    builder: &mut TemplateTreeBuilder,
    depth: usize,
    index: usize,
    mut ptr: usize,
    pos: IVec3,
    rng: &mut StdRng,
    bar: &ProgressBar,
) -> Result<usize> {
    bar.set_position(ptr as u64);

    let mut branch = [false; 8];
    let mut mats = [0; 8];
    let mut num_branches = 0;
    for i in 0..8 {
        let rand_float: f32 = rng.gen();

        mats[i] = rng.gen::<u8>().max(1) * (rand_float < 0.8) as u8;
        branch[i] = rand_float < 0.8 && depth < (builder.get_depth() - 1);
        if branch[i] {
            num_branches += 1;
        }
    }

    let use_ptr = ptr as u64 * (num_branches != 0) as u64;
    builder.set_node(index, use_ptr, branch, mats)?;

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
