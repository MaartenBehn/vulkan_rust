use app::log;

pub const PTR_MASK: u32 = 8388607;
pub const FAR_MASK: u32 = 8388608;
pub const BRANCH_MASK: u32 = 4278190080;
pub const MAX_PTR: usize = FAR_MASK as usize;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CompressedNode {
    pub header: u32,
    pub mats: [u8; 8],
}

impl CompressedNode {
    pub fn new(ptr: usize, branches: u8, mats: [u8; 8], far: bool) -> Self {
        debug_assert!(ptr < MAX_PTR);
    
        let header = (ptr as u32) + ((far as u32) << 23) + ((branches as u32) << 24);
        CompressedNode { header, mats }
    }

    pub fn new_far_pointer(ptr: usize) -> CompressedNode {
        CompressedNode { 
            header: ptr as u32, 
            mats: [0; 8]
        }
    }
    
    pub fn get_ptr(&self) -> usize {
        (self.header & PTR_MASK) as usize
    }
    
    pub fn get_far(&self) -> bool {
        (self.header & FAR_MASK) != 0
    }
    
    pub fn get_branches(&self) -> u8 {
        ((self.header & BRANCH_MASK) >> 24) as u8
    }
    
    pub fn get_branch(&self, index: usize) -> bool {
        (self.header & (1 << (24 + index))) != 0
    }
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


pub const CHILD_CONFIG: [[i32; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 0],
    [0, 1, 1],
    [1, 0, 0],
    [1, 0, 1],
    [1, 1, 0],
    [1, 1, 1],
];

pub fn print_page(nodes: &[CompressedNode]){
    for (i, node) in nodes.iter().enumerate() {
        let ptr = node.get_ptr();
        log::info!("{} {}", i, ptr)
    }
}