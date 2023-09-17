use std::mem::size_of;

use app::{
    anyhow::{bail, Result},
    glam::{uvec3, vec3, UVec3, Vec3},
    log,
};
use octtree_v2::{load::load_page, metadata::Metadata, node::Node};

pub struct Octtree {
    pub path: String,
    pub metadata: Metadata,
    pub loaded_pages: usize,
    pub pages: Vec<Page>,
}

pub struct Page {
    pub nr: usize,
    pub nodes: Vec<Node>,
    pub last_use_in_frame: usize,
}

impl Octtree {
    pub fn new(path: String, loaded_pages: usize) -> Result<Octtree> {
        log::info!("Loading Tree");

        let metadata = Metadata::load(&path)?;
        let mut octtree = Octtree {
            path: path,
            metadata: metadata,
            loaded_pages: loaded_pages,
            pages: Vec::new(),
        };

        octtree.load_pages((0..loaded_pages.min(metadata.page_ammount)).collect(), 0)?;

        let current_size = (size_of::<Node>() * octtree.metadata.page_size + size_of::<Page>())
            * octtree.pages.len();
        log::info!(
            "Size: {} byte {} MB {} GB",
            current_size,
            current_size as f32 / 1000000.0,
            current_size as f32 / 1000000000.0
        );

        let possible_size =
            (size_of::<Node>() * octtree.metadata.page_size + size_of::<Page>()) * loaded_pages;
        log::info!(
            "Max Size: {} byte {} MB {} GB",
            possible_size,
            possible_size as f32 / 1000000.0,
            possible_size as f32 / 1000000000.0
        );

        Ok(octtree)
    }

    pub fn load_page(&mut self, page_nr: usize, current_frame: usize) -> Result<usize> {
        let nodes = load_page(&self.path, page_nr, self.metadata.page_size)?;
        let page = Page {
            nr: page_nr,
            nodes: nodes,
            last_use_in_frame: current_frame,
        };

        self.pages.push(page);
        self.sort();

        Ok(self.pages.len() - 1)
    }

    pub fn load_pages(&mut self, page_nrs: Vec<usize>, current_frame: usize) -> Result<()> {
        for page_nr in page_nrs {
            let nodes = load_page(&self.path, page_nr, self.metadata.page_size)?;
            let page = Page {
                nr: page_nr,
                nodes: nodes,
                last_use_in_frame: current_frame,
            };

            self.pages.push(page);
        }

        self.sort();

        Ok(())
    }

    pub fn get_page_index(&self, page_nr: usize) -> Result<usize> {
        let result = self.pages.binary_search_by(|a| a.nr.cmp(&page_nr));
        if result.is_err() {
            bail!("Page not found.")
        }

        Ok(result.unwrap())
    }

    pub fn sort(&mut self) {
        self.pages.sort_by(|a, b| a.nr.cmp(&b.nr));
    }

    pub fn clean(&mut self) {
        // Sort by last frame use
        let mut last_frame_indices = (0..self.pages.len()).rev().collect::<Vec<_>>();
        last_frame_indices.sort_by_key(|&i| &self.pages[i].last_use_in_frame);

        if self.pages.len() > self.loaded_pages {
            // Remove oldest pages
            for i in 0..self.pages.len() - self.loaded_pages {
                self.pages.swap_remove(last_frame_indices[i]);
            }

            // Sort to fix the order
            self.sort();
        }
    }

    pub fn get_node_pos(pos: Vec3, size: u32) -> UVec3 {
        return (uvec3(pos.x as u32, pos.y as u32, pos.z as u32) / size
            - uvec3(
                (pos.x < 0.0) as u32,
                (pos.x < 0.0) as u32,
                (pos.x < 0.0) as u32,
            ))
            * size;
    }
}
