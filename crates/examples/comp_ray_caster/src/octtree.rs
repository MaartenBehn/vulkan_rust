use app::glam::Vec3;
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

#[derive(Clone)]
pub struct Octtree{
    pub nodes: Vec<OcttreeNode>,

    pub depth: usize,
    pub size: usize,
    
}

#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct OcttreeNode {
    // Static node Data (16 byte)
    node_id_0: u32,
    node_id_1: u32,
    mat_id: u32,
    depth: u32,
}

impl Octtree{
    pub fn new(depth: usize, mut seed: u64) -> Self {
        let mut octtree = Octtree{
            nodes: Vec::new(),
            depth: depth,
            size: Self::get_tree_size(depth),
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

    pub fn get_tree_size(depth: usize) -> usize {
        (1 - i32::pow(8, (depth + 1) as u32) / (1 - 8) - 1) as usize
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
            node_id_0: i as u32, 
            node_id_1: 0,
            mat_id: mat_id, 
            depth: depth as u32,
        });

        let mut new_i = i + 1;
        if depth < self.depth {
            for j in 0..8 {
                
                let inverse_depth = u32::pow(2, (self.depth - depth - 1) as u32);

                let child_index = Self::get_child_id(i as u32, j as u32, (self.depth - depth) as u32) as usize;
                
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
