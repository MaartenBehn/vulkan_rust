use app::glam::{IVec2, ivec2};
use collision::primitive::ConvexPolygon;

use crate::chunk::math::{neigbor_pos_offsets, hex_to_coord, vec2_to_point2};

use super::{particle::Particle, transform::Transform, CHUNK_PART_SIZE};




#[derive(Clone, PartialEq)]
pub struct ChunkPart{
    pub id: usize,
    pub pos: IVec2,
    pub particles: [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    pub transform: Transform,
}

impl ChunkPart {
    pub fn new(pos: IVec2, id: usize) -> Self {
        Self { 
            id: id,
            pos: pos,
            particles: [Particle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
            transform: Transform::default(),
        }
    }   

    pub fn update_colliders(&mut self) {

        
    }
}

#[derive(Clone, Debug)]
pub struct PartIdCounter {
    free_ids: Vec<usize>,
}

impl PartIdCounter {
    pub fn new(size: usize) -> Self {
        let mut free_ids = Vec::new();

        for i in (0..size).rev() {
            free_ids.push(i);
        }

        Self { 
            free_ids,
        }
    }

    pub fn add_free(&mut self, free_id: usize) {
        self.free_ids.push(free_id);
    }

    pub fn pop_free(&mut self) -> Option<usize> {
        self.free_ids.pop()
    }
}