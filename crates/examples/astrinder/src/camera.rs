use app::{
    controls::Controls,
    glam::{vec2, Vec2},
    vulkan::ash::vk::Extent2D,
};
use rapier2d::prelude::TrackedContact;

use crate::{
    math::transform::Transform,
    settings::{self, Settings},
};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub transform: Transform,
    pub scale: f32,

    pub aspect: f32,
    _fill_0: f32,
    _fill_1: f32,
    _fill_2: f32,
}

impl Camera {
    pub fn new(extent: Extent2D, settings: Settings) -> Self {
        Self {
            transform: settings.camera_inital_transform,
            scale: settings.camera_inital_scale,

            aspect: extent.height as f32 / extent.width as f32,
            _fill_0: 0.0,
            _fill_1: 0.0,
            _fill_2: 0.0,
        }
    }

    pub fn update_aspect(&mut self, extent: Extent2D) {
        self.aspect = extent.height as f32 / extent.width as f32;
    }

    pub fn update(&mut self, controls: &Controls, time_step: f32, settings: Settings) {
        if controls.w {
            self.transform.pos.y += time_step * settings.camera_speed;
        }

        if controls.s {
            self.transform.pos.y -= time_step * settings.camera_speed;
        }

        if controls.d {
            self.transform.pos.x += time_step * settings.camera_speed;
        }

        if controls.a {
            self.transform.pos.x -= time_step * settings.camera_speed;
        }

        if controls.up {
            self.scale += time_step * settings.camera_scale_factor;
        }

        if controls.down {
            self.scale -= time_step * settings.camera_scale_factor;
        }
    }
}
