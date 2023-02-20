use super::math::clamp;

use cgmath::{vec3, Vector3, Matrix4, Deg};

#[derive(Clone, Copy)]
pub struct Camera {
    pos: Vector3<f32>,
    pitch: f32,
    max_pitch: f32,
    yaw: f32,
}

impl Camera {
    pub fn rotate(&mut self, theta: f32, phi: f32) {
        self.yaw += theta;
        self.pitch += phi;
    }

    pub fn forward(&mut self, r: f32) {
       self.pos.z += r
    }

    pub fn matrix(&mut self) -> Matrix4<f32>{
        return Matrix4::from_axis_angle(vec3(0.0, 1.0, 0.0),Deg(self.yaw)) * 
                Matrix4::from_angle_x(Deg(self.pitch)) * 
                Matrix4::from_translation(self.pos);
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            pos: Vector3 { x: 0.0, y: 0.0, z: -10.0 },
            pitch: 10.0,
            max_pitch: 85.0,
            yaw: 0.0,
        }
    }
}
