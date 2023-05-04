use std::ops::{Add, AddAssign, Mul, MulAssign};

use app::glam::{Vec2, Vec3, Vec3Swizzles};
use cgmath::{Decomposed, Vector2, Basis2, Rotation2, Rad};


#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Transform{
    pub pos: Vec2,
    pub rot: f32,
}

impl Transform {
    pub fn new(pos: Vec2, rot: f32) -> Self {
        Self { pos, rot }
    }
}

impl From<Vec3> for Transform {
    fn from(value: Vec3) -> Self {
        Self { pos: value.xy(), rot: value.z }
    }
}

impl From<Transform> for Vec3 {
    fn from(value: Transform) -> Self {
        Vec3 { x: value.pos.x, y: value.pos.y, z: value.rot }
    }
}

impl Add<Transform> for Transform {
    type Output = Transform;

    #[inline(always)]
    fn add(self, rhs: Transform) -> Self::Output {
        Self{
            pos: self.pos + rhs.pos,
            rot: self.rot + rhs.rot,
        }
    }
}

impl AddAssign<Transform> for Transform {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Transform) {
        self.pos += rhs.pos;
        self.rot += rhs.rot;
    }
}

impl Mul<f32> for Transform {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        Self{
            pos: self.pos * rhs,
            rot: self.rot * rhs,
        }
    }
}

impl MulAssign<f32> for Transform {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        self.pos *= rhs;
        self.rot *= rhs;
    }
}


impl From<Transform> for Decomposed<Vector2<f32>, Basis2<f32>> {
    fn from(t: Transform) -> Self {
        Decomposed {
            disp: Vector2::new(t.pos.x, t.pos.y),
            rot: Rotation2::from_angle(Rad(-t.rot)),
            scale: 1.,
        }
    }
}

