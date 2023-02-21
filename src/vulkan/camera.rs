use super::{transform::Transform};

use cgmath::{Vector3, Matrix4, Deg, Euler, Vector2};

#[derive(Clone, Copy)]
pub struct Camera {
    pub transform: Transform,

    max_pitch: f32,
    rot_speed: Vector2<f32>
}

impl Camera {
    fn check_rotation(&mut self){

        let mut rotation = self.transform.get_rotation();
        if rotation[0] > self.max_pitch {
            rotation[0] = self.max_pitch;
        }
        else if  rotation[0] < -self.max_pitch {
            rotation[0] = -self.max_pitch;
        }

        if rotation[1] >= 360.0 {
            rotation[1] -= 360.0
        }
        else if  rotation[1] < 0.0 {
            rotation[1] += 360.0;
        }

        self.transform.set_rotation(rotation);
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            transform: Transform::default().set_position(Vector3 { x: 0.0, y: 2.0 , z: -2.0 }),
            max_pitch: 85.0,
            rot_speed: Vector2 { x: 0.1, y: 0.1 }

        }
    }
}
