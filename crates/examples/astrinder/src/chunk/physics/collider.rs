use app::glam::{ivec2, IVec2};
use rapier2d::prelude::*;

use crate::{chunk::CHUNK_PART_SIZE, math::*};

use super::{Chunk, PhysicsController};

impl PhysicsController {
    pub fn update_collider(&mut self, chunk: &mut Chunk) {
        self.collider_set.remove(
            chunk.collider_handle,
            &mut self.island_manager,
            &mut self.rigid_body_set,
            false,
        );

        let mut shapes = Vec::new();
        for part in chunk.parts.iter() {
            for x in 0..CHUNK_PART_SIZE {
                for y in 0..CHUNK_PART_SIZE {
                    let hex = ivec2(x, y);
                    if part.particles[hex_to_particle_index(hex)].material != 0 {
                        let pos = hex_in_chunk_frame(hex, part.pos);

                        shapes.push((
                            Isometry::new(vector![pos.x, pos.y], 0.0),
                            SharedShape::ball(0.55),
                        ));
                    }
                }
            }
        }

        let compound_collider =
            ColliderBuilder::compound(shapes).active_events(ActiveEvents::CONTACT_FORCE_EVENTS);

        chunk.collider_handle = self.collider_set.insert_with_parent(
            compound_collider,
            chunk.rb_handle,
            &mut self.rigid_body_set,
        );
    }
}
