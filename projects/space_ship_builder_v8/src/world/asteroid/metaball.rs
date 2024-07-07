use crate::math::random::get_random_vec3_from_min_size;
use octa_force::glam::Vec3;

pub struct Metaball {
    pub points: Vec<Vec3>,
}

impl Metaball {
    pub fn new() -> Self {
        Metaball { points: vec![] }
    }

    pub fn add_random_points_in_area(&mut self, min: Vec3, max: Vec3, num_points: usize) {
        let size = max - min;

        for _ in 0..num_points {
            let pos = get_random_vec3_from_min_size(min, size);
            self.points.push(pos);
        }
    }

    pub fn gravity_merge(&mut self, strength: f32) {
        let mut new_points = Vec::with_capacity(self.points.len());
        for pos in self.points.iter() {
            let mut sum = Vec3::ZERO;
            for other_pos in self.points.iter() {
                sum += *other_pos - *pos;
            }
            let dir = (sum / self.points.len() as f32) * strength;
            new_points.push(*pos + dir)
        }

        self.points = new_points;
    }

    pub fn get_field(&self, test_pos: Vec3, max_dist: f32) -> f32 {
        let mut field = 0.0;

        let max_dist_squared = max_dist * max_dist;
        let three_div_two = 3.0 / 2.0;

        for point in self.points.iter() {
            let r = point.distance(test_pos);

            let value = if r < (1.0 / 3.0) * max_dist {
                1.0 - (3.0 * r * r) / max_dist_squared
            } else if r < max_dist {
                let a = 1.0 - r / max_dist;
                three_div_two * a * a
            } else {
                0.0
            };

            field += value;
        }

        field
    }
}
