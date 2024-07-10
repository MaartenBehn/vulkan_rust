use octa_force::vulkan::{Buffer, Context};
use octa_force::anyhow::Result;
use octa_force::glam::Mat4;
use crate::render::{Renderer, RenderObjectFunctions};
use crate::world::block_object::BlockObject;

pub struct ComputeObject {
    chunks: Vec<ComputeChunk>
}

pub struct ComputeChunk {
    transform: Mat4,
}


impl ComputeObject {
    pub fn new() -> Result<ComputeObject> {
        Ok(ComputeObject {
            chunks: vec![]
        })
    }
}

impl RenderObjectFunctions for ComputeObject {
    fn update_from_block_object(&mut self, block_object: &BlockObject, changed_chunks: Vec<usize>, image_index: usize, context: &Context, renderer: &Renderer) -> Result<()> {
        
        Ok(())
    }
}