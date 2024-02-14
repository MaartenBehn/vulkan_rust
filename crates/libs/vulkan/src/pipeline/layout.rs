use std::sync::Arc;

use anyhow::Result;
use ash::vk::{self, PushConstantRange};

use crate::{device::Device, Context, DescriptorSetLayout};

pub struct PipelineLayout {
    device: Arc<Device>,
    pub(crate) inner: vk::PipelineLayout,
}

impl PipelineLayout {
    pub(crate) fn new(
        device: Arc<Device>,
        descriptor_set_layouts: &[&DescriptorSetLayout],
        push_constant_ranges: &[PushConstantRange],
    ) -> Result<Self> {
        let layouts = descriptor_set_layouts
            .iter()
            .map(|l| l.inner)
            .collect::<Vec<_>>();

        let pipe_layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&layouts)
            .push_constant_ranges(push_constant_ranges);
        let inner = unsafe {
            device
                .inner
                .create_pipeline_layout(&pipe_layout_info, None)?
        };

        Ok(Self { device, inner })
    }
}

impl Context {
    pub fn create_pipeline_layout(
        &self,
        descriptor_set_layouts: &[&DescriptorSetLayout],
        push_constant_ranges: &[PushConstantRange],
    ) -> Result<PipelineLayout> {
        PipelineLayout::new(self.device.clone(), descriptor_set_layouts, push_constant_ranges)
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe { self.device.inner.destroy_pipeline_layout(self.inner, None) };
    }
}
