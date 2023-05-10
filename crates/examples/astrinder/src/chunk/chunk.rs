use std::{sync::mpsc::Sender, time::Instant};

use app::anyhow::*;
use app::glam::{ivec2, vec2, IVec2, Vec2};
use rapier2d::prelude::*;

use crate::{
    math::{transform::Transform, *},
    render::part::RenderParticle,
    settings::Settings,
};

use super::{
    part::ChunkPart, particle::Particle, physics::PhysicsController, ChunkController, IdCounter,
    CHUNK_PART_SIZE,
};

#[derive(Clone, Default)]
pub struct Chunk {
    pub id: usize,

    pub parts: Vec<ChunkPart>,

    pub transform: Transform,

    pub particle_counter: usize,

    pub rb_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
    pub forces: Vec2,
    pub mass: f32,
    pub stability: f32,
}

impl ChunkController {
    pub fn add_chunk<I>(
        &mut self,
        transform: Transform,
        velocity_transform: Transform,
        particles: I,
        new_spawn: bool,
    ) -> &Chunk
    where
        I: Iterator<Item = (Particle, IVec2)>,
    {
        let id = self.chunk_id_counter.pop_free().unwrap();

        let mut chunk = Chunk {
            id,
            parts: Vec::new(),

            transform,

            particle_counter: 0,

            rb_handle: RigidBodyHandle::default(),
            collider_handle: ColliderHandle::default(),

            forces: Vec2::ZERO,
            mass: 0.0,
            stability: 0.0,
        };

        for (p, hex_pos) in particles {
            chunk.add_particle(p, hex_pos, &mut self.part_id_counter)
        }

        self.physics_controller
            .add_chunk(&mut chunk, velocity_transform);

        chunk.on_chunk_change(
            &mut self.physics_controller,
            &self.to_render_particles,
            true,
        );

        chunk.send_transform(&self.to_render_transform);

        if self.chunks.len() <= id {
            self.chunks.resize(id, Chunk::default());
            self.chunks.push(chunk);
        } else {
            self.chunks[id] = chunk;
        }

        &self.chunks[id]
    }

    pub fn remove_chunk(&mut self, chunk_index: usize) {
        let chunk = &self.chunks[chunk_index];
        self.physics_controller.remove_chunk(chunk);

        for part in chunk.parts.iter() {
            self.part_id_counter.add_free(part.id);
        }

        self.chunk_id_counter.add_free(chunk_index);
    }
}

#[allow(dead_code)]
impl Chunk {
    pub fn add_particle(&mut self, p: Particle, hex_pos: IVec2, part_id_counter: &mut IdCounter) {
        let part_pos = hex_to_chunk_part_pos(hex_pos);
        let mut part = self.get_part_by_pos_mut(part_pos);

        if part.is_none() {
            let part_id = part_id_counter.pop_free();
            if part_id.is_none() {
                println!("Part Id Counter maxed out!!!");
                return;
            }

            let new_part = ChunkPart::new(part_pos, part_id.unwrap());

            self.parts.push(new_part);

            part = self.parts.last_mut();
        }
        debug_assert!(part.is_some());

        let in_part_pos = hex_to_particle_index(hex_pos);
        part.unwrap().particles[in_part_pos] = p.clone();

        self.mass += p.mass as f32;

        self.particle_counter += 1;
    }

