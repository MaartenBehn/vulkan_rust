use std::f32::consts::PI;

use app::glam::{vec3, BVec3};

use crate::{
    node::{BlockIndex, NodeID, BLOCK_INDEX_EMPTY},
    rotation::Rot,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Debug)]
pub struct Config(u8);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Debug)]
pub struct BlockConfig([BlockIndex; 8]);

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

impl From<Vec<bool>> for Config {
    fn from(value: Vec<bool>) -> Self {
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

impl BlockConfig {
    pub fn get_first(self) -> BlockIndex {
        self.0[0]
    }

    pub fn get_possibilities(self, nodes: [NodeID; 8]) -> Vec<(BlockConfig, [NodeID; 8])> {
        let mut possibilities = Vec::new();
        let base_rot = Rot::default();
        let rots = [0.0, PI * 0.5, PI * 1.5];

        for flip_x in [false, true] {
            for flip_y in [false, true] {
                for flip_z in [false, true] {
                    for rot_x in rots {
                        for rot_y in rots {
                            for rot_z in rots {
                                let flip = BVec3::new(flip_x, flip_y, flip_z);
                                let rot: Rot =
                                    base_rot.get_permutation(flip, vec3(rot_x, rot_y, rot_z));

                                let mut config_bools: [BlockIndex; 8] = self.into();
                                config_bools = Self::rotate_x(config_bools, rot_x);
                                config_bools = Self::rotate_y(config_bools, rot_y);
                                config_bools = Self::rotate_z(config_bools, rot_z);
                                config_bools = Self::flip(config_bools, flip);
                                let config: BlockConfig = config_bools.into();

                                let mut new_nodes = nodes.to_owned();
                                for node in new_nodes.iter_mut() {
                                    node.rot = node.rot * rot;
                                }
                                new_nodes = Self::rotate_x(new_nodes, rot_x);
                                new_nodes = Self::rotate_y(new_nodes, rot_y);
                                new_nodes = Self::rotate_z(new_nodes, rot_z);
                                new_nodes = Self::flip(new_nodes, flip);

                                let mut found = false;
                                for (c, _) in possibilities.iter() {
                                    if *c == config {
                                        found = true;
                                    }
                                }

                                if !found {
                                    possibilities.push((config, new_nodes));
                                }
                            }
                        }
                    }
                }
            }
        }

        possibilities
    }

    fn flip<A: Copy>(mut old: [A; 8], axis: BVec3) -> [A; 8] {
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

        new
    }

    fn rotate_x<A: Copy>(old: [A; 8], angle: f32) -> [A; 8] {
        let new = if angle == 0.0 {
            old
        } else if angle == PI * 0.5 {
            [
                old[2], old[3], old[6], old[7], old[0], old[1], old[4], old[5],
            ]
        } else {
            debug_assert!(angle == PI * 1.5);
            [
                old[4], old[5], old[0], old[1], old[6], old[7], old[2], old[3],
            ]
        };

        new
    }

    fn rotate_y<A: Copy>(old: [A; 8], angle: f32) -> [A; 8] {
        let new = if angle == 0.0 {
            old
        } else if angle == PI * 0.5 {
            [
                old[4], old[0], old[6], old[2], old[5], old[1], old[7], old[3],
            ]
        } else {
            assert!(angle == PI * 1.5);
            [
                old[1], old[5], old[3], old[7], old[0], old[4], old[2], old[6],
            ]
        };

        new
    }

    fn rotate_z<A: Copy>(old: [A; 8], angle: f32) -> [A; 8] {
        let new = if angle == 0.0 {
            old
        } else if angle == PI * 0.5 {
            [
                old[1], old[3], old[0], old[2], old[5], old[7], old[4], old[6],
            ]
        } else {
            assert!(angle == PI * 1.5);
            [
                old[2], old[0], old[3], old[1], old[6], old[4], old[7], old[5],
            ]
        };

        new
    }
}

impl From<[BlockIndex; 8]> for BlockConfig {
    fn from(value: [BlockIndex; 8]) -> Self {
        BlockConfig(value)
    }
}

impl Into<[BlockIndex; 8]> for BlockConfig {
    fn into(self) -> [BlockIndex; 8] {
        self.0
    }
}

impl Into<String> for BlockConfig {
    fn into(self) -> String {
        format!(
            "{}{}{}{}{}{}{}{}",
            self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0]
        )
    }
}

impl Into<Config> for BlockConfig {
    fn into(self) -> Config {
        let b = [
            self.0[0] != BLOCK_INDEX_EMPTY,
            self.0[1] != BLOCK_INDEX_EMPTY,
            self.0[2] != BLOCK_INDEX_EMPTY,
            self.0[3] != BLOCK_INDEX_EMPTY,
            self.0[4] != BLOCK_INDEX_EMPTY,
            self.0[5] != BLOCK_INDEX_EMPTY,
            self.0[6] != BLOCK_INDEX_EMPTY,
            self.0[7] != BLOCK_INDEX_EMPTY,
        ];
        b.into()
    }
}
