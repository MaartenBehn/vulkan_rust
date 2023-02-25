use cgmath::{prelude::*, BaseFloat, BaseNum, Matrix4, Rad, Vector2, Vector3, Vector4};
use extend::ext;

/// Perspective matrix that is suitable for Vulkan.
///
/// It inverts the projected y-axis. And set the depth range to 0..1
/// instead of -1..1. Mind the vertex winding order though.
pub fn perspective<S, F>(fovy: F, aspect: S, near: S, far: S) -> Matrix4<S>
where
    S: BaseFloat,
    F: Into<Rad<S>>,
{
    let two = S::one() + S::one();
    let f = Rad::cot(fovy.into() / two);

    let c0r0 = f / aspect;
    let c0r1 = S::zero();
    let c0r2 = S::zero();
    let c0r3 = S::zero();

    let c1r0 = S::zero();
    let c1r1 = -f;
    let c1r2 = S::zero();
    let c1r3 = S::zero();

    let c2r0 = S::zero();
    let c2r1 = S::zero();
    let c2r2 = -far / (far - near);
    let c2r3 = -S::one();

    let c3r0 = S::zero();
    let c3r1 = S::zero();
    let c3r2 = -(far * near) / (far - near);
    let c3r3 = S::zero();

    #[cfg_attr(rustfmt, rustfmt::skip)]
    Matrix4::new(
        c0r0, c0r1, c0r2, c0r3,
        c1r0, c1r1, c1r2, c1r3,
        c2r0, c2r1, c2r2, c2r3,
        c3r0, c3r1, c3r2, c3r3,
    )
}

/// Clamp `value` between `min` and `max`.
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    let value = if value > max { max } else { value };
    if value < min {
        min
    } else {
        value
    }
}

#[ext]
pub impl<S: BaseNum> Vector2<S> {
    /// A Vector from a scalar.
    #[inline]
    fn from_scalar(s: S) -> Vector2<S> {
        Vector2::new(s, s)
    }
}

#[ext]
pub impl<S: BaseNum> Vector3<S> {
    /// A Vector from a scalar.
    #[inline]
    fn from_scalar(s: S) -> Vector3<S> {
        Vector3::new(s, s, s)
    }
}

#[ext]
pub impl<S: BaseNum> Vector4<S> {
    /// A Vector from a scalar.
    #[inline]
    fn from_scalar(s: S) -> Vector4<S> {
        Vector4::new(s, s, s, s)
    }
}
