use app::glam::Vec3;
use rand::Rng;

pub const OCTTREE_DEPTH: usize = 4;
pub const OCTTREE_SIZE: usize = 32;
pub const OCTTREE_NODE_COUNT: usize = 4681;

pub struct Octtree{
    nodes: [OcttreeNode; OCTTREE_NODE_COUNT]
}

#[derive(Clone, Copy)]
pub struct OcttreeNode {
    children: [u16; 8],
    parent: u16,
    data: u16,
    color: Vec3,
}

impl Default for OcttreeNode {
    fn default() -> Self {
        Self { 
            children: Default::default(), 
            parent: Default::default(),
            data: Default::default(),
            color: Vec3::new(1.0, 0.0, 0.0),
        }
    }
}


impl Octtree{
    pub fn new() -> Octtree{
        let mut octtree = Octtree{
            nodes: [OcttreeNode::default(); OCTTREE_NODE_COUNT],
        };

        let mut rng= rand::thread_rng();
        octtree.update(0, 0, &mut rng);

        return octtree;
    }

    fn update(&mut self, i: usize, depth: usize, rng: &mut impl Rng) -> usize {

        let mut new_i = i;
        if depth < OCTTREE_DEPTH {
            for j in 0..8 {

                new_i += 1;
                self.nodes[i].children[j] = new_i as u16;
                self.nodes[new_i].parent = i as u16;

                new_i = self.update(new_i, depth + 1, rng);

                if self.nodes[self.nodes[i].children[j] as usize].data == 1 {
                    self.nodes[i].data = 1
                }
            }
        }else{
            let data: bool = rng.gen();
            self.nodes[i].data = data as u16;
        }
        
        return new_i;
    }
}
