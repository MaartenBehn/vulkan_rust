use std::f32::consts::PI;

use app::glam::{BVec3, Mat3, Mat4, Vec3};

use crate::rotation::Rot;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Config(u8);

impl Config {
    pub fn get_possibilities(self) -> Vec<(Config, Rot)> {
        let mut possibilities = Vec::new();
        let base_rot = Rot::default();
        let rots = [0.0, PI * 0.5, PI * 2.0];

        for flip_x in [false, true] {
            for flip_y in [false, true] {
                for flip_z in [false, true] {
                    for rot_x in rots {
                        for rot_y in rots {
                            for rot_z in rots {
                                let flip_mat = Mat4::from_scale(Vec3::new(
                                    if flip_x { -1.0 } else { 1.0 },
                                    if flip_y { -1.0 } else { 1.0 },
                                    if flip_z { -1.0 } else { 1.0 },
                                ));
                                let rot_x_mat = Mat4::from_rotation_x(rot_x);
                                let rot_y_mat = Mat4::from_rotation_y(rot_y);
                                let rot_z_mat = Mat4::from_rotation_y(rot_z);
                                let trans_mat = flip_mat;
                                //    .mul_mat4(&rot_x_mat)
                                //    .mul_mat4(&rot_y_mat)
                                //    .mul_mat4(&rot_z_mat);

                                let mat = Mat4::from_mat3(base_rot.into()).mul_mat4(&trans_mat);
                                let rot: Rot = Mat3::from_mat4(mat).into();

                                let flip = BVec3::new(flip_x, flip_y, flip_z);
                                let mut config = self.flip(flip);
                                //config = config.rotate_x(rot_x);
                                //config = config.rotate_y(rot_y);
                                // config = config.rotate_z(rot_z);

                                let mut found = false;
                                for (c, _) in possibilities.iter() {
                                    if *c == config {
                                        found = true;
                                    }
                                }

                                if !found {
                                    possibilities.push((config, rot));
                                }
                            }
                        }
                    }
                }
            }
        }

        possibilities
    }

    fn flip(self, axis: BVec3) -> Config {
        let mut old: [bool; 8] = self.into();
        let mut new = old;

        if axis.z {
            new[0] = old[1];
            new[1] = old[0];
            new[2] = old[3];
            new[3] = old[2];

            new[4] = old[5];
            new[5] = old[4];
            new[6] = old[7];
            new[7] = old[6];
        }

        if axis.y {
            old = new;

            new[0] = old[2];
            new[1] = old[3];
            new[2] = old[0];
            new[3] = old[1];

            new[4] = old[6];
            new[5] = old[7];
            new[6] = old[4];
            new[7] = old[5];
        }

        if axis.x {
            old = new;

            new[0] = old[4];
            new[1] = old[5];
            new[2] = old[6];
            new[3] = old[7];

            new[4] = old[0];
            new[5] = old[1];
            new[6] = old[2];
            new[7] = old[3];
        }

        new.into()
    }

    fn rotate_x(self, angle: f32) -> Config {
        let old: [bool; 8] = self.into();

        let new = if angle == 0.0 {
            old
        } else if angle == PI * 0.5 {
            [
                old[4], old[5], old[0], old[1], old[6], old[7], old[2], old[3],
            ]
        } else {
            assert!(angle == PI * 2.0);
            [
                old[2], old[3], old[6], old[7], old[0], old[1], old[4], old[5],
            ]
        };

        new.into()
    }

    fn rotate_y(self, angle: f32) -> Config {
        let old: [bool; 8] = self.into();

        let new = if angle == 0.0 {
            old
        } else if angle == PI * 0.5 {
            [
                old[1], old[5], old[3], old[7], old[0], old[4], old[2], old[6],
            ]
        } else {
            assert!(angle == PI * 2.0);
            [
                old[4], old[0], old[6], old[2], old[5], old[1], old[7], old[3],
            ]
        };

        new.into()
    }

    fn rotate_z(self, angle: f32) -> Config {
        let old: [bool; 8] = self.into();

        let new = if angle == 0.0 {
            old
        } else if angle == PI * 0.5 {
            [
                old[1], old[3], old[0], old[2], old[5], old[7], old[4], old[6],
            ]
        } else {
            assert!(angle == PI * 2.0);
            [
                old[2], old[0], old[3], old[1], old[6], old[4], old[7], old[5],
            ]
        };

        new.into()
    }
}

impl From<[bool; 8]> for Config {
    fn from(value: [bool; 8]) -> Self {
        Config(
            value[0] as u8
                + ((value[1] as u8) << 1)
                + ((value[2] as u8) << 2)
                + ((value[3] as u8) << 3)
                + ((value[4] as u8) << 4)
                + ((value[5] as u8) << 5)
                + ((value[6] as u8) << 6)
                + ((value[7] as u8) << 7),
        )
    }
}

impl From<u8> for Config {
    fn from(value: u8) -> Self {
        Config(value)
    }
}

impl Into<[bool; 8]> for Config {
    fn into(self) -> [bool; 8] {
        [
            (self.0 & 1) == 1,
            ((self.0 >> 1) & 1) == 1,
            ((self.0 >> 2) & 1) == 1,
            ((self.0 >> 3) & 1) == 1,
            ((self.0 >> 4) & 1) == 1,
            ((self.0 >> 5) & 1) == 1,
            ((self.0 >> 6) & 1) == 1,
            ((self.0 >> 7) & 1) == 1,
        ]
    }
}

impl Into<usize> for Config {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl Into<u8> for Config {
    fn into(self) -> u8 {
        self.0 as u8
    }
}
