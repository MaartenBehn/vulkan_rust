use std::mem::size_of;

use crate::node::{bits_to_bools, bools_to_bits, new_node, Node};
use app::log;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub const PAGE_BITS: usize = 8;
pub const PAGE_SIZE: usize = 256;
pub const TREE_DEPTH: usize = 8;

pub struct Octtree {
    pub pages: Vec<[Node; PAGE_SIZE]>,
}

impl Octtree {
    pub fn new() -> Octtree {
        let mut tree = Octtree { pages: Vec::new() };

        let mut rng = StdRng::seed_from_u64(0);
        tree.fill(0, 0, 1, &mut rng);

        log::info!("Tree has {} pages.", tree.pages.len());
        let tree_size = (size_of::<Node>() * PAGE_SIZE * tree.pages.len());
        log::info!(
            "Size: {} MB {} GB",
            tree_size as f32 / 1000000.0,
            tree_size as f32 / 1000000000.0
        );

        tree
    }

    fn fill(&mut self, depth: usize, index: usize, mut ptr: usize, rng: &mut StdRng) -> usize {
        let page_index = index / PAGE_SIZE;
        let in_page_index = index % PAGE_SIZE;
        while page_index >= self.pages.len() {
            self.pages.push([Node::default(); PAGE_SIZE])
        }

        let mut branch = [false; 8];
        let mut mats = [0; 8];
        let mut num_branches = 0;
        for i in 0..8 {
            let rand_float: f32 = rng.gen();

            mats[i] = (depth as u8 + 1) * (rand_float < 0.5) as u8;
            // mats[i] = (depth as u8 + 1) * (i == 0) as u8;
            branch[i] = rand_float < 0.5 && depth < (TREE_DEPTH - 1);
            // branch[i] = i == 0 && depth < (TREE_DEPTH - 1);
            if branch[i] {
                num_branches += 1;
            }
        }

        let branch_bits = bools_to_bits(branch);
        let use_ptr = if branch_bits != 0 { ptr } else { 0 };
        self.pages[page_index][in_page_index] = new_node(use_ptr, branch_bits, mats);

        if num_branches > 0 {
            ptr += num_branches;

            let mut i = 0;
            for b in branch {
                if b {
                    ptr = self.fill(depth + 1, use_ptr + i, ptr, rng);

                    i += 1;
                }
            }
        }

        ptr
    }
}

mod tests {
    use crate::node::{bits_to_bools, get_branches, get_ptr};

    use super::{Octtree, PAGE_SIZE};

    #[test]
    fn tree_creation() {
        let tree = Octtree::new();

        for page in tree.pages {
            for i in 0..PAGE_SIZE {
                let node = page[i];
                println!(
                    "{} Ptr: {}, Branch: {:?} Depth: {}",
                    i,
                    get_ptr(node),
                    bits_to_bools(get_branches(node)),
                    (node).mats[0],
                );
            }
        }
    }
}
