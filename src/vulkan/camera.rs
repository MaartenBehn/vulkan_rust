use super::VulkanApp;
use super::{transform::Transform, math::Vector3SExt};

use cgmath::{Vector3, Matrix4, Deg, Euler, Vector2, vec3};
use cgmath::SquareMatrix;
use cgmath::ElementWise;
use winit::event::VirtualKeyCode;

#[derive(Clone, Copy)]
pub struct Camera {
    pub transform: Transform,

    max_pitch: f32,
    rot_speed: Vector3<f32>,
    move_speed: Vector3<f32>,
}

impl Camera {
    pub fn update(&mut self, is_left_clicked: bool, cursor_delta: Option<[i32; 2]>, keys_pressed: [bool; 255]) {
        if is_left_clicked && cursor_delta.is_some() {
            let delta = cursor_delta.unwrap();
            
            self.rotate(Vector3{x: delta[1] as f32, y: delta[0] as f32, z: 0.0});
        }

        if keys_pressed[VirtualKeyCode::W as usize] {
            self.move_relative(vec3(0.0, 0.0, -1.0));
        }
        if keys_pressed[VirtualKeyCode::S as usize] {
            self.move_relative(vec3(0.0, 0.0, 1.0));
        }
        if keys_pressed[VirtualKeyCode::A as usize] {
            self.move_relative(vec3(-1.0, 0.0, 0.0));
        }
        if keys_pressed[VirtualKeyCode::D as usize] {
            self.move_relative(vec3(1.0, 0.0, 0.0));
        }
    }

    pub fn rotate(&mut self, rotation: Vector3<f32>){
        self.transform.rotate(rotation.mul_element_wise(self.rot_speed));

        self.check_rotation();
    }

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

    pub fn move_relative(&mut self, dir: Vector3<f32>){
        self.transform.move_relative(dir.mul_element_wise(self.move_speed));
    }

    pub fn matrix(&mut self) -> Matrix4<f32> {
        return self.transform.get_matrix().invert().unwrap();
    }

    
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            transform: Transform::new().set_position(vec3(0.0, 1.0, 2.0)).set_rotation(vec3(0.0, 0.0, 0.0)),
            max_pitch: 85.0,
            rot_speed: Vector3::from_scalar(0.1),
            move_speed: Vector3 { x: 0.05, y: 0.05 , z: 0.1},
        }
    }
}
