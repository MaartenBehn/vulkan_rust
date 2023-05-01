use app::glam::Vec2;
use cgmath::{Point2, Decomposed, Vector2, Basis2, Rotation2, Rad};
use collision::{Contact, algorithm::minkowski::GJK2, CollisionStrategy};

use crate::{chunk::{math::{part_corners, point2_to_vec2, vector2_to_vec2}, chunk::Chunk, transform::Transform, MAX_AMMOUNT_OF_PARTS, part::ChunkPart}, aabb::AABB};

#[derive(Clone)]
pub struct CollisionSearch{
    pub chunk0_index: usize,
    pub chunk1_index: usize,
    last_chunk0_index: usize,
    last_chunk1_index: usize,

    pub part0_index: usize,
    pub part1_index: usize,
    part_offset: Vec2,

    pub collider0_index: usize,
    pub collider1_index: usize,

    pub log: Vec<CollisionLogEntry>,
}

#[derive(Clone)]
pub struct CollisionLogEntry {
    pub chunk0_index: usize,
    pub chunk1_index: usize,

    pub parts: Vec<(usize, usize)>,
    pub points: Vec<Vec2>,
    pub normals: Vec<Vec2>,
}


impl CollisionSearch{
    pub fn new() -> Self {
        Self { 
            chunk0_index:       0, 
            chunk1_index:       1, 
            last_chunk0_index:  MAX_AMMOUNT_OF_PARTS,
            last_chunk1_index:  MAX_AMMOUNT_OF_PARTS,
            part0_index:        0, 
            part1_index:        0, 
            part_offset:        part_corners()[3], 
            collider0_index:    0, 
            collider1_index:    0, 
            log:                Vec::new(),
        }
    }

    pub fn get_next_collision(&mut self, chunks: &Vec<Chunk>) -> Option<Contact<Point2<f32>>> {
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
            if self.collider0_index >= part0.colliders.len() { break; }

            let collider0 = &part0.colliders[self.collider0_index];
            
            loop {
                if self.collider1_index >= part1.colliders.len() { break; }

                let collider1 = &part1.colliders[self.collider1_index];
  

                let result = gjk.intersection(
                    &CollisionStrategy::FullResolution,
                    collider0, 
                    &t0, 
                    collider1, 
                    &t1);

                self.collider1_index += 1;

                if result.is_some() {
                    let res = result.unwrap();
                    
                    if self.last_chunk0_index != self.chunk0_index || self.last_chunk1_index != self.chunk1_index {
                        self.log.push(CollisionLogEntry{
                            chunk0_index: self.chunk0_index,
                            chunk1_index: self.chunk1_index,
                            parts: Vec::new(),
                            points: Vec::new(),
                            normals: Vec::new(),
                        });
                    }

                    let entry = self.log.last_mut().unwrap();

                    entry.parts.push((self.part0_index, self.part1_index));
                    entry.points.push(point2_to_vec2(res.contact_point));
                    entry.normals.push(vector2_to_vec2(res.normal));

                    self.last_chunk0_index = self.chunk0_index;
                    self.last_chunk1_index = self.chunk1_index;

                    return Some(res);
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

