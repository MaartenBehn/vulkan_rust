use super::{VulkanApp};


use std::ffi::CString;
use ash::{Device, vk};

impl VulkanApp{

    pub fn create_compute_pipeline(
        device: &Device,
        descriptor_set_layout: &vk::DescriptorSetLayout,
    ) -> (vk::Pipeline, vk::PipelineLayout){
        
        let comp_source = Self::read_shader_from_file("shaders/mandelblub.comp.spv");
        let comp_shader_module = Self::create_shader_module(device, &comp_source);

        let entry_point_name = CString::new("main").unwrap();
        let comp_shader_state_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(comp_shader_module)
            .name(&entry_point_name)
            .build();

        let layout = {
            let layouts = [descriptor_set_layout.clone()];
            let layout_info = vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&layouts)
                // .push_constant_ranges
                .build();

            unsafe { device.create_pipeline_layout(&layout_info, None).unwrap() }
        };
        
        let pipeline_info = vk::ComputePipelineCreateInfo::builder()
            .stage(comp_shader_state_info)
            .layout(layout)
            .build();
        let pipeline_infos = [pipeline_info];

        let pipeline = unsafe {
            device
                .create_compute_pipelines(vk::PipelineCache::null(), &pipeline_infos, None)
                .unwrap()[0]
        };

        unsafe {
            device.destroy_shader_module(comp_shader_module, None);
        };

        (pipeline, layout)
    }
}