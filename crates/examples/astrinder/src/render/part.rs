use crate::{
    chunk::{particle::Particle, CHUNK_PART_SIZE},
    math::transform::Transform,
};

#[derive(Copy, Clone)]
pub struct RenderPart {
    pub id: usize,
    pub particles: [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    pub transform: Transform,
}

#[derive(Copy, Clone)]
pub struct RenderParticle {
    pub material: u32,
}

impl Default for RenderPart {
    fn default() -> Self {
        Self {
            id: 0,
            particles: [RenderParticle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
            transform: Transform::default(),
        }
    }
}

impl Default for RenderParticle {
    fn default() -> Self {
        Self { material: 0 }
    }
}

impl From<&Particle> for RenderParticle {
    fn from(value: &Particle) -> Self {
        Self {
            material: value.material,
        }
    }
}
