use std::time::Duration;

use app::glam::Vec2;

use crate::math::transform::Transform;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    // Render
    pub max_fps: u32,
    pub max_chunks: usize,
    pub max_rendered_parts: usize,

    // Camera
    pub camera_inital_transform: Transform,
    pub camera_inital_scale: f32,
    pub camera_speed: f32,
    pub camera_scale_factor: f32,

    // Chunk
    pub max_chunk_ups: u32,
    pub chunk_ups_use_fixed_time_step: bool,
    pub chunk_ups_fixed_time_step: f32,

    pub slow_down_chunk_ups_factor: u32,

    pub gravity_on: bool,
    pub gravity_factor: f32,
    pub gravity_max_force: f32,

    pub collision_on: bool,

    pub destruction_on: bool,
    pub destruction_force_factor: f32,

    // Debug
    pub max_lines: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_fps: 60,
            max_chunks: 10000,
            max_rendered_parts: 5000,

            camera_inital_transform: Transform::default(),
            camera_inital_scale: 0.03,
            camera_speed: 20.0,
            camera_scale_factor: 0.01,

            max_chunk_ups: 30,
            chunk_ups_use_fixed_time_step: true,
            chunk_ups_fixed_time_step: 1.0 / 30.0,
            slow_down_chunk_ups_factor: 10,

            gravity_on: true,
            gravity_factor: 0.01,
            gravity_max_force: 0.05,

            collision_on: true,

            destruction_on: false,
            destruction_force_factor: 0.000002,

            max_lines: 10_000,
        }
    }
}
