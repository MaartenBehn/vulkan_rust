use app::glam::{ivec2, vec2, Vec2};
use rapier2d::{
    crossbeam::{self, channel::Receiver},
    prelude::*,
};

use crate::{
    chunk::{chunk::Chunk, ChunkController},
    math::transform::Transform,
    settings::Settings,
};

use self::destruction::DestructionSolver;

use super::IdCounter;

pub mod collider;
pub mod destruction;

pub struct PhysicsController {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,

    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,

    collision_recv: Receiver<CollisionEvent>,
    contact_force_recv: Receiver<ContactForceEvent>,
    event_handler: ChannelEventCollector,

    destruction_solver: DestructionSolver,
}

impl PhysicsController {
    pub fn new() -> Self {
        let rigid_body_set = RigidBodySet::new();
        let collider_set = ColliderSet::new();

        /* Create other structures necessary for the simulation. */
        let integration_parameters = IntegrationParameters::default();
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = BroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();

        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, contact_force_recv) = crossbeam::channel::unbounded();
        let event_handler = ChannelEventCollector::new(collision_send, contact_force_send);

        let destruction_solver = DestructionSolver::new();

        Self {
            rigid_body_set,
            collider_set,

            integration_parameters,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,

            collision_recv,
            contact_force_recv,
            event_handler,

            destruction_solver,
        }
    }

    pub fn add_chunk(&mut self, chunk: &mut Chunk, vel: Transform) -> RigidBodyHandle {
        let pos = chunk.transform.pos;
        let rot = chunk.transform.rot;

        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![pos.x, pos.y])
            .rotation(rot)
            .linvel(vector![vel.pos.x, vel.pos.y])
            .angvel(vel.rot)
            //.linear_damping(0.8)
            //.angular_damping(0.9)
            //.lock_rotations()
            .user_data(chunk.id as _)
            .build();

        let rb_handle = self.rigid_body_set.insert(rb);

        rb_handle
    }

    pub fn remove_chunk(&mut self, chunk: &Chunk) {
        self.rigid_body_set.remove(
            chunk.rb_handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
    }

    pub fn step(&mut self) {
        let gravity = vector![0.0, 0.0];

        self.physics_pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None,
            &(),
            &self.event_handler,
        );

        while let Ok(collision_event) = self.collision_recv.try_recv() {}
    }

    pub fn update_chunk(&mut self, chunk: &mut Chunk) {
        let rb = &mut self.rigid_body_set[chunk.rb_handle];
        let pos = rb.translation();
        let rot = rb.rotation();

        chunk.transform.pos = vec2(pos.x, pos.y);
        chunk.transform.rot = rot.angle();

        chunk.update_part_tranforms();

        rb.add_force(vector![chunk.forces.x, chunk.forces.y], true);

        chunk.forces = Vec2::ZERO;
    }

    pub fn get_velocity(&self, chunk: &Chunk) -> Transform {
        let rb = &self.rigid_body_set[chunk.rb_handle];
        Transform::new(vec2(rb.linvel().x, rb.linvel().y), rb.angvel())
    }
}

impl ChunkController {
    pub fn update_gravity(&mut self) {
        let l = self.chunks.len();
        for i in 0..l {
            for j in (i + 1)..l {
                let chunk0 = &self.chunks[i];
                let chunk1 = &self.chunks[j];

                let diff = chunk0.transform.pos - chunk1.transform.pos;
                let dist = diff.length();
                let force = f32::min(
                    (self.settings.gravity_factor * chunk0.mass * chunk1.mass) / (dist * dist),
                    self.settings.gravity_max_force,
                );

                let force = diff * (1.0 / dist) * force;

                self.chunks[i].forces += -force;
                self.chunks[j].forces += force;
            }
        }
    }
}
