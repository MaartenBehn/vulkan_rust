use indicatif::ProgressBar;
use noise::{NoiseFn, Perlin};
use palette::{
    encoding::{Linear, Srgb},
    rgb::Rgb,
    Gradient, LinSrgb,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::{octtree_node::OcttreeNode, Tree, OCTTREE_CONFIG};

use super::BasicOcttree;

pub struct CreateSparceOcttreeData {
    rng: StdRng,
    perlin: Perlin,
    gradient: Gradient<Rgb<Linear<Srgb>, f64>, Vec<(f64, Rgb<Linear<Srgb>, f64>)>>,
    bar: ProgressBar,
}

impl BasicOcttree {
    pub fn inital_fill_sparse_tree(
        &mut self,
        id: u64,
        depth: u16,
        pos: [u64; 3],
        parent_filled: bool,
        data: &mut CreateSparceOcttreeData,
    ) {
        data.bar.set_position(id);

        let rand_float: f32 = data.rng.gen();
        let filled = parent_filled && rand_float < 0.7 || depth < 2;

        let pos_mult = 0.05;

        let a = data
            .perlin
            .get([
                (pos[0] as f64 * pos_mult) + 0.1,
                (pos[1] as f64 * pos_mult * 2.0) + 0.2,
                (pos[2] as f64 * pos_mult * 3.0) + 0.3,
            ])
            .abs();

        let color = data.gradient.get(a);
        let mat_id = ((color.red * 255.0) as u32) * 255 * 255
            + ((color.green * 255.0) as u32) * 255
            + ((color.blue * 255.0) as u32);

        let is_leaf = !filled || depth >= self.depth;
        self.nodes
            .push(OcttreeNode::new(id, mat_id, depth, is_leaf, !filled));

        if !is_leaf {
            for j in 0..8 {
                let child_index = self.get_child_id(id, j, depth);

                let inverse_depth = u64::pow(2, (self.depth - depth - 1) as u32);

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

impl CreateSparceOcttreeData {
    pub fn new(seed: u64, max_tree_size: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            perlin: Perlin::new(seed as u32),
            gradient: Gradient::new(vec![
                LinSrgb::new(1.0, 0.56, 0.0),
                LinSrgb::new(0.4, 0.4, 0.4),
            ]),
            bar: ProgressBar::new(max_tree_size),
        }
    }
}
