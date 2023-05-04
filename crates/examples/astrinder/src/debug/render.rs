use std::{mem::{size_of, align_of}, sync::mpsc::Receiver};

use app::{glam::{Vec2,  Vec3}, vulkan::{Context, Buffer, ash::vk::{self, Extent2D, ColorComponentFlags, BlendOp, BlendFactor}, PipelineLayout, GraphicsPipeline, GraphicsPipelineCreateInfo, GraphicsShaderCreateInfo, CommandBuffer, gpu_allocator::MemoryLocation, WriteDescriptorSet, WriteDescriptorSetKind, DescriptorPool, DescriptorSetLayout, DescriptorSet}, anyhow::Ok};
use app::anyhow::Result;

use crate::{camera::Camera, settings::Settings, render::vulkan::RenderUBO};

pub struct DebugRenderer {
    max_lines: usize,

    lines: Vec<Vertex>,

    vertex_buffer: Buffer,
    _render_ubo: Buffer,

    _descriptor_pool: DescriptorPool,
    _descriptor_layout: DescriptorSetLayout,
    descriptor_sets: Vec<DescriptorSet>,

    _pipeline_layout: PipelineLayout,
    pipeline: GraphicsPipeline,

    from_controller: Receiver<(Vec2, Vec2, Vec3)>
}

impl DebugRenderer {
    pub fn new (
        context: &Context,
        color_attachment_format: vk::Format,
        images_len: u32,
        from_controller: Receiver<(Vec2, Vec2, Vec3)>,
        settings: Settings,
    ) -> Result<Self> {
       
        let vertex_buffer = context.create_buffer(
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (size_of::<Vertex>() * settings.max_lines * 2) as _,
        )?;

        let render_ubo = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<RenderUBO>() as _,
        )?;

        let descriptor_pool = context.create_descriptor_pool(
            images_len * 1,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: images_len,
                },
            ],
        )?;

        let descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ])?;

        let mut descriptor_sets = Vec::new();
        for _ in 0..images_len {
            let render_descriptor_set =
            descriptor_pool.allocate_set(&descriptor_layout)?;

            render_descriptor_set.update(&[
                WriteDescriptorSet {
                    binding: 0,
                    kind: WriteDescriptorSetKind::UniformBuffer {
                        buffer: &render_ubo,
                    },
                },
            ]);
            descriptor_sets.push(render_descriptor_set);
        }

        let pipeline_layout = context.create_pipeline_layout(&[&descriptor_layout])?;

        let color_blending = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(ColorComponentFlags::RGBA)
            .blend_enable(false)

            .src_color_blend_factor(BlendFactor::ONE)
            .dst_color_blend_factor(BlendFactor::ONE)
            .color_blend_op(BlendOp::MIN)

            .src_alpha_blend_factor(BlendFactor::ONE)
            .dst_alpha_blend_factor(BlendFactor::ZERO)
            .alpha_blend_op(BlendOp::ADD)
            .build();

        let pipeline = context.create_graphics_pipeline::<Vertex>(
            &pipeline_layout,
            GraphicsPipelineCreateInfo {
                shaders: &[
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../shaders/debug.vert.spv")[..],
                        stage: vk::ShaderStageFlags::VERTEX,
                    },
                    GraphicsShaderCreateInfo {
                        source: &include_bytes!("../../shaders/debug.frag.spv")[..],
                        stage: vk::ShaderStageFlags::FRAGMENT,
                    },
                ],
                primitive_topology: vk::PrimitiveTopology::LINE_LIST,
                extent: None,
                color_attachment_format,
                color_attachment_blend: Some(color_blending),
                dynamic_states: Some(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]),
            },
        )?;

        Ok(Self { 
            max_lines: settings.max_lines,

            lines: Vec::new(),

            vertex_buffer: vertex_buffer,
            _render_ubo: render_ubo, 

            _descriptor_pool: descriptor_pool,
            _descriptor_layout: descriptor_layout,
            descriptor_sets: descriptor_sets,

            _pipeline_layout: pipeline_layout,
            pipeline: pipeline, 

            from_controller,
        })
    }

    fn add_line (&mut self, x: Vec2, y: Vec2, color: Vec3){
        self.lines.push(Vertex::new(x, color));
        self.lines.push(Vertex::new(y, color));
    }

    fn clear_lines (&mut self){
        self.lines.clear();
    }

    pub fn update (
        &mut self, 
        camera: &Camera,
    ) -> Result<()>{

        loop {
            let result = self.from_controller.try_recv();
            if result.is_err() {
                break;
            }

            let (x, y, color) = result.unwrap();

            if x.is_nan() && y.is_nan() && color.is_nan() {
                self.clear_lines();
                continue;
            }

            self.add_line(x, y, color);
        }

        let left_lines = (self.max_lines * 2) as i32 - self.lines.len() as i32;
        if left_lines > 0 {
            for _ in 0..left_lines {
                self.lines.push(Vertex::new(Vec2::ZERO, Vec3::ZERO));
            }
        }
            
        self.vertex_buffer.copy_data_to_buffer(&self.lines, 0, align_of::<Vertex>())?;

        self._render_ubo.copy_data_to_buffer(&[RenderUBO::new(camera.to_owned())], 0, 16)?;
        
        Ok(())
    }

    pub fn render(
        &self, 
        buffer: &CommandBuffer,
        image_index: usize,
        extent: Extent2D,
    ){
        buffer.bind_graphics_pipeline(&self.pipeline);
        buffer.bind_vertex_buffer(&self.vertex_buffer);

        buffer.set_viewport(extent);
        buffer.set_scissor(extent);

        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self._pipeline_layout,
            0,
            &[&self.descriptor_sets[image_index]],
        );

        buffer.draw(self.max_lines as u32);
    }
}


#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Vertex {
    pos: Vec2,
    color: Vec3,
}

impl Vertex {
    fn new (pos: Vec2, color: Vec3) -> Self {
        Self { pos, color }
    }
}

impl app::vulkan::Vertex for Vertex {
    fn bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 8,
            },
        ]
    }
}