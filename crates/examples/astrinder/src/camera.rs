use app::{glam::Vec2, vulkan::ash::vk::Extent2D};

const CAMERA_INIT_POS: Vec2 = Vec2::ZERO;
const CAMERA_INIT_ROT: f32 = 0.0;
const CAMERA_INIT_SCALE: f32 = 0.03; // 0.0

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec2,
    pub rot: f32,
    pub scale: f32,

    pub aspect: f32,
    _fill_0: f32,
    _fill_1: f32,
    _fill_2: f32,
}

impl Camera {
    pub fn new(extent: Extent2D) -> Self {
        Self {
            pos: CAMERA_INIT_POS,
            rot: CAMERA_INIT_ROT,
            scale: CAMERA_INIT_SCALE,

            aspect: extent.height as f32 / extent.width as f32,
            _fill_0: 0.0,
            _fill_1: 0.0,
            _fill_2: 0.0,
        }
    }
}
