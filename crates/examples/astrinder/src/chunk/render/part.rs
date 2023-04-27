use crate::chunk::{CHUNK_PART_SIZE, transform::Transform, ChunkPart, Chunk, particle::Particle};

use super::ChunkRederer;

#[derive(Copy, Clone)]
pub struct RenderPart{
    pub id: u32,
    pub particles: [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    pub transform: Transform,
}


#[derive(Copy, Clone)]
pub struct RenderParticle {
    pub material: u32,
}


impl ChunkRederer {
    pub fn update_chunk(&mut self, chunk: &Chunk){
        for part in chunk.parts.iter() {
            self.update_part_transform(part.id, part.transform);
            self.update_part_particles(part.id, part)
        }
    }

    fn update_part_transform(&mut self, id: usize, transform: Transform) {
        self.parts[id].transform = transform;
    }

    fn update_part_particles(&mut self, id: usize, part: &ChunkPart) {
        for (i, particle) in part.particles.iter().enumerate() {
            self.parts[id].particles[i] = RenderParticle::from(particle)
        }
    }
}


impl Default for RenderPart {
    fn default() -> Self {
        Self { 
            particles: [RenderParticle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
            ..Default::default()
        }
    }
}

impl Default for RenderParticle {
    fn default() -> Self {
        Self { 
            material: 0 
        }
    }
}

impl From<&Particle> for RenderParticle{
    fn from(value: &Particle) -> Self {
        Self { 
            material: value.material,
        }
    }
}