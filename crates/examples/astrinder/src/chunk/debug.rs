use app::glam::{vec2, Vec2};

use crate::debug::render::DebugRenderer;

use super::{ChunkController, math::{part_pos_to_world, hex_to_coord}, transform::Transform};



impl ChunkController {
    pub fn debug_colliders(&self, debug_renderer: &mut DebugRenderer){
        let push_point = |pos0: Vec2, pos1: Vec2, part_transform: Transform, debug_renderer: &mut DebugRenderer| {
            let angle_vec = Vec2::from_angle(part_transform.rot);
            let r_pos0 = Vec2::rotate(angle_vec, pos0);
            let r_pos1 = Vec2::rotate(angle_vec, pos1);

            debug_renderer.add_line(r_pos0 + part_transform.pos, r_pos1 + part_transform.pos)
        };

        for chunk in self.chunks.iter() {
            for part in chunk.parts.iter() {
                let part_transform = part_pos_to_world(chunk.transform, part.pos, chunk.render_to_transform);

                let colliders = part.get_colliders();

                for collider in colliders.iter() {

                    for (i, point) in collider.1.iter().enumerate() {
                        if i == 0 {
                            push_point(hex_to_coord(*point) - vec2(0.1, 0.0), hex_to_coord(*point), part_transform, debug_renderer);
                            continue;
                        }
                        push_point(hex_to_coord(collider.1[i - 1]), hex_to_coord(*point), part_transform, debug_renderer);
                    }
                }
            }
        }
    }
}
