use crate::node::BlockIndex;
use crate::ship::Ship;
use bitcode::{Decode, Encode};
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Read, Write};

use octa_force::anyhow::Result;

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct ShipSave {
    chunks: Vec<ShipSaveChunk>,
    nodes_per_chunk: [i32; 3],
}

#[derive(Encode, Decode, PartialEq, Debug)]
struct ShipSaveChunk {
    pub pos: [i32; 3],
    pub blocks: Vec<BlockIndex>,
}

impl ShipSave {
    pub fn save(&self, path: &str) -> Result<()> {
        let mut file = File::create(path)?;

        let data: Vec<u8> = bitcode::encode(self);
        file.write_all(&data)?;

        Ok(())
    }

    pub fn load(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;

        let metadata = fs::metadata(path)?;
        let mut data = vec![0; metadata.len() as usize];
        file.read(&mut data)?;
        let ship_save = bitcode::decode(&data)?;

        Ok(ship_save)
    }
}

impl From<&Ship> for ShipSave {
    fn from(ship: &Ship) -> Self {
        let save_chunks: Vec<_> = ship
            .chunks
            .iter()
            .map(|chunk| ShipSaveChunk {
                pos: chunk.pos.into(),
                blocks: chunk.blocks.to_owned(),
            })
            .collect();

        ShipSave {
            chunks: save_chunks,
            nodes_per_chunk: ship.nodes_per_chunk.into(),
        }
    }
}

impl Into<Ship> for ShipSave {
    fn into(self) -> Ship {
        let mut ship = Ship::new(self.nodes_per_chunk[0]);

        for (i, save_chunk) in self.chunks.into_iter().enumerate() {
            ship.add_chunk(save_chunk.pos.into());
            ship.chunks[i].blocks = save_chunk.blocks;
        }

        ship.recompute();

        ship
    }
}
