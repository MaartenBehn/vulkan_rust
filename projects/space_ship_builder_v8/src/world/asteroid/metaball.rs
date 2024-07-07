use crate::math::random::get_random_vec3_from_min_size;
use log::warn;
use octa_force::glam::Vec3;

pub struct Metaball {
    pub points: Vec<(Vec3, f32, f32)>,
}

impl Metaball {
    pub fn new() -> Self {
        Metaball { points: vec![] }
    }

    pub fn add_random_points_in_area(
        &mut self,
        min: Vec3,
        max: Vec3,
        num_points: usize,
        point_size: f32,
        point_strength: f32,
    ) {
        let size = max - min;

        for _ in 0..num_points {
            let pos = get_random_vec3_from_min_size(min, size);
            self.points.push((pos, point_strength, point_size));
        }
    }

    pub fn gravity_merge(&mut self, merge_strength: f32) {
        let mut new_points = Vec::with_capacity(self.points.len());
        for (pos, strength, size) in self.points.iter() {
            let mut pos_sum = Vec3::ZERO;
            for (other_pos, _, _) in self.points.iter() {
                pos_sum += *other_pos - *pos;
            }
            let dir = (pos_sum / self.points.len() as f32) * merge_strength;
            new_points.push((*pos + dir, *strength, *size))
        }

        self.points = new_points;
    }

    pub fn add_random_points_in_area_at_field_value(
        &mut self,
        min: Vec3,
        max: Vec3,
        field_min: f32,
        field_max: f32,
        num_points: usize,
        point_strength: f32,
        point_size: f32,
        iterations_per_point: usize,
    ) {
        let size = max - min;

        for _ in 0..num_points {
            for i in 0..iterations_per_point {
                let pos = get_random_vec3_from_min_size(min, size);

                let field = self.get_field(pos);
                if field < field_min || field > field_max {
                    if i == iterations_per_point - 1 {
                        warn!("Point placement timed out!");
                    }

                    continue;
                }

                self.points.push((pos, point_strength, point_size));

                break;
            }
        }
    }

    pub fn get_field(&self, test_pos: Vec3) -> f32 {
        let mut field = 0.0;

        let three_div_two = 3.0 / 2.0;

        for (point, strength, size) in self.points.iter() {
            let r = point.distance(test_pos);

            let value = if r < (1.0 / 3.0) * size {
                (1.0 - (3.0 * r * r) / (size * size)) * strength
            } else if r < *size {
                let a = 1.0 - r / size;
                (three_div_two * a * a) * strength
            } else {
                0.0
            };

            field += value;
        }

        field
    }
}
