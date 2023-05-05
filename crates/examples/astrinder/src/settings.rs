use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    // Render
    pub max_fps: u32,
    pub max_chunks: usize,
    pub max_rendered_parts: usize,

    // Chunk
    pub max_chunk_ups: u32,
    pub chunk_ups_use_fixed_time_step: bool,
    pub chunk_ups_fixed_time_step: f32,

    pub slow_down_chunk_ups_factor: u32,

    // Physics
    pub rotation_damping: f32,

    pub gravity_on: bool,
    pub gravity_factor: f32,
    pub gravity_max_force: f32,

    pub collision_on: bool,

    pub destruction_on: bool,
    pub min_destruction_force: f32,

    // Debug
    pub max_lines: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_fps: 60,
            max_chunks: 10000,
            max_rendered_parts: 5000,

            max_chunk_ups: 30,
            chunk_ups_use_fixed_time_step: true,
            chunk_ups_fixed_time_step: 1.0 / 30.0,
            slow_down_chunk_ups_factor: 10,

            rotation_damping: 0.9,

            gravity_on: true,
            gravity_factor: 0.01,
            gravity_max_force: 0.05,

            collision_on: true,

            destruction_on: false,
            min_destruction_force: 10000.0,

            max_lines: 10_000,
        }
    }
}
