use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use app::anyhow::Result;
use app::anyhow::Ok;

use serde::{Deserialize, Serialize};

use crate::aabb::AABB;

const METADAT_FILE_NAME: &str = "metadata";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Metadata {
    pub depth: usize,
    pub page_size: usize,
    pub page_ammount: usize,
    pub aabbs: Vec<AABB>,
}

impl Metadata {
    pub fn new(depth: usize, page_size:usize) -> Self {
        Self {
            depth: depth,
            page_size: page_size,
            page_ammount: 0,
            aabbs: Vec::new(),
        }
    }

    pub fn save(&self, folder_path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("{folder_path}/{METADAT_FILE_NAME}.json"))
            .unwrap();

        file.write(json.as_bytes())?;

        Ok(())
    }

    pub fn load(folder_path: &str) -> Result<Metadata> {
        let json = fs::read_to_string(format!("{folder_path}/{METADAT_FILE_NAME}.json"))?;

        let metadata: Metadata = serde_json::from_str(&json)?;

        Ok(metadata)
    }
}

