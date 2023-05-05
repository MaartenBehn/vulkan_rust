use std::{sync::mpsc::Sender};

use app::{glam::{Vec2, vec2, Vec3}};
use app::anyhow::*;

use crate::{settings::Settings, ENABLE_DEBUG_RENDER, math::transform::Transform, render::part::RenderParticle};

use self::{chunk::{Chunk}, physics::PhysicsController};

pub mod particle;
pub mod part;
pub mod chunk;
pub mod shapes;
pub mod debug;

pub mod physics;

pub const CHUNK_PART_SIZE: i32 = 10;

pub struct ChunkController {
    pub chunks: Vec<Chunk>,

    chunk_id_counter: IdCounter,
    part_id_counter: IdCounter,
    
    to_render_transform: Sender<(usize, Transform)>,
    to_render_particles: Sender<(usize, [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
    to_debug: Sender<(Vec2, Vec2, Vec3)>,

    pub settings: Settings,

    physics_controller: PhysicsController
}

impl ChunkController {
    pub fn new(
        to_render_transform: Sender<(usize, Transform)>,
        to_render_particles: Sender<(usize, [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize])>,
        to_debug: Sender<(Vec2, Vec2, Vec3)>,
        settings: Settings,
        ) -> Self {
        let mut chunks = Vec::new();

        let mut chunk_id_counter = IdCounter::new(100);
        let mut part_id_counter = IdCounter::new(settings.max_rendered_parts);
        

        let mut physics_controller = PhysicsController::new();

        //many_chunks(&mut chunks, chunk_id_counter, &mut part_id_counter, settings, &mut physics_controller);
        destruction(&mut chunks, &mut chunk_id_counter, &mut part_id_counter, settings, &mut physics_controller);

        let controller = Self { 
            chunks,

            chunk_id_counter,
            part_id_counter,

            to_render_transform,
            to_render_particles,
            to_debug,

            settings,

            physics_controller,
        };

        let _ = controller.send_all_chunks();

        controller
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

            self.physics_controller.step();
            self.update_gravity();

            for chunk in self.chunks.iter_mut() {
                
                self.physics_controller.update_chunk(chunk);

                let _ = chunk.send_transform(&self.to_render_transform);
            }

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


fn destruction(chunks: &mut Vec<Chunk>, chunk_id_counter: &mut IdCounter, part_id_counter: &mut IdCounter, settings: Settings, physics_controller: &mut PhysicsController){
    
    let id = chunk_id_counter.pop_free().unwrap();
    chunks.insert(id, Chunk::new_hexagon(
        Transform::new(vec2(0.0, 0.0), 0.0), 
        Transform::new(vec2(0., 0.), 0.0),
        20,
        id,
        part_id_counter,
        settings,
        physics_controller)); 

    let id = chunk_id_counter.pop_free().unwrap();
    chunks.insert(id, Chunk::new_hexagon(
        Transform::new(vec2(2.0, 30.0), 0.0), 
        Transform::new(vec2(0.0, -1.0), 0.0),
        2,
        id,
        part_id_counter,
        settings,
        physics_controller)); 
}


fn many_chunks(chunks: &mut Vec<Chunk>, chunk_id_counter: &mut IdCounter, part_id_counter: &mut IdCounter, settings: Settings, physics_controller: &mut PhysicsController){
    for x in -10..10 {
        for y in -10..10 {

            let id = chunk_id_counter.pop_free().unwrap();
            chunks.insert(id, Chunk::new_hexagon(
                Transform::new(vec2(x as f32 * 4.0, y as f32 * 4.0), 0.0), 
                Transform::new(vec2(0., 0.), 0.0),
                1,
                id,
                part_id_counter,
                settings,
                physics_controller)); 
        }
    }
}


#[derive(Clone, Debug)]
pub struct IdCounter {
    free_ids: Vec<usize>,
}

impl IdCounter {
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

