use std::{fs::{self, OpenOptions}, io::Write};

use crate::Tree;

use app::anyhow::Ok;
use::app::anyhow::Result;

use serde::{Deserialize, Serialize};

const METADAT_FILE_NAME: &str = "metadata";

#[derive(Serialize, Deserialize)]
pub struct Metadata{
    depth: u16,
    size: u64,
    max_size: u64,
    batches: Vec<BatchMetadata>,
}

#[derive(Serialize, Deserialize)]
pub struct BatchMetadata{
    index: u64, 
    start: u64,
    end: u64,
    size: u64,
}


impl Metadata {
    pub fn new(tree: &impl Tree, batches: Vec<BatchMetadata>) -> Self {
        Self { 
            depth: tree.get_depth(), 
            size: tree.get_size(), 
            max_size: tree.get_max_size(), 
            batches: batches, 
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

impl BatchMetadata {
    pub fn new(index: u64, start: u64, end: u64, size: u64) -> Self { Self { index, start, end, size } }
}

