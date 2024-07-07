use octa_force::glam::{vec3, Vec3};

pub fn get_random_vec3_from_min_max(min: Vec3, max: Vec3) -> Vec3 {
    get_random_vec3_from_min_size(min, max - min)
}

pub fn get_random_vec3_from_min_size(min: Vec3, size: Vec3) -> Vec3 {
    vec3(
        fastrand::f32() * size.x + min.x,
        fastrand::f32() * size.y + min.y,
        fastrand::f32() * size.z + min.z,
    )
}
