use app::glam::{vec2, Vec2, vec3, ivec2};

use crate::debug::render::DebugRenderer;

use super::{ChunkController, math::{part_pos_to_world, hex_to_coord}, transform::Transform, CHUNK_PART_SIZE};



impl ChunkController {
    pub fn debug_colliders(&self, debug_renderer: &mut DebugRenderer){
        let push_line = |pos0: Vec2, pos1: Vec2, part_transform: Transform, debug_renderer: &mut DebugRenderer| {
            let angle_vec = Vec2::from_angle(-part_transform.rot);
            let r_pos0 = Vec2::rotate(angle_vec, pos0);
            let r_pos1 = Vec2::rotate(angle_vec, pos1);

            debug_renderer.add_line(r_pos0 + part_transform.pos, r_pos1 + part_transform.pos, vec3(1.0, 0.0, 0.5))
        };

        for chunk in self.chunks.iter() {
            for part in chunk.parts.iter() {
                let part_transform = part_pos_to_world(chunk.transform, part.pos, chunk.render_to_transform);

                for collider in part.colliders.iter() {
                    for i in 0..collider.vertices.len() {
                        let pos0 = if i == 0 {
                            let p = collider.vertices.last().unwrap();
                            vec2(p.x, p.y)
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

    pub fn debug_chunk_transforms(&self, debug_renderer: &mut DebugRenderer){
        for chunk in self.chunks.iter() {
            let pos = chunk.transform.pos;
            let angle_vec = Vec2::from_angle(0.0);
            let pos0 = Vec2::rotate(angle_vec, pos + vec2(2.0, 0.0));
            let pos1 = Vec2::rotate(angle_vec, pos+ vec2(0.0, 2.0));
            debug_renderer.add_line(pos, pos0, vec3(0.0, 0.0, 1.0));
            debug_renderer.add_line(pos, pos1, vec3(0.0, 0.0, 1.0));
        }
    }

    pub fn debug_chunk_velocity(&self, debug_renderer: &mut DebugRenderer){
        for chunk in self.chunks.iter() {
            let dir = chunk.velocity_transform.pos.normalize();
            let pos = chunk.transform.pos;
            
            debug_renderer.add_line(pos, pos + dir * 2.0, vec3(1.0, 0.0, 1.0));
        }
    }

    pub fn debug_parts_borders(&self, debug_renderer: &mut DebugRenderer){
        let offset = vec2(-0.75, -0.5);

        let push_line = |pos0: Vec2, pos1: Vec2, part_transform: Transform, debug_renderer: &mut DebugRenderer| {
            let angle_vec = Vec2::from_angle(-part_transform.rot);
            let r_pos0 = Vec2::rotate(angle_vec, pos0);
            let r_pos1 = Vec2::rotate(angle_vec, pos1);

            debug_renderer.add_line(r_pos0 + part_transform.pos, r_pos1 + part_transform.pos, vec3(0.0, 1.0, 0.0))
        };

        for chunk in self.chunks.iter() {
            for part in chunk.parts.iter() {
                let part_transform = part_pos_to_world(chunk.transform, part.pos, chunk.render_to_transform);
                let pos0 = hex_to_coord(ivec2(0, 0)) + offset;
                let pos1 = hex_to_coord(ivec2(CHUNK_PART_SIZE, 0)) + offset;
                let pos2 = hex_to_coord(ivec2(0, CHUNK_PART_SIZE)) + offset;
                let pos3 = hex_to_coord(ivec2(CHUNK_PART_SIZE, CHUNK_PART_SIZE)) + offset;

                push_line(pos0, pos1, part_transform, debug_renderer);
                push_line(pos1, pos3, part_transform, debug_renderer);
                push_line(pos3, pos2, part_transform, debug_renderer);
                push_line(pos2, pos0, part_transform, debug_renderer);
                
            }
        }
    }
}
