use app::glam::{IVec2};

use crate::math::transform::Transform;

use super::{particle::Particle, CHUNK_PART_SIZE};

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

