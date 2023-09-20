use app::anyhow::Result;
use std::{
    fs::{self, File},
    io::Write,
};

use crate::{metadata::Metadata, node::Node, util::{self, create_dir}};

pub struct Saver {
    pub path: String,
    pub metadata: Metadata,
}

impl Saver {
    pub fn new(path: String, depth: usize, page_size: usize) -> Result<Saver> {
        create_dir(&path)?;

        Ok(Saver {
            path: path,
            metadata: Metadata::new(depth, page_size),
        })
    }

    pub fn save_page(&mut self, page: &[Node], page_nr: usize) -> Result<()> {
        self.metadata.page_ammount += 1;

        let mut file = File::create(format!("{}/{}.nodes", self.path, page_nr))?;
        let buffer = unsafe { util::slice_as_u8_slice(page) };
        file.write_all(buffer)?;

        Ok(())
    }

    pub fn done(&mut self) -> Result<()> {
        self.metadata.save(&self.path)?;
        Ok(())
    }
}


