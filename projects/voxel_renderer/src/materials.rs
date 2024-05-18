use octa_force::anyhow::Result;
use octa_force::{
    glam::Vec3,
    vulkan::{ash::vk, Buffer, Context},
};

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Material {
    color: Vec3,
    reflective: f32,
}

pub struct MaterialList {
    pub materials: Vec<Material>,
}

pub struct MaterialController {
    pub material_list: MaterialList,
    pub material_buffer: Buffer,
}

impl MaterialController {
    pub fn new(material_list: MaterialList, context: &Context) -> Result<Self> {
        let material_buffer: Buffer = context.create_gpu_only_buffer_from_data(
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &material_list.materials,
        )?;

        Ok(MaterialController {
            material_list,
            material_buffer,
        })
    }
}

impl MaterialList {
    pub fn new(materials: Vec<Material>) -> Self {
        MaterialList { materials }
    }
}

impl Default for MaterialList {
    fn default() -> Self {
        let mut materials = Vec::new();

        for r in 0..255 {
            for g in 0..255 {
                for b in 0..255 {
                    materials.push(Material {
                        color: Vec3::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0),
                        reflective: 1.0,
                    })
                }
            }
        }

        Self::new(materials)
    }
}
