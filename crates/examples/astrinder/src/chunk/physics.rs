use app::glam::{Vec2, vec2};
use app::anyhow::*;
use cgmath::{Decomposed, Rotation2, Vector2, Rad, Basis2, Point2};
use collision::{algorithm::minkowski::GJK2, CollisionStrategy, Contact};

use crate::aabb::AABB;

use super::math::vector2_to_vec2;
use super::{ChunkController, Chunk, transform::Transform, math::{part_corners, point2_to_vec2, cross2d}, ChunkPart};

const GRAVITY_G: f32 = 0.01;
const GRAVITY_MAX_FORCE: f32 = 1.0;
const GRAVITY_ON: bool = false;



impl ChunkController {
    pub fn update_physics(&mut self, time_step: f32) {
        let mut accelerations = vec![Transform::default(); self.chunks.len()];
        let l = self.chunks.len();


        // Gravity
        if GRAVITY_ON {
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
        }

        for (i, chunk) in self.chunks.iter_mut().enumerate() {
            chunk.velocity_transform += accelerations[i];
            chunk.transform += chunk.velocity_transform * time_step;
            chunk.on_transform_change();
        }

        // Collision
        assert!(self.chunks.len() >= 2);
        let mut collision_search = CollisionSearch::new();

        loop {
            let result = collision_search.get_next_collision(&self.chunks);
            if result.is_none() {
                break;
            }

            let chunk0 = &self.chunks[collision_search.chunk0_index];
            let chunk1 = &self.chunks[collision_search.chunk1_index];

            let res = result.unwrap();
            let normal = vector2_to_vec2(res.normal).normalize();
            let point = point2_to_vec2(res.contact_point);

            // Resolve collsion
            let mass0_fraction = chunk0.mass / (chunk0.mass + chunk1.mass);
            let mass1_fraction = 1.0 - mass0_fraction;

            // Collision Response
            let r_a = point - chunk0.transform.pos;
            let r_b = point - chunk1.transform.pos;
            let r_a_cross_n = cross2d(r_a, normal);
            let r_b_cross_n = cross2d(r_a, normal);
            let r_a_cross_n_2 = r_a_cross_n.powf(2.0);
            let r_b_cross_n_2 = r_b_cross_n.powf(2.0);

            let c = 0.0;

            let j = (-1.0 - c) 
                     * (chunk0.velocity_transform.pos.dot(normal) - chunk1.velocity_transform.pos.dot(normal)
                      + chunk0.velocity_transform.rot * r_a_cross_n 
                      - chunk1.velocity_transform.rot * r_b_cross_n)
                     / (1.0 / chunk0.mass + 1.0 / chunk1.mass + r_a_cross_n_2)
                     / chunk0.moment_of_inertia 
                    + r_b_cross_n_2 / chunk1.moment_of_inertia;

            if j < 0.0 {
                continue;
            }

            let j_vec = normal * j;

            let vel0 = j_vec / chunk0.mass;
            let rot_vel0 = cross2d(r_a, j_vec) / chunk0.moment_of_inertia;

            let vel1 = j_vec / chunk1.mass;
            let rot_vel1 = cross2d(r_b, j_vec) / chunk1.moment_of_inertia;

            // Resolve collsion
            self.chunks[collision_search.chunk0_index].transform.pos -= normal * mass1_fraction * res.penetration_depth;
            self.chunks[collision_search.chunk1_index].transform.pos += normal * mass0_fraction * res.penetration_depth;

            // Collision Response
            self.chunks[collision_search.chunk0_index].velocity_transform.pos += vel0;
            self.chunks[collision_search.chunk0_index].velocity_transform.rot += rot_vel0;

            self.chunks[collision_search.chunk1_index].velocity_transform.pos -= vel1;
            self.chunks[collision_search.chunk1_index].velocity_transform.rot -= rot_vel1;
        }

        for chunk in self.chunks.iter_mut() {
            chunk.on_transform_change();
        }
    }
}

