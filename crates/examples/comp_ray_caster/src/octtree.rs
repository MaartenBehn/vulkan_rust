use app::glam::Vec3;
use shuffle::shuffler::Shuffler;
use shuffle::irs::Irs;
use rand::rngs::mock::StepRng;
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
    pub octtree_info: OcttreeInfo,

    pub buffer_size: usize, 
    pub transfer_size: usize,
    pub worker_count: usize,

    worker_offset: usize,
    transfer_offset: usize,
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

    node_id_0: u32,
    node_id_1: u32,
    mat_id: u32,
    data: u32,
}


#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct OcttreeInfo {
    tree_size: u32,
    buffer_size: u32,
    transfer_buffer_size: u32,
    depth: u32,
    worker_size_buffer: u32,
    worker_size_transfer: u32,
    transfer_offset: u32,
    fill_1: u32,
}


impl OcttreeController{
    pub fn new(octtree: Octtree, buffer_size: usize, worker_count: usize, transfer_slots_per_worker: usize) -> Self{

        let depth = octtree.depth;
        let size = octtree.size;
        let transfer_size = worker_count * transfer_slots_per_worker;

        Self { 
            octtree, 
            octtree_info: OcttreeInfo { 
                tree_size:              size as u32, 
                buffer_size:            buffer_size as u32, 
                transfer_buffer_size:   transfer_size as u32, 
                depth:                  depth as u32, 
                worker_size_buffer:     (buffer_size / worker_count) as u32, 
                worker_size_transfer:   transfer_slots_per_worker as u32, 
                transfer_offset:        0, 
                fill_1:                 0 
            },
            buffer_size:        buffer_size, 
            transfer_size:      transfer_size,
            worker_count:       worker_count,
            worker_offset:      0,
            transfer_offset:    0,
        }
    }

    pub fn step(&mut self){
        let transfer_worker_size = self.transfer_size / self.worker_count;
        self.worker_offset = (self.worker_offset + transfer_worker_size) % self.transfer_size;

        self.octtree_info.transfer_offset = (self.octtree_info.transfer_offset + self.octtree_info.worker_size_transfer) % self.octtree_info.worker_size_buffer;
    }

    pub fn get_inital_buffer_data(&self) -> &[OcttreeNode] {
        return &self.octtree.nodes[0 .. self.buffer_size];
    }

    pub fn get_requested_nodes(&mut self, requested_ids: Vec<u32>) -> Vec<OcttreeNode> {

        

        let out = self.buffer_size as u16;
        let new_children = [out, out, out, out,  out, out, out, out];
        let new_children_zero = [0, 0, 0, 0,  0, 0, 0, 0];

        let mut nodes = vec![OcttreeNode::default(); self.transfer_size];

        for (i, id) in requested_ids.iter().enumerate() {

            if *id >= self.octtree.size as u32 {
                log::error!("Requested Child ID: {:?}", id);
            }

            if *id <= 0 || *id >= self.octtree.size as u32 {
                continue;
            }

            let mut index = self.worker_offset + i;
            while index >= self.transfer_size {
                index = index - self.transfer_size;
            }

            nodes[index] = self.octtree.nodes[*id as usize];

            if nodes[index].data < self.octtree.depth as u32{
                nodes[index].children = new_children;
            }
            else{
                nodes[index].children = new_children_zero;
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

        let radius = f32::powf(2.0, self.depth as f32) / 2.0;
        let dist = Vec3::new(
            pos[0] as f32 - radius, 
            pos[1] as f32 - radius, 
            pos[2] as f32 - radius
        ).length();

        let mut mat_id = 0;
        if dist < radius {
            mat_id = 1;
        }
        
        self.nodes.push(OcttreeNode { 
            children: [0 as u16; 8], 
            node_id_0: i as u32, 
            node_id_1: 0,
            mat_id: mat_id, 
            data: depth as u32 
        });

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

                let child_material = self.nodes[child_index].mat_id;
                if child_material != 0 {
                    self.nodes[i].mat_id = child_material;
                }
            }

        }

        return new_i;
    }

}


impl OcttreeNode{
    pub fn get_ID(&self) -> u64{
        return self.node_id_0 as u64 | (self.node_id_1 as u64) << 32;
    }
}

