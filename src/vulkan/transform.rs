use std::ops::{Add, Mul};

use cgmath::{Vector3, Matrix4, Deg, Euler, Vector2, vec3, SquareMatrix, ElementWise, Vector4, vec4};

#[derive(Clone, Copy)]
pub struct Transform {
    position: Vector3<f32>,
    rotation: Vector3<f32>,
    scale: Vector3<f32>,

    rotation_matrix: Matrix4<f32>,
	matrix: Matrix4<f32>,

    needs_rotation_matrix_update: bool,
	needs_matrix_update: bool
}

impl Transform {
    pub fn new(position: Vector3<f32>, rotation: Vector3<f32>, scale: Vector3<f32>) -> Self {
        Self {
            position,
            rotation,
            scale,
            
            rotation_matrix: Matrix4::from_scale(1.0),
            matrix: Matrix4::from_scale(1.0),

            needs_rotation_matrix_update: true,
            needs_matrix_update: true,
        }
    }

    pub fn get_position(&mut self) -> Vector3<f32> {
        return self.position;
    }

    pub fn set_position(&mut self, position: Vector3<f32>) -> Transform {
        self.position = position;

        self.needs_matrix_update = true;

        *self
    }

    pub fn move_relative(&mut self, dir:Vector3<f32>) -> Transform{
        if self.rotation != vec3(0.0,0.0,0.0) {
            let rot_dir = self.get_rotation_matrix() * vec4(dir.x, dir.y, dir.z, 1.0);
            self.position.x += rot_dir.x;
            self.position.y += rot_dir.y;
            self.position.z += rot_dir.z;
        }
        else{
            self.position += dir
        }

        self.needs_matrix_update = true;

        *self
    }

    pub fn get_rotation(&mut self) -> Vector3<f32> {
        return self.rotation;
    }

    pub fn set_rotation(&mut self, rotation: Vector3<f32>) -> Transform {
        self.rotation = rotation;

        self.needs_rotation_matrix_update = true;

        *self
    }

    pub fn rotate(&mut self, rotation: Vector3<f32>) -> Transform {
        self.rotation += rotation;

        self.needs_rotation_matrix_update = true;

        *self
    }

    pub fn get_rotation_matrix(&mut self) -> Matrix4<f32> {
        self.update_rotation_matrix();
        return self.rotation_matrix;
    }

    pub fn set_scale(&mut self, scale: Vector3<f32>){
        self.scale = scale;
    }

    pub fn get_scale(&self) -> Vector3<f32> {
        self.scale
    }

    fn update_rotation_matrix(&mut self){
        if !self.needs_rotation_matrix_update {
            return;
        }

		self.rotation_matrix = Matrix4::from_angle_z(Deg(self.rotation[2])) * 
            Matrix4::from_angle_y(Deg(self.rotation[1])) * 
            Matrix4::from_angle_x(Deg(self.rotation[0]));

		self.needs_rotation_matrix_update = false;
    }

    fn update_matrix(&mut self){
        if !self.needs_matrix_update && !self.needs_rotation_matrix_update {
            return;
        }

        self.matrix = Matrix4::from_translation(self.position) * 
            self.get_rotation_matrix() * 
            Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        
        self.needs_matrix_update = false;
    }

    pub fn get_matrix(&mut self) -> Matrix4<f32> {
        self.update_matrix();
        return self.matrix;
    }

}


impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
            rotation: Vector3 { x: 0.0, y: 0.0, z: 0.0 },
            scale: Vector3 { x: 1.0, y: 1.0, z: 1.0 },
            
            rotation_matrix: Matrix4::from_scale(1.0),
            matrix: Matrix4::from_scale(1.0),

            needs_rotation_matrix_update: true,
            needs_matrix_update: true,
        }
    }
}