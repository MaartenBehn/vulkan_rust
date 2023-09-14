use crate::node::{bits_to_bools, bools_to_bits, new_node, Node};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub const PAGE_BITS: usize = 8;
pub const PAGE_SIZE: usize = 256;

pub struct Octtree {
    pub pages: Vec<[Node; PAGE_SIZE]>,
}

impl Octtree {
    pub fn new() -> Octtree {
        let mut tree = Octtree { pages: Vec::new() };

        let mut rng = StdRng::seed_from_u64(0);
        tree.fill(0, 0, 1, &mut rng);

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
            branch[i] = rand_float < 0.8 && depth < 5;
            mats[i] = depth as u8;

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
