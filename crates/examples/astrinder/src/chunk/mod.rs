use std::{sync::mpsc::Sender};

use app::{glam::{Vec2, vec2, Vec3}};
use app::anyhow::*;

use self::{particle::{Particle}, transform::Transform, chunk::Chunk, physics::destruction::DestructionSolver, part::PartIdCounter};


pub mod render;
pub mod physics;

pub mod math;
pub mod transform;
pub mod debug;

pub mod chunk;
pub mod part;
pub mod particle;

pub mod shapes;

const CHUNK_PART_SIZE: i32 = 10;
const MAX_AMMOUNT_OF_PARTS: usize = 10000;
const USE_FIXED_TIME_STEP: bool = true;
const FIXED_TIME_STEP: f32 = 1.0 / 30.0;
const CONTROLLER_FRAME_RATE: u32 = 30;

pub struct ChunkController {
    pub chunks: Vec<Chunk>,
    part_id_counter: PartIdCounter,

    destruction_solver: DestructionSolver,

    to_render_transform: Sender<(usize, Transform)>,
    to_render_particles: Sender<(usize, [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
    to_debug: Sender<(Vec2, Vec2, Vec3)>,
}

impl ChunkController {
    pub fn new(
        to_render_transform: Sender<(usize, Transform)>,
        to_render_particles: Sender<(usize, [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
        to_debug: Sender<(Vec2, Vec2, Vec3)>,
        ) -> Self {
        let mut chunks = Vec::new();
        let mut part_id_counter = PartIdCounter::new(MAX_AMMOUNT_OF_PARTS);
        let destruction_solver = DestructionSolver::new();

        /* 
        chunks.push(Chunk::new_cube(
            Transform::new(vec2(0.0, 0.0), 0.0), 
            Transform::new(vec2(0., 0.), 1.1),
            uvec2(6, 6),
            &mut part_id_counter,
        ));
        */

        
        chunks.push(Chunk::new_hexagon(
            Transform::new(vec2(0.0, 0.0), 0.0), 
            Transform::new(vec2(0., 0.), 0.0),
            20,
            &mut part_id_counter)); 

        chunks.push(Chunk::new_hexagon(
            Transform::new(vec2(2.0, 50.0), 0.0), 
            Transform::new(vec2(0.0, -11.0), 0.0),
            1,
            &mut part_id_counter)); 
        
        /* 
        chunks.push(destruction_solver.patterns[2].into_chunk( 
            Transform::new(vec2(0.0, 0.0), 0.0), 
            Transform::new(vec2(0., 0.), 0.0),
            &mut part_id_counter));
        */

        Self { 
            chunks,
            part_id_counter,
            destruction_solver,
            to_render_transform,
            to_render_particles,
            to_debug,
        }
    }

    pub fn run(&mut self){

        let mut fps = fps_clock::FpsClock::new(CONTROLLER_FRAME_RATE);
        let mut nanosecs_since_last_tick = 0.0;
        loop {
            let time_step = if !USE_FIXED_TIME_STEP { nanosecs_since_last_tick * 1e-9 } else { FIXED_TIME_STEP };

            self.update_physics(time_step);
            let _ = self.send_parts();

            self.send_debug();

            nanosecs_since_last_tick = fps.tick();
        }
    }

    pub fn send_parts(&mut self) -> Result<()> {
        for chunk in self.chunks.iter() {
            for part in chunk.parts.iter() {
                self.to_render_transform.send((part.id, part.transform))?;
                self.to_render_particles.send((part.id, part.particles))?;
            }
        }

        Ok(())
    }


}