    pub fn on_chunk_change(
        &mut self,
        physics_controller: &mut PhysicsController,
        to_render_particles: &Sender<(
            usize,
            [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
        )>,
        collider: bool,
    ) {
        self.stability = 1.0 / self.mass;

        self.update_part_tranforms();

        if collider {
            physics_controller.update_collider(self);
        }

        let _ = self.send_particles(&to_render_particles);
    }

    pub fn update_part_tranforms(&mut self) {
        for part in self.parts.iter_mut() {
            part.transform = part_pos_to_world(self.transform, part.pos);
        }
    }

    pub fn send_particles(
        &self,
        to_render_particles: &Sender<(
            usize,
            [RenderParticle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
        )>,
    ) -> Result<()> {
        for part in self.parts.iter() {
            let mut particles =
                [RenderParticle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize];
            for (i, particle) in part.particles.iter().enumerate() {
                particles[i] = particle.into();
            }

            to_render_particles.send((part.id, particles))?;
        }

        Ok(())
    }

    pub fn send_transform(&self, to_render_transform: &Sender<(usize, Transform)>) -> Result<()> {
        for part in self.parts.iter() {
            to_render_transform.send((part.id, part.transform))?;
        }

        Ok(())
    }

    pub fn get_part_by_pos(&self, pos: IVec2) -> Option<&ChunkPart> {
        for p in &self.parts {
            if p.pos == pos {
                return Some(p);
            }
        }

        return None;
    }

    pub fn get_part_index_by_pos(&self, pos: IVec2) -> Option<usize> {
        for (i, p) in self.parts.iter().enumerate() {
            if p.pos == pos {
                return Some(i);
            }
        }

        return None;
    }

    pub fn get_part_by_pos_mut(&mut self, pos: IVec2) -> Option<&mut ChunkPart> {
        for p in &mut self.parts {
            if p.pos == pos {
                return Some(p);
            }
        }

        return None;
    }

    pub fn get_neigbor_particles_pos(
        &self,
        part_pos: IVec2,
        part_index: usize,
        hex: IVec2,
    ) -> Vec<Option<(IVec2, usize)>> {
        let mut neigbor_particles = Vec::new();

        for i in 0..6 {
            let res = self.get_neigbor_particle_pos(part_pos, part_index, hex, i);

            neigbor_particles.push(res);
        }

        neigbor_particles
    }

    pub fn get_neigbor_particles_pos_cleaned(
        &self,
        part_pos: IVec2,
        part_index: usize,
        hex: IVec2,
    ) -> Vec<(IVec2, usize)> {
        let mut neigbor_particles = Vec::new();

        for i in 0..6 {
            let res = self.get_neigbor_particle_pos(part_pos, part_index, hex, i);

            if res.is_none() {
                continue;
            }
            neigbor_particles.push(res.unwrap());
        }

        neigbor_particles
    }

    pub fn get_neigbor_particle_pos(
        &self,
        part_pos: IVec2,
        part_index: usize,
        hex: IVec2,
        neigbor_index: usize,
    ) -> Option<(IVec2, usize)> {
        let mut hex_neigbor = hex + neigbor_hex_offsets()[neigbor_index];
        let neigbor_part_index = match neigbor_index {
            0 => {
                if hex_neigbor.x < CHUNK_PART_SIZE {
                    Some(part_index)
                } else {
                    hex_neigbor.x -= CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(1, 0))
                }
            }
            1 => {
                if hex_neigbor.y < CHUNK_PART_SIZE {
                    Some(part_index)
                } else {
                    hex_neigbor.y -= CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(0, 1))
                }
            }
            2 => {
                if hex_neigbor.x >= 0 && hex_neigbor.y < CHUNK_PART_SIZE {
                    Some(part_index)
                } else {
                    let mut new_pos = part_pos;
                    if hex_neigbor.x < 0 {
                        hex_neigbor.x += CHUNK_PART_SIZE;
                        new_pos += ivec2(-1, 0);
                    }
                    if hex_neigbor.y >= CHUNK_PART_SIZE {
                        hex_neigbor.y -= CHUNK_PART_SIZE;
                        new_pos += ivec2(0, 1);
                    }

                    self.get_part_index_by_pos(new_pos)
                }
            }
            3 => {
                if hex_neigbor.x >= 0 {
                    Some(part_index)
                } else {
                    hex_neigbor.x += CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(-1, 0))
                }
            }
            4 => {
                if hex_neigbor.y >= 0 {
                    Some(part_index)
                } else {
                    hex_neigbor.y += CHUNK_PART_SIZE;
                    self.get_part_index_by_pos(part_pos + ivec2(0, -1))
                }
            }
            5 => {
                if hex_neigbor.x < CHUNK_PART_SIZE && hex_neigbor.y >= 0 {
                    Some(part_index)
                } else {
                    let mut new_pos = part_pos;
                    if hex_neigbor.x >= CHUNK_PART_SIZE {
                        hex_neigbor.x -= CHUNK_PART_SIZE;
                        new_pos += ivec2(1, 0);
                    }
                    if hex_neigbor.y < 0 {
                        hex_neigbor.y += CHUNK_PART_SIZE;
                        new_pos += ivec2(0, -1);
                    }

                    self.get_part_index_by_pos(new_pos)
                }
            }
            _ => None,
        };

        if neigbor_part_index.is_none()
            || self.parts[neigbor_part_index.unwrap()].particles[hex_to_particle_index(hex_neigbor)]
                .mass
                == 0
        {
            return None;
        }

        return Some((hex_neigbor, neigbor_part_index.unwrap()));
    }
}
