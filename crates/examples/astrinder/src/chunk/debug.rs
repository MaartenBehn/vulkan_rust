use app::glam::{ivec2, vec2, vec3, Vec2, Vec3};

use crate::math::transform::Transform;

use super::ChunkController;

impl ChunkController {
    #[allow(unused_must_use)]
    pub fn send_debug(&self) {
        self.to_debug.send((Vec2::NAN, Vec2::NAN, Vec3::NAN));

        self.debug_chunk_transforms();
        self.debug_colliders();
    }

    #[allow(unused_must_use)]
    fn debug_chunk_transforms(&self) {
        for chunk in self.chunks.iter() {
            let pos = chunk.transform.pos;
            let pos0 = pos + vec2(2.0, 0.0);
            let pos1 = pos + vec2(0.0, 2.0);

            self.to_debug.send((pos, pos0, vec3(0.0, 0.0, 1.0)));
            self.to_debug.send((pos, pos1, vec3(0.0, 0.0, 1.0)));
        }
    }

    #[allow(unused_must_use)]
    fn debug_colliders(&self) {
        for chunk in self.chunks.iter() {
            let collider = &self.physics_controller.collider_set[chunk.collider_handle];
            for (coll_pos, _) in collider.shape().as_compound().unwrap().shapes() {
                let coll_pos = collider.rotation() * coll_pos;
                let pos = vec2(
                    collider.position().translation.x + coll_pos.translation.x,
                    collider.position().translation.y + coll_pos.translation.y,
                );

                let pos0 = pos + vec2(0.3, 0.0);
                let pos1 = pos + vec2(0.0, 0.3);
                self.to_debug.send((pos, pos0, vec3(0.0, 1.0, 0.0)));
                self.to_debug.send((pos, pos1, vec3(0.0, 1.0, 0.0)));
            }
        }
    }
}
