use super::{transform::Transform};

use cgmath::{Vector3, Matrix4, Deg, Euler, Vector2};

#[derive(Clone, Copy)]
pub struct Camera {
    transform: Transform,

    max_pitch: f32,
    rot_speed: Vector2<f32>
}

impl Camera {
    pub fn rotate(&mut self, pitch: f32, yaw: f32) {
        self.rot[0] += pitch * self.rot_speed[0];
        self.rot[1] += yaw * self.rot_speed[1];

        self.check_rotation();
    }

    pub fn set_rotation(&mut self, pitch: f32, yaw: f32) {
        self.rot[0] = pitch;
        self.rot[1] = yaw;

        self.check_rotation();
    }

    fn check_rotation(&mut self){
        if self.rot[0] > self.max_pitch {
            self.rot[0] = self.max_pitch;
        }
        else if  self.rot[0] < -self.max_pitch {
            self.rot[0] = -self.max_pitch;
        }

        if self.rot[1] >= 360.0 {
            self.rot[1] -= 360.0
        }
        else if  self.rot[1] < 0.0 {
            self.rot[1] += 360.0;
        }
    }

    pub fn translate(&mut self, dir:Vector3<f32>){
        let rot_dir = 
        self.pos += dir;
    }

    pub fn set_position(&mut self, pos:Vector3<f32>){
        self.pos = pos
    }

    pub fn matrix(&mut self) -> Matrix4<f32>{
        return Matrix4::from(Euler {x: Deg(self.rot[0]), y: Deg(self.rot[1]), z: Deg(0.0)}) * Matrix4::from_translation(self.pos);
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            pos: Vector3 { x: 0.0, y: -2.0, z: -2.0 },
            rot: Vector2 { x: 0.0, y: 0.0 },
            max_pitch: 85.0,
            rot_speed: Vector2 { x: 0.1, y: 0.1 }

        }
    }
}
