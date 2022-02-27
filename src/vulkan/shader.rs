use crate::vulkan::fs;

use super::VulkanApp;

use ash::{vk, Device};

impl VulkanApp{

    pub fn read_shader_from_file<P: AsRef<std::path::Path>>(path: P) -> Vec<u32> {
        log::debug!("Loading shader file {}", path.as_ref().to_str().unwrap());
        let mut cursor = fs::load(path);
        ash::util::read_spv(&mut cursor).unwrap()
    }

    pub fn create_shader_module(device: &Device, code: &[u32]) -> vk::ShaderModule {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(code).build();
        unsafe { device.create_shader_module(&create_info, None).unwrap() }
    }
}