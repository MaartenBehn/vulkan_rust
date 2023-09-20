use app::glam::{ivec3, IVec3};
use indicatif::ProgressBar;
use octtree_v2::node::CHILD_CONFIG;
use octtree_v2::template::{TemplateNode, TemplateTree};
use octtree_v2::util::create_dir;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use app::anyhow::Result;

pub fn build_template_tree(path: &str, depth: usize, page_size: usize) -> Result<()> {
    create_dir(path)?;

    let mut tree = TemplateTree::new(path.to_owned(), page_size);

    let mut rng = StdRng::seed_from_u64(0);
    let bar = ProgressBar::new(u32::MAX as u64);
    fill(&mut tree, 0, depth, 0, 1, ivec3(0, 0, 0), &mut rng, &bar)?;

    tree.save_all_pages()?;
    tree.save_metadata()?;

    Ok(())
}

fn fill(
    tree: &mut TemplateTree,
    depth: usize,
    tree_depth: usize,
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

        mats[i] = rng.gen::<u16>().max(1) * (rand_float < 0.8) as u16;
        branch[i] = rand_float < 0.8 && depth < (tree_depth - 1);
        if branch[i] {
            num_branches += 1;
        }
    }

    let use_ptr = ptr as u64 * (num_branches != 0) as u64;
    tree.set_node(index, TemplateNode::new(use_ptr, branch, mats))?;

    let parent_ptr = ptr;
    if num_branches > 0 {
        ptr += num_branches;

        let child_size = i32::pow(2, (tree_depth - depth - 1) as u32);

        let mut i = 0;
        for (j, b) in branch.iter().enumerate() {
            if *b {
                let new_pos = ivec3(
                    pos[0] + CHILD_CONFIG[j][0] * child_size,
                    pos[1] + CHILD_CONFIG[j][1] * child_size,
                    pos[2] + CHILD_CONFIG[j][2] * child_size,
                );

                ptr = fill(
                    tree,
                    depth + 1,
                    tree_depth,
                    parent_ptr + i,
                    ptr,
                    new_pos,
                    rng,
                    bar,
                )?;

                i += 1;
            }
        }
    }

    Ok(ptr)
}
