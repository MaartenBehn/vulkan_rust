use app::{glam::Vec3, log};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub const OCTTREE_DEPTH: usize = 2;
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

    color: Vec3,
    data: u32,
}


impl Octtree{
    pub fn new() -> Octtree{
        let mut octtree = Octtree{
            nodes: [OcttreeNode::default(); OCTTREE_NODE_COUNT],
        };

        let mut seed_rng= rand::thread_rng();
        let seed: u64 = seed_rng.gen();

        log::info!("Octtree Seed: {:?}", seed);
        let mut rng = StdRng::seed_from_u64(seed);
       
        octtree.update(0, 0, [0, 0, 0], &mut rng);

        return octtree;
    }

    fn update(&mut self, i: usize, depth: usize, pos: [u32; 3], rng: &mut impl Rng) -> usize {

        let mut new_i = i;
        if depth < OCTTREE_DEPTH {
            for j in 0..8 {
                new_i += 1;
                let child_index = new_i;

                self.nodes[i].children[j] = child_index as u16;
                self.nodes[child_index].data = i as u32;
                //self.nodes[childIndex].child_index = j as u32;

                let inverse_depth = u32::pow(2, (OCTTREE_DEPTH - depth - 1) as u32);
                let new_pos = [
                    pos[0] + OCTTREE_CONFIG[j][0] * inverse_depth, 
                    pos[1] + OCTTREE_CONFIG[j][1] * inverse_depth, 
                    pos[2] + OCTTREE_CONFIG[j][2] * inverse_depth,
                    ];
                
                new_i = self.update(new_i, depth + 1, new_pos, rng);

                if (self.nodes[child_index].color != Vec3::new(0.0, 0.0, 0.0)){
                    self.nodes[i].color = self.nodes[child_index].color;
                }
            }

            //self.nodes[i].pos = self.nodes[self.nodes[i].children[0] as usize].pos

        }else{
            //self.nodes[i].pos = pos;

            let data: f32 = rng.gen();
            if (data < 0.4){
                self.nodes[i].color = Vec3::new(rng.gen(), rng.gen(), rng.gen());
            }

            if (pos == [0, 0, 0] && depth == OCTTREE_DEPTH){
                self.nodes[i].color = Vec3::new(1.0, 1.0, 1.0);
            }
        }

        //self.nodes[i].depth = depth as u32;
        
        return new_i;
    }
}
