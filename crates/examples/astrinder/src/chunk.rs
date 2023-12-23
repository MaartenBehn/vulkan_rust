use crate::{
    aabb::AABB,
    particle::{Particle, PARTICLE_RADIUS},
};
use app::glam::{IVec2, UVec2, Vec2};

const CHUNK_PART_SIZE: u32 = 10;

pub struct Chunk {
    parts: Vec<ChunkPart>,
    parts_id_counter: usize,

    mass: f32,
    aabb: AABB,
}

#[derive(Copy, Clone)]
pub struct ChunkPart {
    id: usize,
    pos: UVec2,
    particles: [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
}

#[allow(dead_code)]
impl Chunk {
    pub fn new(particles: Vec<(Particle, UVec2)>) -> Self {
        let mut chunk = Self {
            parts: Vec::new(),
            parts_id_counter: 0,
            mass: 0.0,
            aabb: AABB::default(),
        };

        for (p, hex_pos) in particles {
            chunk.add_particle(p, hex_pos)
        }

        chunk
    }

    pub fn add_particle(&mut self, p: Particle, hex_pos: UVec2) {
        let part_pos = hex_to_chunk_part_pos(hex_pos);

        let mut part = None;
        for p in &mut self.parts {
            if p.pos == part_pos {
                part = Some(p);
            }
        }

        if part.is_none() {
            let new_part = ChunkPart::new(self.parts_id_counter, part_pos);
            self.parts_id_counter += 1;

            self.parts.push(new_part);

            part = self.parts.last_mut();
        }
        debug_assert!(part.is_some());

        let in_part_pos = hex_to_in_chunk_part_pos(hex_pos);
        part.unwrap().particles[in_part_pos] = p;

        self.mass += p.mass;
        self.aabb.extend(hex_pos)
    }

    /*
    pub fn check_collision_with_particle(&self, chunk_pos: Vec2, check_pos: Vec2) -> bool {
        self.particles.query_around((check_pos - chunk_pos).to_array(), PARTICLE_RADIUS).peekable().peek().is_some()
    }

    pub fn check_collision_with_chunk(&self, chunk_pos: Vec2, other_chunk: &Chunk, other_pos: Vec2) -> bool {
        for (pos, _) in self.particles.objects(){

            let check_pos = Vec2::from_array(pos) + chunk_pos - other_pos;
            if other_chunk.particles.query_around(check_pos.to_array(), PARTICLE_RADIUS).peekable().peek().is_some(){
                return true;
            }
        }
        return false;
    }
    */
}

#[allow(dead_code)]
impl Chunk {
    pub fn new_cube(size: UVec2) -> Self {
        let mut particles = Vec::new();

        for x in 0..size.x {
            for y in 0..size.y {
                let hex = UVec2::new(x, y);
                particles.push((Particle::new(), hex))
            }
        }

        Self::new(particles)
    }

    pub fn new_circle(layers: u32) -> Self {
        let mut particles = Vec::new();

        let hex_dirs = [
            IVec2::new(1, 0),
            IVec2::new(0, 1),
            IVec2::new(-1, 1),
            IVec2::new(-1, 0),
            IVec2::new(0, -1),
            IVec2::new(1, -1),
        ];

        let mut hex = IVec2::ZERO;
        particles.push((Particle::new(), hex.as_uvec2()));

        for layer in 1..layers {
            for dir in 0..5 {
                for _ in 0..layer {
                    hex += hex_dirs[dir];
                    particles.push((Particle::new(), hex.as_uvec2()));
                }
            }
        }

        Self::new(particles)
    }
}

pub fn hex_to_coord(hex: UVec2) -> Vec2 {
    Vec2::new(
        hex.x as f32 * PARTICLE_RADIUS * 2.0 + hex.y as f32 * PARTICLE_RADIUS,
        hex.y as f32 * PARTICLE_RADIUS * 1.5,
    )
}

pub fn hex_to_chunk_part_pos(hex: UVec2) -> UVec2 {
    UVec2::new(hex.x / CHUNK_PART_SIZE, hex.y / CHUNK_PART_SIZE)
}

pub fn hex_to_in_chunk_part_pos(hex: UVec2) -> usize {
    (hex.x * CHUNK_PART_SIZE + hex.y) as usize
}

impl ChunkPart {
    pub fn new(id: usize, pos: UVec2) -> Self {
        Self {
            id: id,
            pos: pos,
            particles: [Particle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
        }
    }
}
