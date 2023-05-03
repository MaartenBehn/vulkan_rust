use std::cmp::min;

use app::glam::Vec2;
use cgmath::{Point2, Decomposed, Vector2, Basis2, Rotation2, Rad};
use collision::{Contact, algorithm::minkowski::GJK2};
use crate::chunk::math::part_pos_to_world;

use crate::{chunk::{math::{part_corners, point2_to_vec2, vector2_to_vec2}, chunk::Chunk, transform::Transform, part::ChunkPart}, aabb::AABB, settings::Settings};

pub enum CollisionSearchResult {
    Done,
    Contact(Contact<Point2<f32>>),
    ChunkDone(usize),
}

pub struct CollisionSearch{
    pub gjk: GJK2<f32>,
    pub chunk0_index: usize,
    pub chunk1_index: usize,
    pub chunk0_transform: Decomposed<Vector2<f32>, Basis2<f32>>,
    pub chunk0_next_transform: Decomposed<Vector2<f32>, Basis2<f32>>,
    pub time_of_first_collide: Vec<f32>,
}

impl CollisionSearch{
    pub fn new(settings: Settings, chunks: &Vec<Chunk>, time_step: f32) -> Self {
        Self { 
            gjk: GJK2::new(),
            chunk0_index: 0, 
            chunk1_index: 1, 
            chunk0_transform: chunks[0].transform.into(),
            chunk0_next_transform: (chunks[0].transform + chunks[0].velocity_transform * time_step).into(),
            time_of_first_collide: vec![1.0; chunks.len()],
        }
    }

    pub fn get_next(&mut self, chunks: &Vec<Chunk>, time_step: f32) -> CollisionSearchResult {

        if self.chunk0_index >= chunks.len() - 1 { return CollisionSearchResult::Done; }

        let chunk0 = &chunks[self.chunk0_index];

        while self.chunk1_index < chunks.len() {
            
            let chunk1 = &chunks[self.chunk1_index];

            if chunk0.aabb.collides(chunk1.aabb) {
                
                let chunk1_transform: Decomposed<Vector2<f32>, Basis2<f32>> = chunks[self.chunk1_index].transform.into();
                let chunk1_next_transform: Decomposed<Vector2<f32>, Basis2<f32>> = (chunks[self.chunk1_index].transform + 
                    chunks[self.chunk1_index].velocity_transform * time_step).into();

                let result = self.gjk.intersection_complex_time_of_impact(
                    &collision::CollisionStrategy::CollisionOnly, 
                    &chunk0.colliders,
                    &self.chunk0_transform..&self.chunk0_next_transform,
                    &chunk1.colliders,
                    &chunk1_transform..&chunk1_next_transform
                );

                if result.is_some() {
                    let res = result.unwrap();

                    let time_of_impact = res.time_of_impact;

                    if self.time_of_first_collide[self.chunk0_index] > time_of_impact {
                        self.time_of_first_collide[self.chunk0_index] = time_of_impact;
                    }

                    if self.time_of_first_collide[self.chunk1_index] > time_of_impact {
                        self.time_of_first_collide[self.chunk1_index] = time_of_impact;
                    }

                    self.chunk1_index += 1;

                    return CollisionSearchResult::Contact(res);
                }
            }
            
            self.chunk1_index += 1;
            
        }

        self.chunk0_index += 1;
        self.chunk1_index = self.chunk0_index + 1;

        if self.chunk0_index >= chunks.len() - 1 { return CollisionSearchResult::Done; }

        self.chunk0_transform = chunks[self.chunk0_index].transform.into();
        self.chunk0_next_transform = (chunks[self.chunk0_index].transform + chunks[self.chunk0_index].velocity_transform * time_step).into();

        CollisionSearchResult::ChunkDone(self.chunk0_index - 1)
    }
}