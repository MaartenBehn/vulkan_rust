use app::glam::BVec3;

use crate::rotation::Rot;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Config(u8);

impl Config {
    pub fn get_possibilities(self) -> Vec<(Config, Rot)> {
        let mut possibilities = Vec::new();
        let base_rot = Rot::default();

        for flip_x in [false, true] {
            for flip_y in [false, true] {
                for flip_z in [false, true] {
                    let axis = BVec3::new(flip_x, flip_y, flip_z);

                    let config = self.flip(axis);
                    let rot = base_rot.flip(axis);

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

        possibilities
    }

    fn flip(self, axis: BVec3) -> Config {
        let mut old: [bool; 8] = self.into();
        let mut new = old;

        if axis.x {
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

        if axis.z {
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
