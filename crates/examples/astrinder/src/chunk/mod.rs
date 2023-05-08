use std::sync::mpsc::Sender;
use std::time::Instant;

use app::anyhow::*;
use app::glam::{vec2, IVec2, Vec2, Vec3};
use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

use crate::{
    math::transform::Transform, render::part::RenderParticle, settings::Settings,
    ENABLE_DEBUG_RENDER,
};

use self::particle::Particle;
use self::{chunk::Chunk, physics::PhysicsController};

pub mod chunk;
pub mod debug;
pub mod part;
pub mod particle;
pub mod shapes;

pub mod physics;

pub const CHUNK_PART_SIZE: i32 = 10;

pub struct ChunkController {
    pub chunks: Vec<Chunk>,

    chunk_id_counter: IdCounter,
    part_id_counter: IdCounter,

    to_render_transform: Sender<(usize, Transform)>,
    to_render_particles: Sender<(
        usize,
        [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    )>,
    to_debug: Sender<(Vec2, Vec2, Vec3)>,

    pub settings: Settings,

    physics_controller: PhysicsController,

    step: usize,
}

impl ChunkController {
    pub fn new(
        to_render_transform: Sender<(usize, Transform)>,
        to_render_particles: Sender<(
            usize,
            [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
        )>,
        to_debug: Sender<(Vec2, Vec2, Vec3)>,
        settings: Settings,
    ) -> Self {
        let chunks = Vec::new();

        let chunk_id_counter = IdCounter::new(settings.max_chunks);
        let part_id_counter = IdCounter::new(settings.max_rendered_parts);

        let physics_controller = PhysicsController::new();

        let mut controller = Self {
            chunks,

            chunk_id_counter,
            part_id_counter,

            to_render_transform,
            to_render_particles,
            to_debug,

            settings,

            physics_controller,

            step: usize::MAX,
        };

        controller.destruction();

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

            if self.step != 0 {
                self.check_break();
            }

            self.update_gravity();

            for chunk in self.chunks.iter_mut() {
                self.physics_controller.update_chunk(chunk);

                let _ = chunk.send_transform(&self.to_render_transform);
            }

            if ENABLE_DEBUG_RENDER && cfg!(debug_assertions) {
                self.send_debug();
            }

            nanosecs_since_last_tick = fps.tick();
        }

        Ok(())
    }

    fn destruction(&mut self) {
        self.new_hexagon(
            Transform::new(vec2(0.0, 0.0), 0.0),
            Transform::new(vec2(0., 0.), 0.0),
            10,
        );

        self.new_hexagon(
            Transform::new(vec2(2.0, 30.0), 0.0),
            Transform::new(vec2(0.0, -50.0), 0.0),
            2,
        );
    }

    fn many_chunks(&mut self) {
        for x in -10..10 {
            for y in -10..10 {
                self.new_hexagon(
                    Transform::new(vec2(x as f32 * 4.0, y as f32 * 4.0), 0.0),
                    Transform::new(vec2(0., 0.), 0.0),
                    1,
                );
            }
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

        Self { free_ids }
    }

    pub fn add_free(&mut self, free_id: usize) {
        self.free_ids.push(free_id);
    }

    pub fn pop_free(&mut self) -> Option<usize> {
        self.free_ids.pop()
    }
}
