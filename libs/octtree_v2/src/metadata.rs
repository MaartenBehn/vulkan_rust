use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use octa_force::anyhow::Result;

use crate::aabb::AABB;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub page_size: usize,
    pub page_ammount: usize,
    pub depth: usize,

    pub aabbs: Vec<AABB>,
}

impl Metadata {
    pub fn new(page_size: usize, page_ammount: usize, depth: usize) -> Metadata {
        Metadata {
            page_size,
            page_ammount,
            depth,
            aabbs: Vec::new(),
        }
    }

    pub fn from_file(path: &str) -> Result<Metadata> {
        let json = fs::read_to_string(format!("{}/metadata.json", path))?;
        let metadata = serde_json::from_str(&json)?;
        Ok(metadata)
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("{}/metadata.json", path))
            .unwrap();

        file.write(json.as_bytes())?;

        Ok(())
    }
}
