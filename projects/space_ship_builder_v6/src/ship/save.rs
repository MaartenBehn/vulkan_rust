use crate::node::{BlockIndex, BLOCK_INDEX_EMPTY};
use crate::rules::Rules;
use crate::ship::data::ShipData;
use bitcode::{Decode, Encode};
use octa_force::anyhow::Result;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct ShipSave {
    blocks: Vec<([i32; 3], BlockIndex)>,
    nodes_per_chunk: [i32; 3],
}

impl ShipData {
    pub fn save(&self, path: &str) -> Result<()> {
        let save = self.get_save();

        let mut file = File::create(path)?;
        let data: Vec<u8> = bitcode::encode(&save);
        file.write_all(&data)?;

        Ok(())
    }

    pub fn get_save(&self) -> ShipSave {
        let mut blocks = Vec::new();
        for chunk in self.chunks.iter() {
            for (i, block) in chunk.blocks.iter().enumerate() {
                if *block == BLOCK_INDEX_EMPTY {
                    continue;
                }

                let block_pos = self.block_world_pos_from_in_chunk_block_index(i, chunk.pos);

                blocks.push((block_pos.into(), *block))
            }
        }

        ShipSave {
            blocks,
            nodes_per_chunk: self.nodes_per_chunk.into(),
        }
    }

    pub fn load(path: &str, rules: &Rules) -> Result<Self> {
        let mut file = File::open(path)?;

        let metadata = fs::metadata(path)?;
        let mut data = vec![0; metadata.len() as usize];
        file.read(&mut data)?;
        let ship_save: ShipSave = bitcode::decode(&data)?;

        let ship = Self::new_from_save(ship_save, rules);
        Ok(ship)
    }

    pub fn new_from_save(save: ShipSave, rules: &Rules) -> Self {
        let mut ship = ShipData::new(save.nodes_per_chunk[0]);

        for (pos, block) in save.blocks {
            ship.place_block(pos.into(), block, rules);
        }

        ship
    }
}
