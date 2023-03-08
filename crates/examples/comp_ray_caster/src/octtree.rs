use app::log;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub const OCTTREE_DEPTH: usize = 5; // 4; // max 255
pub const OCTTREE_SIZE: usize = 37499; // 4681; // (1 - pow(8, OCTTREE_DEPTH + 1)) / 1 - 8
pub const OCTTREE_BUFFER_SIZE: usize = 4000; 
pub const OCTTREE_TRANSFER_BUFFER_SIZE: usize = 32;

const OCTTREE_CONFIG: [[u32; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 0],
    [0, 1, 1],
    [1, 0, 0],
    [1, 0, 1],
    [1, 1, 0],
    [1, 1, 1],
];

#[derive(Clone)]
pub struct Octtree{
    pub nodes: Vec<OcttreeNode>,
}

#[derive(Clone, Copy, Default)]
pub struct OcttreeNode {
    children: [u16; 8],

    color: [u16; 4],
    node_id: u32,
    data: u32, // first 8 bits = depth, Nr 8 is render 
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct OcttreeInfo {
    octtree_size: u32,
    octtree_buffer_size: u32,
    octtree_transfer_buffer_size: u32,
    octtree_depth: u32,
}

impl OcttreeNode{
    fn new(node_id: u32, children: [u16; 8], color: [u16; 4], data: u32) -> Self {
        Self { 
            children, 
            color, 
            node_id, 
            data,
        }
    }
}


impl Octtree{
    pub fn new() -> Self {
        let mut octtree = Octtree{
            nodes: Vec::new(),
        };

        let mut seed_rng= rand::thread_rng();
        let seed: u64 = seed_rng.gen();

        log::info!("Octtree Seed: {:?}", seed);
        let mut rng = StdRng::seed_from_u64(8998840515808983062);
       
        octtree.update(0, 0, [0, 0, 0], &mut rng);

        return octtree;
    }

    fn get_child_id(node_id: u32, child_nr: u32, depth: u32) -> u32{
        let child_size = ((1 - i32::pow(8, depth)) / -7) as u32;
        return (node_id + child_size * child_nr + 1) as u32;
    }

    fn update(&mut self, i: usize, depth: usize, pos: [u32; 3], rng: &mut impl Rng) -> usize {

        
        let node_id = i as u32;
        let data = depth as u32;
        let children = [0 as u16; 8];
        let color = [0 as u16; 4];

        let node = OcttreeNode::new(node_id, children, color, data);
        self.nodes.push(node);

        let mut new_i = i + 1;
        if depth < OCTTREE_DEPTH {
            for j in 0..8 {
                
                let inverse_depth = u32::pow(2, (OCTTREE_DEPTH - depth - 1) as u32);

                let child_index = Self::get_child_id(i as u32, j as u32, (OCTTREE_DEPTH - depth) as u32) as usize;
                self.nodes[i].children[j] = child_index as u16;
                
                let new_pos = [
                    pos[0] + OCTTREE_CONFIG[j][0] * inverse_depth, 
                    pos[1] + OCTTREE_CONFIG[j][1] * inverse_depth, 
                    pos[2] + OCTTREE_CONFIG[j][2] * inverse_depth,
                    ];
                
                new_i = self.update(new_i, depth + 1, new_pos, rng);

                let child_color = self.nodes[child_index].color;
                if child_color[0] != 0 || child_color[1] != 0 || child_color[2] != 0{
                    self.nodes[i].color = child_color;
                }
            }

        }else{

            let rand_float: f32 = rng.gen();
            if rand_float < 0.2 {
                self.nodes[i].color = [rng.gen(), rng.gen(), rng.gen(), 0];
            }

            if pos == [0, 0, 0] && depth == OCTTREE_DEPTH {
                self.nodes[i].color = [u16::MAX, u16::MAX, u16::MAX, 0];
            }
        }

        return new_i;
    }


    pub fn get_inital_buffer_data(&self) -> &[OcttreeNode] {
        return &self.nodes[0..OCTTREE_BUFFER_SIZE];
    }

    pub fn get_requested_nodes(&self, requested_ids: Vec<u32>) -> [OcttreeNode; OCTTREE_TRANSFER_BUFFER_SIZE] {

        let out = OCTTREE_BUFFER_SIZE as u16;
        let new_children = [out, out, out, out,  out, out, out, out];
        let new_children_zero = [0, 0, 0, 0,  0, 0, 0, 0];

        let mut nodes = [OcttreeNode::default(); OCTTREE_TRANSFER_BUFFER_SIZE];

        for (i, id) in requested_ids.iter().enumerate() {

            if *id >= OCTTREE_SIZE as u32 {
                log::error!("Requested Child ID: {:?}", id);
            }

            if *id <= 0 || *id >= OCTTREE_SIZE as u32 {
                break;
            }

            nodes[i] = self.nodes[*id as usize];

            if nodes[i].data < OCTTREE_DEPTH as u32{
                nodes[i].children = new_children;
            }
            else{
                nodes[i].children = new_children_zero;
            }

        }

        nodes
    }

}


impl OcttreeInfo{
    pub fn new() -> Self {
        Self { 
            octtree_size:                   OCTTREE_SIZE as u32, 
            octtree_buffer_size:            OCTTREE_BUFFER_SIZE as u32,
            octtree_transfer_buffer_size:   OCTTREE_TRANSFER_BUFFER_SIZE as u32, 
            octtree_depth:                  OCTTREE_DEPTH as u32, 
        }
    }
}