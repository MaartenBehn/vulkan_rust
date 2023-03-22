use app::glam::Vec3;
use app::log;
use palette::encoding::{Linear, Srgb};
use palette::rgb::Rgb;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use noise::{NoiseFn, Perlin};
use palette::{Gradient, LinSrgb};
use indicatif::ProgressBar;

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

pub enum OcttreeFill {
    Sphere,
    SpareseTree,
}

#[derive(Clone)]
pub struct Octtree{
    pub nodes: Vec<OcttreeNode>,

    pub depth: usize,
    pub max_size: usize,
    pub size: usize,
}

#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct OcttreeNode {
    // Static node Data (16 byte)
    node_id_0: u32,
    node_id_1: u32,
    mat_id: u32,
    bit_field: u32,
}

pub struct CreateSparceOcttreeData{
    rng: StdRng,
    perlin: Perlin,
    gradient: Gradient<Rgb<Linear<Srgb>, f64>, Vec<(f64, Rgb<Linear<Srgb>, f64>)>>,
    bar: ProgressBar,
}

impl Octtree{
    pub fn new(depth: usize, mut seed: u64, fill_kind: OcttreeFill) -> Self {
        let mut octtree = Octtree{
            nodes: Vec::new(),
            depth: depth,
            max_size: Self::get_max_tree_size(depth),
            size: 0,
        };

        if seed == 0 {
            let mut seed_rng= rand::thread_rng();
            seed = seed_rng.gen();
        }

        log::info!("Octtree Seed: {:?}", seed);
        
        
        match fill_kind {
            OcttreeFill::Sphere => {
                log::info!("Building Sphere.");
                octtree.inital_fill_sphere(0, 0, [0, 0, 0]); 
            },
            OcttreeFill::SpareseTree => {
                log::info!("Building Sparse Octtree:");
                octtree.inital_fill_sparse_tree(0, 0, [0, 0, 0], true, &mut CreateSparceOcttreeData::new(seed, octtree.max_size));
            },
        }

        octtree.size = octtree.nodes.len();

        return octtree;
    }

    pub fn get_max_tree_size(depth: usize) -> usize {
        ((i64::pow(8, (depth + 1) as u32) - 1) / 7)as usize
    }

    pub fn get_child_id(node_id: u32, child_nr: u32, depth: u32) -> u32{
        let child_size = ((1 - i32::pow(8, depth)) / -7) as u32;
        return (node_id + child_size * child_nr + 1) as u32;
    }

    fn inital_fill_sphere(&mut self, i: usize, depth: usize, pos: [u32; 3]) -> usize {

        let radius = f32::powf(2.0, self.depth as f32) / 2.0;
        let dist = Vec3::new(
            pos[0] as f32 - radius, 
            pos[1] as f32 - radius, 
            pos[2] as f32 - radius
        ).length();

        let mut mat_id = 0;
        if dist < radius {
            mat_id = ((pos[0] % 255) * 255 * 255 + (pos[1] % 255) * 255 + (pos[2] % 255)) - 1;
        }

        self.nodes.push(OcttreeNode::new(i as u64, mat_id, depth as u16, depth >= self.depth));

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
                
                new_i = self.inital_fill_sphere(new_i, depth + 1, new_pos);

                let child_material = self.nodes[child_index].mat_id;
                if child_material != 0 {
                    self.nodes[i].mat_id = child_material;
                }
            }
        }

        return new_i;
    }

    fn inital_fill_sparse_tree(
        &mut self, 
        i: usize, 
        depth: usize, 
        pos: [u32; 3],  
        parent_filled: bool,
        data: &mut CreateSparceOcttreeData,
    ) {

        data.bar.set_position(i as u64);

        let rand_float: f32 = data.rng.gen();
        let filled = parent_filled && rand_float > 0.15;

        let pos_mult = 0.05;
        let mat_id = if filled {

            let a = data.perlin.get([
                (pos[0] as f64 * pos_mult) + 0.1, 
                (pos[1] as f64 * pos_mult * 2.0) + 0.2, 
                (pos[2] as f64 * pos_mult * 3.0) + 0.3]).abs();

            let color = data.gradient.get(a);

            ((color.red * 255.0) as u32) * 255 * 255 + ((color.green * 255.0) as u32) * 255 + ((color.blue * 255.0) as u32) 
        }else{
            0
        };

        let is_leaf = !filled || depth >= self.depth;
        self.nodes.push(OcttreeNode::new(i as u64, mat_id, depth as u16, is_leaf));

        if !is_leaf {
            for j in 0..8 {
                
                let inverse_depth = u32::pow(2, (self.depth - depth - 1) as u32);

                let child_index = Self::get_child_id(i as u32, j as u32, (self.depth - depth) as u32) as usize;
                
                let new_pos = [
                    pos[0] + OCTTREE_CONFIG[j][0] * inverse_depth, 
                    pos[1] + OCTTREE_CONFIG[j][1] * inverse_depth, 
                    pos[2] + OCTTREE_CONFIG[j][2] * inverse_depth,
                    ];
                
                self.inital_fill_sparse_tree(child_index, depth + 1, new_pos, filled, data);
            }
        }
    }

}

const UPPER16BITS: u32 = (u16::MAX as u32) << 16;
const LOWER16BITS: u32 = u16::MAX as u32;

impl OcttreeNode{
    fn new(node_id: u64, mat_id: u32, depth: u16, leaf: bool) -> Self {
        let mut node = OcttreeNode{
            node_id_0: 0,
            node_id_1: 0,
            mat_id: mat_id,
            bit_field: 0,
        };

        node.set_node_id(node_id);
        node.set_depth(depth);
        node.set_leaf(leaf);

        node
    }

    fn set_node_id(&mut self, node_id: u64){
        self.node_id_0 = node_id as u32;
        self.node_id_1 = (node_id >> 32) as u32;
    }

    pub fn get_node_id(&self) -> u64{
        (self.node_id_0 as u64) + ((self.node_id_1 as u64) >> 32)
    }

    fn set_depth(&mut self, depth: u16) {
        self.bit_field = (depth as u32) + (self.bit_field & UPPER16BITS);
    }

    pub fn get_depth(&self) -> u16 {
        self.bit_field as u16
    }

    fn set_leaf(&mut self, leaf: bool) {
        self.bit_field = ((leaf as u32) << 16) + (self.bit_field & LOWER16BITS);
    }

    pub fn get_leaf(&self) -> bool {
        ((self.bit_field >> 16) & 1) == 1
    }
}

impl CreateSparceOcttreeData{
    fn new(seed: u64, max_tree_size: usize) -> Self {
        Self { 
            rng: StdRng::seed_from_u64(seed), 
            perlin: Perlin::new(seed as u32), 
            gradient: Gradient::new(vec![
                LinSrgb::new(1.0, 0.56, 0.0),
                LinSrgb::new(0.4, 0.4, 0.4),
            ]),
            bar: ProgressBar::new(max_tree_size as u64),
        }
    }
}

