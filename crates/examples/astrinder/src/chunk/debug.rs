use app::glam::{vec2, Vec2};

use crate::debug::render::DebugRenderer;

use super::{ChunkController, math::{part_pos_to_world, hex_to_coord}, transform::Transform};



impl ChunkController {
    pub fn debug_colliders(&self, debug_renderer: &mut DebugRenderer){
        let push_line = |pos0: Vec2, pos1: Vec2, part_transform: Transform, debug_renderer: &mut DebugRenderer| {
            let angle_vec = Vec2::from_angle(part_transform.rot);
            let r_pos0 = Vec2::rotate(angle_vec, pos0);
            let r_pos1 = Vec2::rotate(angle_vec, pos1);

            debug_renderer.add_line(r_pos0 + part_transform.pos, r_pos1 + part_transform.pos)
        };

        for chunk in self.chunks.iter() {
            for part in chunk.parts.iter() {
                let part_transform = part_pos_to_world(chunk.transform, part.pos, chunk.render_to_transform);

                for collider in part.colliders.iter() {
                    for i in 0..collider.vertices.len() {
                        let pos0 = if i == 0 {
                            let p = collider.vertices.last().unwrap();
                            vec2(p.x - 0.1, p.y - 0.1)
                        }else {
                            let p = collider.vertices[i - 1];
                            vec2(p.x, p.y)
                        };

                        let p = collider.vertices[i];
                        let pos1 = vec2(p.x, p.y);

                        push_line(pos0, pos1, part_transform, debug_renderer);


                    }
                }
            }
        }
    }
}
