use serde::{Deserialize, Serialize};

const UPPER16BITS: u32 = (u16::MAX as u32) << 16;

#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct OcttreeNode {
    // Static node Data (16 byte)
    node_id_0: u32,
    node_id_1: u32,
    mat_id: u32,
    bit_field: u32,
}

impl OcttreeNode {
    pub fn new(node_id: u64, mat_id: u32, depth: u16, leaf: bool, empty: bool) -> Self {
        let mut node = OcttreeNode {
            node_id_0: 0,
            node_id_1: 0,
            mat_id: mat_id,
            bit_field: 0,
        };

        node.set_node_id(node_id);
        node.set_depth(depth);
        node.set_leaf(leaf);
        node.set_empty(empty);

        node
    }

    pub fn from_data(node_id_0: u32, node_id_1: u32, mat_id: u32, bit_field: u32) -> Self {
        Self {
            node_id_0,
            node_id_1,
            mat_id,
            bit_field,
        }
    }

    pub fn set_node_id(&mut self, node_id: u64) {
        self.node_id_0 = node_id as u32;
        self.node_id_1 = (node_id >> 32) as u32;
    }

    pub fn get_node_id(&self) -> u64 {
        (self.node_id_0 as u64) + ((self.node_id_1 as u64) >> 32)
    }

    pub fn set_depth(&mut self, depth: u16) {
        self.bit_field = (depth as u32) + (self.bit_field & UPPER16BITS);
    }

    pub fn get_depth(&self) -> u16 {
        self.bit_field as u16
    }

    pub fn set_leaf(&mut self, leaf: bool) {
        self.bit_field = ((leaf as u32) << 16) + (self.bit_field & !(1 << 16));
    }

    pub fn get_leaf(&self) -> bool {
        ((self.bit_field >> 16) & 1) == 1
    }

    pub fn set_empty(&mut self, empty: bool) {
        self.bit_field = ((empty as u32) << 17) + (self.bit_field & !(1 << 17));
    }

    pub fn get_empty(&self) -> bool {
        ((self.bit_field >> 17) & 1) == 1
    }

    pub fn set_mat_id(&mut self, mat_id: u32) {
        self.mat_id = mat_id
    }

    pub fn get_mat_id(&self) -> u32 {
        self.mat_id
    }
}
