use app::glam::{UVec2, Vec2, IVec2, uvec2, vec2, ivec2};
use cgmath::Point2;
use collision::primitive::ConvexPolygon;
use crate::{aabb::AABB, chunk::math::*};
use geo::{line_string, polygon, Polygon, LineString};
use geo::ConvexHull;

use self::{particle::{Particle}, transform::Transform};

pub mod math;
pub mod particle;
pub mod shapes;
pub mod render;
pub mod physics;
pub mod transform;

const CHUNK_PART_SIZE: i32 = 10;

pub struct ChunkController {
    pub chunks: Vec<Chunk>
}

impl ChunkController {
    pub fn new() -> Self {
        let mut chunks = Vec::new();

        chunks.push(Chunk::new_hexagon(
            Transform::new(vec2(0.0, 0.0), 0.0), 
            Transform::new(vec2(0., 0.), 0.0),
            10));
        chunks.push(Chunk::new_hexagon(
            Transform::new(vec2(30.0, 0.0), 0.0), 
            Transform::new(vec2(0.0, -20.0), 0.0),
            2));

        Self { 
            chunks: chunks 
        }
    }
}

pub struct Chunk { 
    pub parts: Vec<ChunkPart>, 

    pub mass: f32,

    pub aabb: AABB,
    particle_max_dist_to_transform: Vec2,
    
    pub transform: Transform,
    pub render_to_transform: Vec2,

    particle_counter: usize,
    particle_pos_sum: Vec2,
   
    pub velocity_transform: Transform,
}

#[allow(dead_code)]
impl Chunk {
    pub fn new(transform: Transform, velocity_transform: Transform, particles: Vec<(Particle, IVec2)>) -> Self {
        let mut chunk = Self { 
            parts: Vec::new(),
            mass: 0.0,

            aabb: AABB::default(),
            particle_max_dist_to_transform: Vec2::ZERO,
           
            transform: transform,
            render_to_transform: Vec2::ZERO,

            particle_counter: 0,
            particle_pos_sum: Vec2::ZERO,

            velocity_transform: velocity_transform,
        };

        for (p, hex_pos) in particles {
            chunk.add_particle(p, hex_pos)
        }

        for part in chunk.parts.iter_mut() {
            part.update_collider();
        }

        chunk.render_to_transform = chunk.particle_pos_sum / Vec2::new(
            chunk.particle_counter as f32, 
            chunk.particle_counter as f32);


        chunk.on_transform_update();

        chunk
    }

    pub fn add_particle(
        &mut self, 
        p: Particle, 
        hex_pos: IVec2,
    ){
        let part_pos = hex_to_chunk_part_pos(hex_pos);
        
        let mut part = None;
        for p in &mut self.parts {
            if p.pos == part_pos  {
                part = Some(p);
            }
        }

        if part.is_none() {
            let new_part= ChunkPart::new(part_pos);

            self.parts.push(new_part);

            part = self.parts.last_mut();
        }
        debug_assert!(part.is_some());

        let in_part_pos = hex_to_in_chunk_part_pos(hex_pos);
        part.unwrap().particles[in_part_pos] = p;
        
        self.mass += p.mass as f32;

        // Needed for center of mass
        let particle_pos = hex_to_coord(hex_pos);
        self.particle_pos_sum += particle_pos;

        // Needed for AABB
        let particle_pos_abs = particle_pos.abs();
        self.particle_max_dist_to_transform.x = if self.particle_max_dist_to_transform.x < particle_pos_abs.x { 
            particle_pos_abs.x } else { self.particle_max_dist_to_transform.x };
        self.particle_max_dist_to_transform.y = if self.particle_max_dist_to_transform.y < particle_pos_abs.y { 
                particle_pos_abs.y } else { self.particle_max_dist_to_transform.y };

        self.particle_counter += 1;
    }

    fn on_transform_update (&mut self) {
        self.aabb = AABB::new(
            self.transform.pos - self.particle_max_dist_to_transform, 
            self.transform.pos + self.particle_max_dist_to_transform);

        self.update_part_tranforms();
    }

    pub fn update_part_tranforms(&mut self){
        for part in self.parts.iter_mut() {
            part.transform = part_pos_to_world(self.transform, part.pos, self.render_to_transform);
        }
    }
}



#[derive(Clone)]
pub struct ChunkPart{
    pub pos: IVec2,
    pub particles: [Particle; (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
    pub transform: Transform,

    pub colliders: Vec<ConvexPolygon<f32>>
}

impl ChunkPart {
    pub fn new(pos: IVec2) -> Self {
        Self { 
            pos: pos,
            particles: [Particle::default(); (CHUNK_PART_SIZE * CHUNK_PART_SIZE) as usize],
            transform: Transform::default(),

            colliders: Vec::new(),
        }
    }   

    pub fn update_collider(&mut self) {

        let mut outside_particles = Vec::new();

        let offsets = [
            ( 1,  0),
            ( 0,  1),
            (-1,  1),
            (-1,  0),
            ( 0, -1),
            ( 1, -1),
        ];

        for i in 0..CHUNK_PART_SIZE {
            for j in 0..CHUNK_PART_SIZE {
                let index = (i * CHUNK_PART_SIZE + j) as usize;
                let material = self.particles[index].material;

                if material == 0 {
                    continue;
                }

                let mut outside =  i == 0 || i == CHUNK_PART_SIZE - 1 || j == 0 || j == CHUNK_PART_SIZE - 1;
                if !outside {

                    for (x, y) in offsets.iter() {
                        let other_index = ((i + x) * CHUNK_PART_SIZE + j + y) as usize;
                        let other_material = self.particles[other_index].material;

                        if other_material == 0 {
                            outside = true;
                            break;
                        }
                    }

                    if !outside {
                        continue;
                    }
                }

                let pos = hex_to_coord(ivec2(i, j));
                outside_particles.push((pos.x, pos.y));
            }
        }

        let polygon = Polygon::new(
            LineString::from(outside_particles),
            vec![],
        );

        self.colliders.clear();
        for trinagle in polygon.exterior().triangles() {
            let points = trinagle.to_array(); 

            self.colliders.push(ConvexPolygon::new(vec![
                Point2::<f32>::new(points[0].x, points[0].y),
                Point2::<f32>::new(points[1].x, points[1].y),
                Point2::<f32>::new(points[2].x, points[2].y)]));
        }        
    }
}




