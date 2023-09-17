const PTR_MASK: u32 = 0x00FFFFFF;
const BRANCH_MASK: u32 = 0xFF000000;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Node {
    pub header: u32,
    pub mats: [u8; 8],
}

pub fn new_node(ptr: usize, branches: u8, mats: [u8; 8]) -> Node {
    let header = (ptr as u32) + ((branches as u32) << 24);
    Node { header, mats }
}

pub fn get_ptr(node: Node) -> usize {
    (node.header & PTR_MASK) as usize
}

pub fn get_branches(node: Node) -> u8 {
    ((node.header & BRANCH_MASK) >> 24) as u8
}

pub fn get_branch(node: Node, index: usize) -> bool {
    (node.header & (1 << (24 + index))) != 0
}

pub fn bools_to_bits(bools: [bool; 8]) -> u8 {
    (bools[0] as u8)
        + ((bools[1] as u8) << 1)
        + ((bools[2] as u8) << 2)
        + ((bools[3] as u8) << 3)
        + ((bools[4] as u8) << 4)
        + ((bools[5] as u8) << 5)
        + ((bools[6] as u8) << 6)
        + ((bools[7] as u8) << 7)
}

pub fn bits_to_bools(bits: u8) -> [bool; 8] {
    [
        (bits & 1) == 1,
        (bits & 2) == 2,
        (bits & 4) == 4,
        (bits & 8) == 8,
        (bits & 16) == 16,
        (bits & 32) == 32,
        (bits & 64) == 64,
        (bits & 128) == 128,
    ]
}


pub const CHILD_CONFIG: [[u32; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 0],
    [0, 1, 1],
    [1, 0, 0],
    [1, 0, 1],
    [1, 1, 0],
    [1, 1, 1],
];