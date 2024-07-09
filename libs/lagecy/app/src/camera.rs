use std::time::Duration;

use glam::{vec3, Mat3, Mat4, Quat, Vec3};
use vulkan::ash::vk::Extent2D;

use crate::controls::Controls;

const ANGLE_PER_POINT: f32 = 0.001745;

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub direction: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub speed: f32,
    pub up: Vec3,
}

impl Camera {
    pub fn new(
        position: Vec3,
        direction: Vec3,
        fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
        up: Vec3,
    ) -> Self {
        Self {
            position,
            direction: direction.normalize(),
            fov,
            aspect_ratio,
            z_near,
            z_far,
            speed: 3.0,
            up,
        }
    }

    pub fn base(extent: Extent2D) -> Self {
        Self::new (
            vec3(0.0, 0.0, 1.0),
            vec3(0.0, 0.0, -1.0),
            60.0,
            extent.width as f32 / extent.height as f32,
            0.1,
            10.0,
            vec3(0.0, 1.0, 0.0)
        )
    }

    pub fn update(&mut self, controls: &Controls, delta_time: Duration) {
        let delta_time = delta_time.as_secs_f32();
        let side = self.direction.cross(self.up);

        // Update direction
        let new_direction = if controls.rigth {
            let side_rot = Quat::from_axis_angle(side, -controls.cursor_delta[1] * ANGLE_PER_POINT);
            let up_rot =  Quat::from_axis_angle(self.up, -controls.cursor_delta[0] * ANGLE_PER_POINT);
            let rot = Mat3::from_quat(side_rot * up_rot);

            (rot * self.direction).normalize()
        } else {
            self.direction
        };

        // Update position
        let mut direction = Vec3::ZERO;

        if controls.w {
            direction += new_direction;
        }
        if controls.s {
            direction -= new_direction;
        }
        if controls.d {
            direction += side;
        }
        if controls.a {
            direction -= side;
        }
        if controls.up {
            direction += self.up;
        }
        if controls.down {
            direction -= self.up;
        }

        let direction = if direction.length_squared() == 0.0 {
            direction
        } else {
            direction.normalize()
        };

        self.position += direction * self.speed * delta_time;
        self.direction = new_direction;
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(
            self.position,
            self.position + self.direction,
            self.up,
        )
    }

    pub fn projection_matrix(&self) -> Mat4 {
        perspective(
            self.fov.to_radians(),
            self.aspect_ratio,
            self.z_near,
            self.z_far,
        )
    }
}

#[rustfmt::skip]
pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    
    let f = (fovy / 2.0).tan().recip();

    let c0r0 = f / aspect;
    let c0r1 = 0.0f32;
    let c0r2 = 0.0f32;
    let c0r3 = 0.0f32;

    let c1r0 = 0.0f32;
    let c1r1 = -f;
    let c1r2 = 0.0f32;
    let c1r3 = 0.0f32;

    let c2r0 = 0.0f32;
    let c2r1 = 0.0f32;
    let c2r2 = -far / (far - near);
    let c2r3 = -1.0f32;

    let c3r0 = 0.0f32;
    let c3r1 = 0.0f32;
    let c3r2 = -(far * near) / (far - near);
    let c3r3 = 0.0f32;

    Mat4::from_cols_array(&[
        c0r0, c0r1, c0r2, c0r3,
        c1r0, c1r1, c1r2, c1r3,
        c2r0, c2r1, c2r2, c2r3,
        c3r0, c3r1, c3r2, c3r3
    ])
}

