use app::glam::Vec2;
use cgmath::{Decomposed, Rotation2, Vector2, Rad, Basis2};
use collision::{algorithm::minkowski::GJK2, CollisionStrategy};

use crate::aabb::AABB;

use super::{ChunkController, Chunk, transform::Transform, math::part_corners, ChunkPart};

const GRAVITY_G: f32 = 0.1;
const GRAVITY_MAX_FORCE: f32 = 10.0;

impl ChunkController {
    pub fn update_physics(&mut self, time_step: f32) {
        let mut accelerations = vec![Transform::default(); self.chunks.len()];
        let l = self.chunks.len();

        // Gravity
        for (i, chunk) in self.chunks.iter().enumerate() {
            for j in (i+1)..l {
                let other_chunk = &self.chunks[j];

                let gravity_force = get_gravity_force(
                    chunk.transform.pos, 
                    other_chunk.transform.pos, 
                    chunk.mass, 
                    other_chunk.mass);
                
                accelerations[i].pos -= gravity_force / chunk.mass;
                accelerations[j].pos += gravity_force / other_chunk.mass;
            }
        }

        // Collision
        let chunks = self.get_colliding_chunks();
        for (chunk0, chunk1) in chunks {
            if chunk0.is_colliding(chunk1) {
                println!("Collision");
            }
        }

        for (i, chunk) in self.chunks.iter_mut().enumerate() {
            chunk.velocity_transform += accelerations[i];

            chunk.apply_velocity(time_step)
        }
    }

    fn get_colliding_chunks(&self) -> Vec<(&Chunk, &Chunk)> {
        let mut res = Vec::new();

        let l = self.chunks.len();
        for (i, chunk) in self.chunks.iter().enumerate() {
            for j in (i+1)..l {
                let other_chunk = &self.chunks[j];

                if chunk.aabb.collides(other_chunk.aabb) {
                    res.push((chunk, other_chunk));
                }
            }
        }

        res
    }
}

fn get_gravity_force(pos0: Vec2, pos1: Vec2, mass0: f32, mass1: f32) -> Vec2 {
    let diff = pos0 - pos1;
    let dist = diff.length();
    let force = f32::min((GRAVITY_G * mass0 * mass1) / (dist * dist), GRAVITY_MAX_FORCE);

    diff * (1.0 / dist) * force
}

impl Chunk {
    fn apply_velocity(&mut self, time_step: f32) {
        
        self.transform += self.velocity_transform * time_step;

        self.on_transform_update()
    }

    fn is_colliding<'a>(&'a self, other: &'a Chunk) -> bool {
        let gjk = GJK2::new();

        let l1 = other.parts.len();
        let mut aabbs1 = vec![None; l1];

        let part_offset = part_corners()[3];
        for part in self.parts.iter() {
            let aabb0 = AABB::new(part.transform.pos, part.transform.pos + part_offset);

            for (i, other_part) in other.parts.iter().enumerate() {
                if !aabbs1[i].is_some() { 
                    let new_aabb1 = AABB::new(other_part.transform.pos, other_part.transform.pos + part_offset);
                    aabbs1[i] = Some(new_aabb1);
                } 
                let aabb1 = aabbs1[i].unwrap();

                if aabb0.collides(aabb1) {

                    let t0 = transform(part.transform);
                    let t1 = transform(other_part.transform);

                    let result = gjk.intersect(
                        &part.collider, 
                        &t0, 
                        &other_part.collider, 
                        &t1);
                    
                    if result.is_some() {
                        return true;
                    }
                }
            } 
        }

        false
    }
}

fn transform(t: Transform) -> Decomposed<Vector2<f32>, Basis2<f32>> {
    Decomposed {
        disp: Vector2::new(t.pos.x, t.pos.y),
        rot: Rotation2::from_angle(Rad(t.rot)),
        scale: 1.,
    }
}