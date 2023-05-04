use app::glam::{vec2, Vec2, vec3, ivec2, Vec3};

use super::ChunkController;

impl ChunkController {

    #[allow(unused_must_use)]
    pub fn send_debug(&self){
        self.to_debug.send((Vec2::NAN, Vec2::NAN, Vec3::NAN));

        self.debug_chunk_transforms();
        //self.debug_chunk_velocity();
        // self.debug_parts_borders();
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
        
    }

    #[allow(unused_must_use)]
    fn debug_parts_borders(&self){
        
    }
}
