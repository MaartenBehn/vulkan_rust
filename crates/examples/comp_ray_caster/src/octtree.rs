use app::log;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

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


pub struct OcttreeController{
    pub octtree: Octtree,

    pub buffer_size: usize, 
    pub transfer_size: usize,
}

#[derive(Clone)]
pub struct Octtree{
    pub nodes: Vec<OcttreeNode>,

    pub depth: usize,
    pub size: usize,
    
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


impl OcttreeController{
    pub fn new(octtree: Octtree, buffer_size: usize, transfer_size: usize) -> Self{
        Self { 
            octtree, 
            buffer_size: buffer_size, 
            transfer_size: transfer_size 
        }
    }

    pub fn get_octtree_info(&self) -> OcttreeInfo {
        OcttreeInfo::new(self.octtree.depth as u32, self.octtree.size as u32, self.buffer_size as u32, self.transfer_size as u32)
    }

    pub fn get_inital_buffer_data(&self) -> &[OcttreeNode] {
        return &self.octtree.nodes[0 .. self.buffer_size];
    }

    pub fn get_requested_nodes(&self, requested_ids: Vec<u32>) -> Vec<OcttreeNode> {

        let out = self.buffer_size as u16;
        let new_children = [out, out, out, out,  out, out, out, out];
        let new_children_zero = [0, 0, 0, 0,  0, 0, 0, 0];

        let mut nodes = vec![OcttreeNode::default(); self.transfer_size];

        for (i, id) in requested_ids.iter().enumerate() {

            if *id >= self.octtree.size as u32 {
                log::error!("Requested Child ID: {:?}", id);
            }

            if *id <= 0 || *id >= self.octtree.size as u32 {
                break;
            }

            nodes[i] = self.octtree.nodes[*id as usize];

            if nodes[i].data < self.octtree.depth as u32{
                nodes[i].children = new_children;
            }
            else{
                nodes[i].children = new_children_zero;
            }
        }

        nodes
    }
}





impl Octtree{
    pub fn new(depth: usize, mut seed: u64) -> Self {
        let mut octtree = Octtree{
            nodes: Vec::new(),
            depth: depth,
            size: (1 - i32::pow(8, depth as u32 + 1) / -7) as usize,
        };

        if seed == 0 {
            let mut seed_rng= rand::thread_rng();
            seed = seed_rng.gen();
        }
       
        log::info!("Octtree Seed: {:?}", seed);
        let mut rng = StdRng::seed_from_u64(seed);
       
        octtree.inital_fill(0, 0, [0, 0, 0], &mut rng);

        return octtree;
    }

    fn get_child_id(node_id: u32, child_nr: u32, depth: u32) -> u32{
        let child_size = ((1 - i32::pow(8, depth)) / -7) as u32;
        return (node_id + child_size * child_nr + 1) as u32;
    }

    fn inital_fill(&mut self, i: usize, depth: usize, pos: [u32; 3], rng: &mut impl Rng) -> usize {

        
        let node_id = i as u32;
        let data = depth as u32;
        let children = [0 as u16; 8];
        let color = [0 as u16; 4];

        let node = OcttreeNode::new(node_id, children, color, data);
        self.nodes.push(node);

        let mut new_i = i + 1;
        if depth < self.depth {
            for j in 0..8 {
                
                let inverse_depth = u32::pow(2, (self.depth - depth - 1) as u32);

                let child_index = Self::get_child_id(i as u32, j as u32, (self.depth - depth) as u32) as usize;
                self.nodes[i].children[j] = child_index as u16;
                
                let new_pos = [
                    pos[0] + OCTTREE_CONFIG[j][0] * inverse_depth, 
                    pos[1] + OCTTREE_CONFIG[j][1] * inverse_depth, 
                    pos[2] + OCTTREE_CONFIG[j][2] * inverse_depth,
                    ];
                
                new_i = self.inital_fill(new_i, depth + 1, new_pos, rng);

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

            if pos == [0, 0, 0] && depth == self.depth {
                self.nodes[i].color = [u16::MAX, u16::MAX, u16::MAX, 0];
            }
        }

        return new_i;
    }

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


impl OcttreeInfo{
    fn new(depth: u32, tree_size: u32, buffer_size: u32, transfer_size: u32) -> Self {
        Self { 
            octtree_size: tree_size, 
            octtree_buffer_size: buffer_size,
            octtree_transfer_buffer_size: transfer_size, 
            octtree_depth: depth, 
        }
    }
}