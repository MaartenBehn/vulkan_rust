use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

use crate::math::transform::Transform;

use super::ChunkController;

pub const PLAYER_RB_ID: u128 = u128::MAX;

#[derive(Clone, Copy, Default)]
pub struct Player {
    pub transform: Transform,
    pub rb_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
}

impl Player {
    pub fn new(transform: Transform) -> Self {
        Self {
            transform: transform,
            rb_handle: RigidBodyHandle::default(),
            collider_handle: ColliderHandle::default(),
        }
    }
}

impl ChunkController {
    pub fn add_player(&mut self, transform: Transform) -> &Player {
        let mut player = Player::new(transform);
        self.physics_controller.add_player(&mut player);

        self.player = player;

        &self.player
    }
}
