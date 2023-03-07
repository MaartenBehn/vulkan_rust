use app::{glam::Vec3, log};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub const OCTTREE_DEPTH: usize = 2; // 255
pub const OCTTREE_NODE_COUNT: usize = 73; // 4681; // (1 - pow(8, OCTTREE_DEPTH + 1)) / 1 - 8
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


#[derive(Clone, Copy)]
pub struct Octtree{
    pub nodes: [OcttreeNode; OCTTREE_NODE_COUNT]
}

#[derive(Clone, Copy, Default)]
pub struct OcttreeNode {
    children: [u16; 8],

    color: [u16; 4],
    //reflective: u16,
    nodeId: u32,
    data: u32, // first 8 bits = depth, Nr 8 is render 
}


impl Octtree{
    pub fn new() -> Octtree{
        let mut octtree = Octtree{
            nodes: [OcttreeNode::default(); OCTTREE_NODE_COUNT],
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

        self.nodes[i].nodeId = i as u32;
        self.nodes[i].data = depth as u32;

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

                let childColor = self.nodes[child_index].color;
                if childColor[0] != 0 || childColor[1] != 0 || childColor[2] != 0{
                    self.nodes[i].color = childColor;
                }
            }

        }else{

            let rand_float: f32 = rng.gen();
            if (rand_float < 0.4){
                self.nodes[i].color = [rng.gen(), rng.gen(), rng.gen(), 0];
            }

            if (pos == [0, 0, 0] && depth == OCTTREE_DEPTH){
                self.nodes[i].color = [u16::MAX, u16::MAX, u16::MAX, 0];
            }
        }
        
        return new_i;
    }
}
