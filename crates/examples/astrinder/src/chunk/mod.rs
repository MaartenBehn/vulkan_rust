use app::glam::{UVec2, Vec2, IVec2, uvec2};
use crate::{aabb::AABB, chunk::math::*};

use self::particle::{Particle};

mod math;
mod particle;
mod shapes;
pub mod render;
mod collision;

const CHUNK_PART_SIZE: i32 = 10;

pub struct ChunkController {
    chunks: Vec<Chunk>
}

impl ChunkController {
    pub fn new() -> Self {
        let mut chunks = Vec::new();

        chunks.push(Chunk::new_circle(Vec2::ZERO, 0.0, 5));

        Self { 
            chunks: chunks 
        }
    }
}

pub struct Chunk { 
    parts: Vec<ChunkPart>, 
    parts_id_counter: usize,

    mass: u32,
    aabb: AABB,

    pos: Vec2,
    rot: f32,
}

#[allow(dead_code)]
impl Chunk {
    pub fn new(pos: Vec2, rot: f32, particles: Vec<(Particle, IVec2)>) -> Self {
        let mut chunk = Self { 
            parts: Vec::new(),
            parts_id_counter: 0,
            mass: 0,
            aabb: AABB::default(),
            pos: pos,
            rot: rot,
        };

        for (p, hex_pos) in particles {
            chunk.add_particle(p, hex_pos)
        }

        chunk
    }

    pub fn add_particle(
        &mut self, 
        p: Particle, 
        hex_pos: IVec2,
    ){
        let part_pos = hex_to_chunk_part_pos(hex_pos);
        
        let mut part = None;
        for p in &mut self.parts {
            if p.pos == part_pos  {
                part = Some(p);
            }
        }

        if part.is_none() {
            let new_part= ChunkPart::new(self.parts_id_counter, part_pos);
            self.parts_id_counter += 1;

            self.parts.push(new_part);

            part = self.parts.last_mut();
        }
        debug_assert!(part.is_some());

        let in_part_pos = hex_to_in_chunk_part_pos(hex_pos);
        part.unwrap().particles[in_part_pos] = p;
        
        self.mass += p.mass as u32;
        self.aabb.extend(hex_pos)
    }
}


#[derive(Copy, Clone)]
pub struct ChunkPart{
    pos: IVec2,
    particles: [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize]
}

impl ChunkPart {
    pub fn new(id: usize, pos: IVec2) -> Self {
        Self { 
            pos: pos,
            particles: [Particle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize] 
        }
    }   
}




