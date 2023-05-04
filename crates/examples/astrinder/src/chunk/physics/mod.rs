use std::time::{Instant};

use app::anyhow::*;
use app::glam::{Vec2};
use collision::algorithm::minkowski::GJK2;
use crate::chunk::CHUNK_PART_SIZE;
use crate::chunk::math::{world_pos_to_hex, };
use crate::chunk::physics::collide::CollisionSearch;
use crate::settings::Settings;

use self::collide::CollisionSearchResult;

use super::math::vector2_to_vec2;
use super::{ChunkController, transform::Transform, math::{point2_to_vec2, cross2d}};

mod collide;
pub mod destruction;

impl ChunkController {
    pub fn update_physics(&mut self, time_step: f32, settings: Settings) -> Result<()> {
        
        let l = self.chunks.len();

        // Gravity
        if settings.gravity_on && l >= 2 {
            for i in 0..l {
                for j in (i+1)..l {
                    let chunk0 = &self.chunks[i];
                    let chunk1 = &self.chunks[j];
    
                    let gravity_force = get_gravity_force(
                        chunk0.transform.pos, 
                        chunk1.transform.pos, 
                        chunk0.mass, 
                        chunk1.mass, 
                        settings);

                    let vel0 = gravity_force / -chunk0.mass;
                    let vel1 = gravity_force / chunk1.mass;
                    
                    self.chunks[i].velocity_transform.pos += vel0;
                    self.chunks[j].velocity_transform.pos += vel1;
                }
            }
        }

        let to_render_transform = self.to_render_transform.clone();
        let to_render_particles = self.to_render_particles.clone();
        let mut collision_search = CollisionSearch::new(settings, &mut self.chunks, time_step);
       
        // https://research.ncl.ac.uk/game/mastersdegree/gametechnologies/physicstutorials/5collisionresponse/Physics%20-%20Collision%20Response.pdf
        loop {
            match collision_search.get_next(&self.chunks, time_step) {
                CollisionSearchResult::Done => break,

                CollisionSearchResult::Contact((contact, chunk0_index, chunk1_index)) => {
                    let chunk0 = &self.chunks[chunk0_index];
                    let chunk1 = &self.chunks[chunk1_index];

                    let last_vel0 = chunk0.last_velocity_transform;
                    let last_vel1 = chunk1.last_velocity_transform;
        
                    let point = point2_to_vec2(contact.contact_point);
                    let normal = vector2_to_vec2(contact.normal);

                    if normal.is_nan() {
                        continue;
                    }

                    let totalMass = chunk0.inverse_mass + chunk1.inverse_mass;
                    let penetration = 1.0 - contact.time_of_impact;

                    let offset0 = -normal * penetration * (chunk0.inverse_mass / totalMass);
                    let offset1 = -normal * penetration * (chunk1.inverse_mass / totalMass);

                    self.chunks[chunk0_index].transform.pos += offset0;
                    self.chunks[chunk1_index].transform.pos += offset1;




                    // Collision Response
                    //self.chunks[chunk0_index].velocity_transform += last_vel0 * -2.0;

                    //self.chunks[chunk1_index].velocity_transform += last_vel1 * -2.0;
        
                },

                CollisionSearchResult::ChunkDone(i) => {
                    let chunk = &mut self.chunks[i];

                    chunk.velocity_transform.rot *= settings.rotation_damping;

                    chunk.transform += chunk.velocity_transform * time_step;
                    chunk.last_velocity_transform = chunk.velocity_transform;

                    chunk.on_transform_change();
                    chunk.send_transform(&to_render_transform)?;
                },
            }           
        }

        /* 
        if !settings.destruction_on {
            return Ok(());
        }

        let old_chunks = self.chunks.clone();
        for entry in collision_search.log.iter() {
            let chunk0 = &old_chunks[entry.chunk0_index];
            let chunk1 = &old_chunks[entry.chunk1_index];

            let part0 = &chunk0.parts[entry.parts[0].0];
            let part1 = &chunk1.parts[entry.parts[0].1];
            
            let point = entry.points[0];
            let normal = entry.normals[0];

            let hex0 = world_pos_to_hex(part0.transform, point - normal * 0.5) + part0.pos * CHUNK_PART_SIZE;
            let hex1 = world_pos_to_hex(part1.transform, point + normal * 0.5) + part1.pos * CHUNK_PART_SIZE;

            let now = Instant::now();

            let vel_diff = (chunk0.velocity_transform.pos- chunk1.velocity_transform.pos).length();
            let break_val_0 = (chunk0.mass / chunk1.mass) / vel_diff; 
            let break_val_1 = (chunk1.mass / chunk0.mass) / vel_diff; 

            let break0 = chunk0.particle_counter > 1 
                && break_val_0 < 20.
                && now.duration_since(chunk0.break_cool_down) > settings.destruction_cool_down;

            let break1 = chunk1.particle_counter > 1 
                && break_val_1 < 20.
                && now.duration_since(chunk1.break_cool_down) > settings.destruction_cool_down;

            assert!(entry.chunk0_index < entry.chunk1_index);
            if break1 {
                self.remove_chunk(entry.chunk1_index);
            }
            if break0 {
                self.remove_chunk(entry.chunk0_index);
            }

            if break0 {
                let mut new_chunks_0 = self.destruction_solver.patterns[0]
                    .apply_to_chunk(chunk0, hex0, &mut self.part_id_counter, settings);

                for c in new_chunks_0.iter() {
                    c.send(&to_render_transform, &to_render_particles)?;
                }

                self.chunks.append(&mut new_chunks_0);
            }

            if break1 {
                let mut new_chunks_1 = self.destruction_solver.patterns[0]
                    .apply_to_chunk(chunk1, hex1, &mut self.part_id_counter, settings);

                for c in new_chunks_1.iter() {
                    c.send(&to_render_transform, &to_render_particles)?;
                }

                self.chunks.append(&mut new_chunks_1);
            }
        }
        */

        Ok(())
    }
}


fn get_gravity_force(pos0: Vec2, pos1: Vec2, mass0: f32, mass1: f32, settings: Settings) -> Vec2 {
    let diff = pos0 - pos1;
    let dist = diff.length();
    let force = f32::min((settings.gravity_factor * mass0 * mass1) / (dist * dist), settings.gravity_max_force);

    diff * (1.0 / dist) * force
}





