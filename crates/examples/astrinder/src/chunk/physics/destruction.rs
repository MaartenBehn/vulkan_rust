use app::glam::{Vec2, vec2, ivec2, IVec2};
use rand::rngs::StdRng;
use rand::{SeedableRng, Rng};

use crate::chunk::part::PartIdCounter;
use crate::chunk::particle::Particle;
use crate::chunk::CHUNK_PART_SIZE;
use crate::chunk::chunk::Chunk;
use crate::chunk::math::{hex_to_coord, hex_to_particle_index};
use crate::chunk::transform::Transform;
use crate::settings::Settings;


const BREAK_PATTERN_SIZE: usize = 100;

pub enum FallOffFunc {
    Linear(f32),
    Quadratic(f32, f32),
    Qubic(f32, f32, f32),
}

pub struct DestructionSolver{
    pub patterns: Vec<BreakPattern>,
}

impl DestructionSolver {
    pub fn new() -> Self {
        let patterns = vec![
            BreakPattern::new(100, FallOffFunc::Qubic(50.0, 0.0, 0.0), 2),
        ];

        Self { 
            patterns 
        }
    }
}

#[derive(Clone, Debug)]
pub struct BreakPattern{
    points_len: usize,
    grid: Vec<u32>,
}

impl BreakPattern {
    pub fn new(points_num: usize, fall_off_func: FallOffFunc, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let mut points = Vec::with_capacity(points_num);
        
        for _ in 0..points_num {
            let rand_point = vec2(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0));

            let point = match fall_off_func {
                FallOffFunc::Linear(a) => apply_linear_fall_off(rand_point, a),
                FallOffFunc::Quadratic(a, b) => apply_quadratic_fall_off(rand_point, a, b),
                FallOffFunc::Qubic(a, b, c) => apply_quibic_fall_off(rand_point, a, b, c),
            };

            points.push(point)
        }

        let mut grid = Vec::new();
        let half_size = (BREAK_PATTERN_SIZE / 2) as i32;

        grid.resize(BREAK_PATTERN_SIZE * BREAK_PATTERN_SIZE, u32::default());
        for x in 0..BREAK_PATTERN_SIZE as i32 {
            for y in 0..BREAK_PATTERN_SIZE as i32 {
                let hex = ivec2(x, y);
                let pos = hex_to_coord(hex - half_size);

                let mut closest_index = 0;
                let mut closest_dist = f32::MAX;

                for (i, point) in points.iter().enumerate() {
                    let dist = pos.distance(*point);

                    if dist < closest_dist {
                        closest_index = i;
                        closest_dist = dist;
                    }
                }

                grid[(hex.x * BREAK_PATTERN_SIZE as i32 + hex.y) as usize] = closest_index as u32;
            }
        }

        Self { 
            points_len: points.len(),
            grid,
        }
    }

    #[allow(dead_code)]
    pub fn into_chunk(&self, transform: Transform, velocity_transform: Transform, part_id_counter: &mut PartIdCounter, settings: Settings) -> Chunk {

        let mut particles = Vec::with_capacity(self.grid.len());
        for x in 0..BREAK_PATTERN_SIZE as i32 {
            for y in 0..BREAK_PATTERN_SIZE as i32 {
                let hex = ivec2(x, y);
                let material = self.grid[(hex.x * BREAK_PATTERN_SIZE as i32 + hex.y) as usize];
                
                particles.push((Particle::new(1, material * 850), hex))
            }
        }

        Chunk::new(transform, velocity_transform, particles, part_id_counter, true, settings)
    }

    pub fn apply_to_chunk(&self, chunk: &Chunk, start_hex: IVec2, part_id_counter: &mut PartIdCounter, settings: Settings) -> Vec<Chunk> {
        let mut new_particles = Vec::new();
        new_particles.resize_with(self.points_len, || { Vec::new() });

        let half_size = (BREAK_PATTERN_SIZE / 2) as i32;
        for part in chunk.parts.iter() {
            for x in 0..CHUNK_PART_SIZE {
                for y in 0..CHUNK_PART_SIZE {
                    let hex = ivec2(x, y);
                    let mut particle = part.particles[hex_to_particle_index(hex)];
                    if particle.mass == 0 {
                        continue;
                    }
                    
                    let hex_pattern = half_size - start_hex + hex + part.pos * CHUNK_PART_SIZE;
                    let index = self.grid[(hex_pattern.x * BREAK_PATTERN_SIZE as i32 + hex_pattern.y) as usize];

                    particle.material = index * 850;
                   
                    new_particles[index as usize].push((particle, hex + part.pos * CHUNK_PART_SIZE))
                }
            }
        }

        let mut new_chunks = Vec::new();
        let force_transform = chunk.velocity_transform * (1.0 / chunk.mass);
        for particles in new_particles {
            if particles.is_empty() {
                continue;
            }

            let new_chunk = Chunk::new(
                chunk.transform, 
                force_transform * particles.len() as f32, 
                particles, 
                part_id_counter,
                false,
                settings);
            new_chunks.push(new_chunk);
        }

        new_chunks
    }
}

fn apply_linear_fall_off(pos: Vec2, a: f32) -> Vec2 {
    a * pos
}

fn apply_quadratic_fall_off(pos: Vec2, a: f32, b: f32) -> Vec2 {
    let l = pos.length();
    a * l * pos + b * pos
}

fn apply_quibic_fall_off(pos: Vec2, a: f32, b: f32, c: f32) -> Vec2 {
    let l = pos.length();
    a * l * l * pos + b * l * pos + c * pos
}

