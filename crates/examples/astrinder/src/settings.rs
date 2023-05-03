use std::time::Duration;


#[derive(Clone, Copy, Debug)]
pub struct Settings {
    // Render
    pub max_fps: u32,
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
    pub destruction_cool_down: Duration,

    // Debug
    pub max_lines: usize,
}


impl Default for Settings {
    fn default() -> Self {
        Self { 
            max_fps: 60, 
            max_rendered_parts: 10_000, 

            max_chunk_ups: 120, 
            chunk_ups_use_fixed_time_step: true, 
            chunk_ups_fixed_time_step: 1.0 / 120.0, 
            slow_down_chunk_ups_factor: 10, 

            rotation_damping: 0.9,

            gravity_on: true, 
            gravity_factor: 0.01, 
            gravity_max_force: 1.0, 

            collision_on: true, 

            destruction_on: false, 
            destruction_cool_down: Duration::from_secs(1),

            max_lines: 1_000_000
        }
    }
}
