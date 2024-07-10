use octa_force::camera::Camera;
use octa_force::glam::UVec2;
use octa_force::anyhow::Result;
use octa_force::vulkan::{CommandBuffer, Context};
use crate::render::{RenderFunctions, RenderObject};
use crate::rules::Rules;

pub struct RaytraceRenderer {
    
}

impl RaytraceRenderer {
    pub fn new() -> Result<RaytraceRenderer> {
        
        Ok(RaytraceRenderer {
            
        })
    }
}

impl RenderFunctions for RaytraceRenderer {
    fn update(&mut self, camera: &Camera, res: UVec2) -> Result<()>{
        
        Ok(())
    }
    fn on_recreate_swapchain(&mut self, context: &Context, res: UVec2) -> Result<()> {
        
        Ok(())
    }

    fn render(
        &self,
        buffer: &CommandBuffer,
        image_index: usize,
        render_object: &RenderObject,
    ) -> Result<()> {
        
        Ok(())
    }

    fn on_rules_changed(
        &mut self,
        rules: &Rules,
        context: &Context,
        num_frames: usize,
    ) -> Result<()> {
        
        Ok(())
    }
}