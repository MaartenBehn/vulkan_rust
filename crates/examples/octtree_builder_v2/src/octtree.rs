use std::mem::size_of;

use app::anyhow::{bail, Result};
use app::glam::{ivec3, IVec3};
use app::log;
use indicatif::ProgressBar;
use octtree_v2::aabb::AABB;
use octtree_v2::node::{bools_to_bits, new_node, Node, CHILD_CONFIG};
use octtree_v2::save::Saver;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

// pub const PAGE_BITS: usize = 16;
//pub const PAGE_SIZE: usize = 65536;
pub const PAGE_SIZE: usize = 16;
pub const MAX_PTR_SIZE: u64 = 16777216;

pub struct Octtree {
    depth: usize,
    pages: Vec<Page>,
    page_nr_counter: usize,

    saver: Saver,
}

pub struct Page {
    nr: usize,
    nodes: [Node; PAGE_SIZE],
    write_counter: usize,
}

impl Octtree {
    pub fn build(save_path: &str, depth: usize) -> Result<Octtree> {
        let mut tree = Octtree {
            depth: depth,
            pages: Vec::new(),
            page_nr_counter: 0,
            saver: Saver::new(save_path.to_owned(), depth, PAGE_SIZE)?,
        };

        let mut rng = StdRng::seed_from_u64(0);
        let mut bar = ProgressBar::new(MAX_PTR_SIZE);
        tree.fill(0, 0, 1, 0, ivec3(0, 0, 0), &mut rng, &mut bar)?;
        tree.done()?;

        log::info!("Tree has {} pages.", tree.page_nr_counter);
        let tree_size = size_of::<Node>() * PAGE_SIZE * tree.page_nr_counter;
        log::info!(
            "Size: {} MB {} GB",
            tree_size as f32 / 1000000.0,
            tree_size as f32 / 1000000000.0
        );

        Ok(tree)
    }

    fn fill(
        &mut self,
        depth: usize,
        index: usize,
        mut ptr: usize,
        mut parent_ptr: usize,
        pos: IVec3,
        rng: &mut StdRng,
        bar: &ProgressBar,
    ) -> Result<(usize, IVec3, IVec3)> {
        bar.set_position(ptr as u64);

        let page_nr = index / PAGE_SIZE;
        let in_page_index = index % PAGE_SIZE;

        let mut branch = [false; 8];
        let mut mats = [0; 8];
        let mut num_branches = 0;
        for i in 0..8 {
            let rand_float: f32 = rng.gen();

            mats[i] = ((rng.gen::<u8>() / 2) + 1) * (rand_float < 0.8) as u8;
            branch[i] = rand_float < 0.8 && depth < (self.depth - 1);
            if branch[i] {
                num_branches += 1;
            }
        }

        let branch_bits = bools_to_bits(branch);
        let use_ptr = if branch_bits != 0 {
            ptr - parent_ptr
        } else {
            0
        };

        self.set_node(page_nr, in_page_index, new_node(use_ptr, branch_bits, mats))?;

        let mut min = self.saver.metadata.aabbs[page_nr].min;
        let mut max = self.saver.metadata.aabbs[page_nr].max;
        let pos_min = pos;
        let pos_max = pos + IVec3::ONE;
        if min == IVec3::ZERO && max == IVec3::ZERO {
            min = pos;
            max = pos_max;
        } else {
            min = min.min(pos_min);
            max = max.max(pos_max);
        }

        parent_ptr = ptr;
        if num_branches > 0 {
            ptr += num_branches;

            let child_size = i32::pow(2, (self.depth - depth - 1) as u32);

            let mut i = 0;
            for (j, b) in branch.iter().enumerate() {
                if *b {
                    let new_pos = ivec3(
                        pos[0] + CHILD_CONFIG[j][0] * child_size,
                        pos[1] + CHILD_CONFIG[j][1] * child_size,
                        pos[2] + CHILD_CONFIG[j][2] * child_size,
                    );

                    let (new_ptr, child_min, child_max) = self.fill(
                        depth + 1,
                        parent_ptr + i,
                        ptr,
                        parent_ptr,
                        new_pos,
                        rng,
                        bar,
                    )?;

                    ptr = new_ptr;
                    min = min.min(child_min);
                    max = max.max(child_max);

                    i += 1;
                }
            }
        }

        self.saver.metadata.aabbs[page_nr].min = min;
        self.saver.metadata.aabbs[page_nr].max = max;

        Ok((ptr, min, max))
    }

    fn set_node(&mut self, page_nr: usize, in_page_index: usize, node: Node) -> Result<()> {
        let page_index = self.get_page_index(page_nr)?;

        self.pages[page_index].nodes[in_page_index] = node;
        self.pages[page_index].write_counter += 1;

        if self.pages[page_index].write_counter >= PAGE_SIZE {
            self.save_page(page_index)?;
        }

        Ok(())
    }

    fn save_page(&mut self, page_index: usize) -> Result<()> {
        self.saver
            .save_page(&self.pages[page_index].nodes, self.pages[page_index].nr)?;
        self.pages.remove(page_index);

        Ok(())
    }

    fn get_page_index(&mut self, page_nr: usize) -> Result<usize> {
        if page_nr >= self.page_nr_counter {
            self.pages.push(Page::new(page_nr));
            self.page_nr_counter += 1;

            self.saver.metadata.aabbs.push(AABB::default());

            return Ok(self.pages.len() - 1);
        }

        for (i, page) in self.pages.iter().enumerate() {
            if page_nr == page.nr {
                return Ok(i);
            }
        }

        bail!("Page is unloaded");
    }

    fn done(&mut self) -> Result<()> {
        let len = self.pages.len();
        for i in (0..len).rev() {
            self.save_page(i)?;
        }

        self.saver.done()?;

        Ok(())
    }
}

impl Page {
    pub fn new(nr: usize) -> Page {
        Page {
            nr: nr,
            nodes: [Node::default(); PAGE_SIZE],
            write_counter: 0,
        }
    }
}
