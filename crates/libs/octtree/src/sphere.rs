use app::glam::{DVec3};

use crate::{Octtree, octtree_node::OcttreeNode, OCTTREE_CONFIG};

impl Octtree{
    pub fn inital_fill_sphere(&mut self, id: u64, depth: u16, pos: [u64; 3]) -> u64 {

        let radius = f64::powf(2.0, self.depth as f64) / 2.0;
        let dist = DVec3::new(
            pos[0] as f64 - radius, 
            pos[1] as f64 - radius, 
            pos[2] as f64 - radius
        ).length();

        let mut mat_id = 0;
        if dist < radius {
            mat_id = (pos[0] % 255) * 255 * 255 + (pos[1] % 255) * 255 + (pos[2] % 255);
        }

        self.nodes.push(OcttreeNode::new(id as u64, mat_id as u32, depth as u16, depth >= self.depth));

        let mut new_id = id + 1;
        if depth < self.depth {
            for j in 0..8 {

                let child_index = self.get_child_id(id, j , depth);

                let inverse_depth = u64::pow(2, (self.depth - depth - 1) as u32);
                
                let new_pos = [
                    pos[0] + OCTTREE_CONFIG[j][0] * inverse_depth, 
                    pos[1] + OCTTREE_CONFIG[j][1] * inverse_depth, 
                    pos[2] + OCTTREE_CONFIG[j][2] * inverse_depth,
                    ];
                
                new_id = self.inital_fill_sphere(new_id, depth + 1, new_pos);

                let child_material = self.nodes[child_index as usize].get_mat_id();
                if child_material != 0 {
                    self.nodes[id as usize].set_mat_id(child_material);
                }
            }
        }

        return new_id;
    }
}