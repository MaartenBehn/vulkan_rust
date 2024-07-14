use octa_force::glam::{vec3, Mat4, UVec3, Vec3};

pub fn get_aabb_of_transformed_cube(transform: Mat4, cube_size: Vec3) -> (Vec3, Vec3) {
    let corners = [
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, cube_size.y, 0.0),
        vec3(0.0, 0.0, cube_size.z),
        vec3(0.0, cube_size.y, cube_size.z),
        vec3(cube_size.x, 0.0, 0.0),
        vec3(cube_size.x, cube_size.y, 0.0),
        vec3(cube_size.x, 0.0, cube_size.z),
        vec3(cube_size.x, cube_size.y, cube_size.z),
    ];

    let mut min = Vec3::MAX;
    let mut max = Vec3::MIN;

    for corner in corners {
        let transformed = transform.transform_point3(corner);
        min = min.min(transformed);
        max = max.max(transformed);
    }

    (min, max)
}