fn get_gravity_force(pos0: Vec2, pos1: Vec2, mass0: f32, mass1: f32) -> Vec2 {
    let diff = pos0 - pos1;
    let dist = diff.length();
    let force = f32::min((GRAVITY_G * mass0 * mass1) / (dist * dist), GRAVITY_MAX_FORCE);

    diff * (1.0 / dist) * force
}


#[derive(Clone)]
struct CollisionSearch{
    chunk0_index: usize,
    chunk1_index: usize,

    part0_index: usize,
    part1_index: usize,
    part_offset: Vec2,

    collider0_index: usize,
    collider1_index: usize,
}



struct CollisionSearchResult {
    chunk0: usize,
    chunk1: usize,

    part0: usize,
    part1: usize,

    collider0: usize,
    collider1: usize,
}

impl CollisionSearch{
    fn new() -> Self {
        Self { 
            chunk0_index:   0, 
            chunk1_index:   1, 
            part0_index:    0, 
            part1_index:    0, 
            part_offset:    part_corners()[3], 
            collider0_index: 0, 
            collider1_index: 0, 
        }
    }

    fn get_next_collision(&mut self, chunks: &Vec<Chunk>) -> Option<Contact<Point2<f32>>> {
        self.get_next_broad_chunk_aabb_collision(chunks)
    }

    fn get_next_broad_chunk_aabb_collision(&mut self, chunks: &Vec<Chunk>) -> Option<Contact<Point2<f32>>> {
        loop {
            let chunk0 = &chunks[self.chunk0_index];

            loop {
                let chunk1 = &chunks[self.chunk1_index];

                if chunk0.aabb.collides(chunk1.aabb) {
                    let res = self.get_next_part_aabb_collision(chunk0, chunk1);
                    if res.is_some() {
                        return res;
                    }
                }

                self.chunk1_index += 1;

                if self.chunk1_index >= chunks.len() { break; }
            }

            self.chunk0_index += 1;
            self.chunk1_index = self.chunk0_index + 1;

            if self.chunk0_index >= chunks.len() - 1 { break; }
        }

        None
    }


    fn get_next_part_aabb_collision(&mut self, chunk0: &Chunk, chunk1: &Chunk) -> Option<Contact<Point2<f32>>> {
        loop  {

            let part0 = &chunk0.parts[self.part0_index];
            let aabb0 = AABB::new(part0.transform.pos, part0.transform.pos + self.part_offset);
            loop {

                let part1 = &chunk1.parts[self.part1_index];
                let aabb1 = AABB::new(part1.transform.pos, part1.transform.pos + self.part_offset);
                if aabb0.collides(aabb1) {
                    let res = self.get_next_collider_collision(part0, part1);
                    if res.is_some() {
                        return res;
                    }
                }

                self.part1_index += 1;

                if  self.part1_index >= chunk1.parts.len() { break; }
            }
            self.part1_index = 0;


            self.part0_index += 1;

            if self.part0_index >= chunk0.parts.len() { break; }
        }
        self.part0_index = 0;
        
        None
    }

    fn get_next_collider_collision(&mut self, part0: &ChunkPart, part1: &ChunkPart) -> Option<Contact<Point2<f32>>> {

        fn transform(t: Transform) -> Decomposed<Vector2<f32>, Basis2<f32>> {
            Decomposed {
                disp: Vector2::new(t.pos.x, t.pos.y),
                rot: Rotation2::from_angle(Rad(-t.rot)),
                scale: 1.,
            }
        }

        let gjk = GJK2::new();

        let t0 = transform(part0.transform);
        let t1 = transform(part1.transform);
        
        loop {
            let collider0 = &part0.colliders[self.collider0_index];
            
            loop {
                if self.collider1_index >= part1.colliders.len() {break;}

                let collider1 = &part1.colliders[self.collider1_index];
  

                let result = gjk.intersection(
                    &CollisionStrategy::FullResolution,
                    collider0, 
                    &t0, 
                    collider1, 
                    &t1);

                self.collider1_index += 1;

                if result.is_some() {
                    return result;
                }
            }
            self.collider1_index = 0;

            self.collider0_index += 1;

            if self.collider0_index >= part0.colliders.len() {break;}
        }
        self.collider0_index = 0;

        None
    }
}