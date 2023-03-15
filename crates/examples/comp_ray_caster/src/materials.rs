use app::{glam::Vec3, vulkan::{Buffer, Context, utils::create_gpu_only_buffer_from_data, ash::vk}};
use app::anyhow::Result;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Material{
    color: Vec3,
    reflective: f32,
}

pub struct MaterialController{
    pub materials: Vec<Material>,
    pub material_buffer: Option<Buffer>,
}


impl MaterialController{
    pub fn new(materials: Vec<Material>) -> Self{
        MaterialController { 
            materials: materials, 
            material_buffer: None
        }
    }

    pub fn create_buffer(& mut self, context: &Context) -> Result<()> {

        let buffer: Buffer = create_gpu_only_buffer_from_data(context, vk::BufferUsageFlags::STORAGE_BUFFER, &self.materials)?;
        self.material_buffer = Some(buffer);

        Ok(())
    }
}

impl Default for MaterialController{
    fn default() -> Self {
        let mut materials = Vec::new();

        for r in 0..255 {
            for g in 0..255 {
                for b in 0..255 {
                    materials.push(Material { 
                        color: Vec3::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0), 
                        reflective: 1.0 
                    }
                    )
                }
            }
        }

        Self::new(materials)
    }
}

