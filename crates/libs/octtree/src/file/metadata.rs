use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::Tree;

use octa_force::anyhow::Result;
use octa_force::anyhow::{format_err, Ok};

use serde::{Deserialize, Serialize};

const METADAT_FILE_NAME: &str = "metadata";

#[derive(Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub depth: u16,
    pub size: u64,
    pub max_size: u64,
    pub batch_size: usize,
    pub batches: Vec<BatchMetadata>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct BatchMetadata {
    pub index: u64,
    pub start: u64,
    pub end: u64,
    pub size: u64,
}

impl Metadata {
    pub fn new(tree: &impl Tree, batches: Vec<BatchMetadata>, batch_size: usize) -> Self {
        Self {
            depth: tree.get_depth(),
            size: tree.get_size(),
            max_size: tree.get_max_size(),
            batch_size,
            batches,
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

    pub fn get_batch_metadata(&self, index: usize) -> Result<&BatchMetadata> {
        self.batches
            .iter()
            .find(|b| b.index == (index as u64))
            .ok_or(format_err!("Batch metadata with {index} not found!"))
    }
}

impl BatchMetadata {
    pub fn new(index: u64, start: u64, end: u64, size: u64) -> Self {
        Self {
            index,
            start,
            end,
            size,
        }
    }
}
