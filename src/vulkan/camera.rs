use super::math::clamp;

use cgmath::{Deg, Matrix4, Vector3};

#[derive(Clone, Copy)]
pub struct Camera {
    pub pos: Vector3<f32>,
    pub dir: Vector3<f32>,
}

impl Camera {
    pub fn forward(&mut self, r: f32) {
        self.pos.z += r;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            pos: Vector3{x: 0.0, y: 0.0, z: -2.0},
            dir: Vector3{x: 0.0, y: 0.0, z: 1.0},
        }
    }
}