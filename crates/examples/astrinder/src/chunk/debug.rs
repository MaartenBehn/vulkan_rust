use app::glam::{vec2, Vec2, vec3, ivec2, Vec3};

use super::{ChunkController, math::{part_pos_to_world, hex_to_coord, hex_to_particle_index}, transform::Transform, CHUNK_PART_SIZE, particle};


impl ChunkController {

    #[allow(unused_must_use)]
    pub fn send_debug(&self){
        self.to_debug.send((Vec2::NAN, Vec2::NAN, Vec3::NAN));

        self.debug_chunk_transforms();
        self.debug_chunk_velocity();
        self.debug_parts_borders();
        self.debug_colliders();
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

                        push_line(pos0, pos1, part_transform);


                    }
                }
            }
        }
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
    fn debug_chunk_velocity(&self){
        for chunk in self.chunks.iter() {
            let dir = chunk.velocity_transform.pos.normalize();
            let pos = chunk.transform.pos;
            
            self.to_debug.send((pos, pos + dir * 2.0, vec3(1.0, 0.0, 1.0)));
        }
    }

    #[allow(unused_must_use)]
    fn debug_parts_borders(&self){
        let offset = vec2(-0.75, -0.5);

        let push_line = |pos0: Vec2, pos1: Vec2, part_transform: Transform| {
            let angle_vec = Vec2::from_angle(-part_transform.rot);
            let r_pos0 = Vec2::rotate(angle_vec, pos0);
            let r_pos1 = Vec2::rotate(angle_vec, pos1);

            self.to_debug.send((r_pos0 + part_transform.pos, r_pos1 + part_transform.pos, vec3(0.0, 1.0, 0.0)));
        };

        for chunk in self.chunks.iter() {
            for part in chunk.parts.iter() {
                let part_transform = part_pos_to_world(chunk.transform, part.pos, chunk.render_to_transform);
                let pos0 = hex_to_coord(ivec2(0, 0)) + offset;
                let pos1 = hex_to_coord(ivec2(CHUNK_PART_SIZE, 0)) + offset;
                let pos2 = hex_to_coord(ivec2(0, CHUNK_PART_SIZE)) + offset;
                let pos3 = hex_to_coord(ivec2(CHUNK_PART_SIZE, CHUNK_PART_SIZE)) + offset;

                push_line(pos0, pos1, part_transform);
                push_line(pos1, pos3, part_transform);
                push_line(pos3, pos2, part_transform);
                push_line(pos2, pos0, part_transform);
                
            }
        }
    }
}
