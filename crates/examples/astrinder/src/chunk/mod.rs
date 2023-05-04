use std::{sync::mpsc::Sender};

use app::{glam::{Vec2, vec2, Vec3}};
use app::anyhow::*;

use crate::{settings::{Settings, self}, ENABLE_DEBUG_RENDER};

use self::{particle::{Particle}, transform::Transform, chunk::Chunk, physics::destruction::DestructionSolver, part::{PartIdCounter, ChunkPart}, render::part::RenderParticle};


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


pub struct ChunkController {
    pub chunks: Vec<Chunk>,
    part_id_counter: PartIdCounter,

    destruction_solver: DestructionSolver,

    to_render_transform: Sender<(usize, Transform)>,
    to_render_particles: Sender<(usize, [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
    to_debug: Sender<(Vec2, Vec2, Vec3)>,

    settings: Settings,
}

impl ChunkController {
    pub fn new(
        to_render_transform: Sender<(usize, Transform)>,
        to_render_particles: Sender<(usize, [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
        to_debug: Sender<(Vec2, Vec2, Vec3)>,
        settings: Settings,
        ) -> Self {
        let mut chunks = Vec::new();
        let mut part_id_counter = PartIdCounter::new(settings.max_rendered_parts);
        let destruction_solver = DestructionSolver::new();

        many_chunks(&mut chunks, &mut part_id_counter, settings);
        //destruction(&mut chunks, &mut part_id_counter, settings);

        let mut c = Self { 
            chunks,
            part_id_counter,

            destruction_solver,
            to_render_transform,
            to_render_particles,
            to_debug,

            settings
        };

        let _ = c.send_all_chunks();

        c
    }

    pub fn run(&mut self, settings: Settings) -> Result<()> {

        let mut fps = fps_clock::FpsClock::new(settings.max_chunk_ups);
        let mut nanosecs_since_last_tick = 0.0;
        loop {
            let time_step = if !settings.chunk_ups_use_fixed_time_step { 
                nanosecs_since_last_tick * 1e-9 
            } else { 
                settings.chunk_ups_fixed_time_step 
            };

            self.update_physics(time_step)?;

            if ENABLE_DEBUG_RENDER && cfg!(debug_assertions){
                self.send_debug();
            }
            
            nanosecs_since_last_tick = fps.tick();
        }

        Ok(())
    }

    pub fn send_all_chunks(&self) -> Result<()> {
        for chunk in self.chunks.iter() {
            chunk.send(&self.to_render_transform, &self.to_render_particles)?;
        }

        Ok(())
    }


}


fn destruction(chunks: &mut Vec<Chunk>, part_id_counter: &mut PartIdCounter, settings: Settings){
    chunks.push(Chunk::new_hexagon(
        Transform::new(vec2(0.0, 0.0), 0.0), 
        Transform::new(vec2(0., 0.), 0.0),
        20,
        part_id_counter,
        settings)); 

    chunks.push(Chunk::new_hexagon(
        Transform::new(vec2(2.0, 30.0), 0.0), 
        Transform::new(vec2(0.0, -1.0), 0.0),
        1,
        part_id_counter,
        settings)); 
}


fn many_chunks(chunks: &mut Vec<Chunk>, part_id_counter: &mut PartIdCounter, settings: Settings){
    for x in -10..10 {
        for y in -10..10 {
            chunks.push(Chunk::new_hexagon(
                Transform::new(vec2(x as f32 * 4.0, y as f32 * 4.0), 0.0), 
                Transform::new(vec2(0., 0.), 0.0),
                0,
                part_id_counter,
                settings)); 
        }
    }
}


