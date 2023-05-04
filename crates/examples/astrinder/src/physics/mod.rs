use app::glam::{vec2, Vec2};
use rapier2d::prelude::*;

use crate::chunk::{chunk::Chunk, ChunkController};



pub struct PhysicsController{
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,

    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
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
            ccd_solver  
        }
    }

    pub fn add_chunk(&mut self, chunk: &mut Chunk) -> RigidBodyHandle {

        let pos = chunk.transform.pos;
        let rot = chunk.transform.rot;

        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![pos.x, pos.y])
            .rotation(rot)
            //.linear_damping(0.8)
            //.angular_damping(0.9)
            .build();

        let rb_handle = self.rigid_body_set.insert(rb);

        let collider = ColliderBuilder::ball(0.5)
            .mass(chunk.mass)
            .restitution(0.3)
            .friction(0.5)
            .build();
        self.collider_set.insert_with_parent(collider, rb_handle, &mut self.rigid_body_set);

        rb_handle
    }

    pub fn step (&mut self){

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
            &(),
          );
    }

    pub fn update_chunk(&mut self, chunk: &mut Chunk) {
        let rb = &mut self.rigid_body_set[chunk.rb_handle];
        let pos = rb.translation();
        let rot = rb.rotation();
        
        chunk.transform.pos = vec2(pos.x, pos.y);
        chunk.transform.rot = rot.re;

        chunk.update_part_tranforms();

        rb.add_force(vector![chunk.forces.x, chunk.forces.y], true);

        chunk.forces = Vec2::ZERO;
    }
}

impl ChunkController {
    pub fn update_gravity(&mut self){

        let l = self.chunks.len();
        for i in 0..l {
            for j in (i+1)..l {
                let chunk0 = &self.chunks[i];
                let chunk1 = &self.chunks[j];

                let diff = chunk0.transform.pos - chunk1.transform.pos;
                let dist = diff.length();
                let force = f32::min((self.settings.gravity_factor * chunk0.mass * chunk1.mass) 
                    / (dist * dist), self.settings.gravity_max_force);

                let force = diff * (1.0 / dist) * force;

                self.chunks[i].forces += -force;
                self.chunks[j].forces += force;
            }
        }
    }
}