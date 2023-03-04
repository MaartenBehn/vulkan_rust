use app::glam::Vec3;
use rand::Rng;

pub const OCTTREE_DEPTH: usize = 4;
pub const OCTTREE_SIZE: usize = 16;
pub const OCTTREE_NODE_COUNT: usize = 4681;

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

    pos: [u32; 3],
    depth: u32,
    
    color: Vec3,
    parent: u32,

    child_index: u32,
    fill_0: u32,
    fill_1: u32,
    fill_2: u32,
}


impl Octtree{
    pub fn new() -> Octtree{
        let mut octtree = Octtree{
            nodes: [OcttreeNode::default(); OCTTREE_NODE_COUNT],
        };

        let mut rng= rand::thread_rng();
        octtree.update(0, 0, [0, 0, 0], &mut rng);

        return octtree;
    }

    fn update(&mut self, i: usize, depth: usize, pos: [u32; 3], rng: &mut impl Rng) -> usize {

        let mut new_i = i;
        if depth < OCTTREE_DEPTH {
            for j in 0..8 {
                new_i += 1;

                self.nodes[i].children[j] = new_i as u16;
                self.nodes[new_i].parent = i as u32;
                self.nodes[new_i].child_index = j as u32;

                let inverse_depth = u32::pow(2, (OCTTREE_DEPTH - depth - 1) as u32);
                let new_pos = [
                    pos[0] + OCTTREE_CONFIG[j][0] * inverse_depth, 
                    pos[1] + OCTTREE_CONFIG[j][1] * inverse_depth, 
                    pos[2] + OCTTREE_CONFIG[j][2] * inverse_depth,
                    ];
                
                new_i = self.update(new_i, depth + 1, new_pos, rng);

                if (self.nodes[new_i].color != Vec3::new(0.0, 0.0, 0.0)){
                    self.nodes[i].color = self.nodes[new_i].color;
                }
            }

            self.nodes[i].pos = self.nodes[self.nodes[i].children[0] as usize].pos

        }else{
            let data: f32 = rng.gen();
            if (data < 0.1){
                self.nodes[i].color = Vec3::new(rng.gen(), rng.gen(), rng.gen());
            }

            self.nodes[i].pos = pos;
        }

        self.nodes[i].depth = depth as u32;
        
        return new_i;
    }
}
