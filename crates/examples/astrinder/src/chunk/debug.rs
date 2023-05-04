use app::glam::{vec2, Vec2, vec3, ivec2, Vec3};

use crate::math::transform::Transform;

use super::ChunkController;

impl ChunkController {

    #[allow(unused_must_use)]
    pub fn send_debug(&self){
        self.to_debug.send((Vec2::NAN, Vec2::NAN, Vec3::NAN));

        self.debug_chunk_transforms();
        self.debug_colliders();
    }

    #[allow(unused_must_use)]
    fn debug_chunk_transforms(&self){
        for chunk in self.chunks.iter() {
            let pos = chunk.transform.pos;
            let angle_vec = Vec2::from_angle(0.0);
            let pos0 = Vec2::rotate(angle_vec, pos + vec2(2.0, 0.0));
            let pos1 = Vec2::rotate(angle_vec, pos+ vec2(0.0, 2.0));
            self.to_debug.send((pos, pos0, vec3(0.0, 0.0, 1.0)));
            self.to_debug.send((pos, pos1, vec3(0.0, 0.0, 1.0)));
        }
    }

    #[allow(unused_must_use)]
    fn debug_colliders(&self){
        let push_line = |pos0: Vec2, pos1: Vec2, part_transform: Transform| {
            let angle_vec = Vec2::from_angle(-part_transform.rot);
            let r_pos0 = Vec2::rotate(angle_vec, pos0);
            let r_pos1 = Vec2::rotate(angle_vec, pos1);

            self.to_debug.send((r_pos0 + part_transform.pos, r_pos1 + part_transform.pos, vec3(1.0, 0.0, 0.5)));
        };

        for chunk in self.chunks.iter() {

            let mut chunk_transform = chunk.transform;

            let collider = &self.physics_controller.collider_set[chunk.collider_handle];
            for (_, shape) in collider.shape().as_compound().unwrap().shapes() {

                let vertices = shape.as_convex_polygon().unwrap().points();
                for i in 0..vertices.len() {

                    let pos0 = if i == 0 {
                        let p = vertices.last().unwrap();
                        vec2(p.x, p.y)
                    }else {
                        let p = vertices[i - 1];
                        vec2(p.x, p.y)
                    };

                    let p = vertices[i];
                    let pos1 = vec2(p.x, p.y);

                    push_line(pos0, pos1, chunk_transform);

                }
            }
        }
    }
}


