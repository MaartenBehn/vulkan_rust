use ash::vk::{PushConstantRange, ShaderStageFlags};

pub fn create_push_constant_range(stage_flags: ShaderStageFlags, push_constant_size: usize) -> PushConstantRange {
    PushConstantRange::builder()
        .stage_flags(stage_flags)
        .offset(0)
        .size(push_constant_size as u32)
        .build()
}