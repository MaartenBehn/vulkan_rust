use std::mem::size_of;
use std::time::{Duration, Instant};

use octa_force::anyhow::Result;
use octa_force::camera::Camera;
use octa_force::egui::DragValue;
use octa_force::egui_winit::winit::event::WindowEvent;
use octa_force::glam::{uvec2, vec3, Mat4};
use octa_force::gui::Gui;
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    Buffer, BufferBarrier, ComputePipeline, ComputePipelineCreateInfo, Context, DescriptorPool,
    DescriptorSet, DescriptorSetLayout, GraphicsPipeline, GraphicsPipelineCreateInfo,
    GraphicsShaderCreateInfo, PipelineLayout, Vertex, WriteDescriptorSet, WriteDescriptorSetKind,
};
use octa_force::{egui, log, App, BaseApp};
use rand::Rng;

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 576;
const APP_NAME: &str = "GPU Particles";

const DISPATCH_GROUP_SIZE_X: u32 = 256;
const MAX_PARTICLE_COUNT: u32 = DISPATCH_GROUP_SIZE_X * 32_768; // 8M particles
const MIN_PARTICLE_SIZE: f32 = 1.0;
const MAX_PARTICLE_SIZE: f32 = 3.0;
const MIN_ATTRACTOR_STRENGTH: u32 = 0;
const MAX_ATTRACTOR_STRENGTH: u32 = 100;

fn main() -> Result<()> {
    octa_force::run::<Particles>(APP_NAME, uvec2(WIDTH, HEIGHT), false)
}
struct Particles {
    particle_count: u32,
    particle_size: f32,
    attractor_position: [f32; 3],
    new_attractor_position: Option<[f32; 3]>,
    attractor_strength: u32,
    color1: [f32; 4],
    color2: [f32; 4],
    color3: [f32; 4],
    attractor_center: [f32; 3],

    particles_buffer: Buffer,
    compute_ubo_buffer: Buffer,
    _compute_descriptor_pool: DescriptorPool,
    _compute_descriptor_layout: DescriptorSetLayout,
    compute_descriptor_set: DescriptorSet,
    compute_pipeline_layout: PipelineLayout,
    compute_pipeline: ComputePipeline,
    graphics_ubo_buffer: Buffer,
    _graphics_descriptor_pool: DescriptorPool,
    _graphics_descriptor_layout: DescriptorSetLayout,
    graphics_descriptor_set: DescriptorSet,
    graphics_pipeline_layout: PipelineLayout,
    graphics_pipeline: GraphicsPipeline,

    gui: Gui,
    camera: Camera,
}

impl App for Particles {
    fn new(base: &mut BaseApp<Self>) -> Result<Self> {
        let context = &mut base.context;

        let particles_buffer = create_particle_buffer(context)?;
        let compute_ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<ComputeUbo>() as _,
        )?;

        let compute_descriptor_pool = context.create_descriptor_pool(
            1,
            &[
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::STORAGE_BUFFER,
                    descriptor_count: 1,
                },
                vk::DescriptorPoolSize {
                    ty: vk::DescriptorType::UNIFORM_BUFFER,
                    descriptor_count: 1,
                },
            ],
        )?;

        let compute_descriptor_layout = context.create_descriptor_set_layout(&[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
        ])?;

        let compute_descriptor_set =
            compute_descriptor_pool.allocate_set(&compute_descriptor_layout)?;

        compute_descriptor_set.update(&[
            WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::StorageBuffer {
                    buffer: &particles_buffer,
                },
            },
            WriteDescriptorSet {
                binding: 1,
                kind: WriteDescriptorSetKind::UniformBuffer {
                    buffer: &compute_ubo_buffer,
                },
            },
        ]);

        let compute_pipeline_layout =
            context.create_pipeline_layout(&[&compute_descriptor_layout], &[])?;

        let compute_pipeline = context.create_compute_pipeline(
            &compute_pipeline_layout,
            ComputePipelineCreateInfo {
                shader_source: &include_bytes!("../shaders/shader.comp.spv")[..],
            },
        )?;

        let graphics_ubo_buffer = context.create_buffer(
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
            size_of::<GraphicsUbo>() as _,
        )?;

        let graphics_descriptor_pool = context.create_descriptor_pool(
            1,
            &[vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            }],
        )?;

        let graphics_descriptor_layout =
            context.create_descriptor_set_layout(&[vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            }])?;

        let graphics_descriptor_set =
            graphics_descriptor_pool.allocate_set(&graphics_descriptor_layout)?;

        graphics_descriptor_set.update(&[WriteDescriptorSet {
            binding: 0,
            kind: WriteDescriptorSetKind::UniformBuffer {
                buffer: &graphics_ubo_buffer,
            },
        }]);

        let graphics_pipeline_layout =
            context.create_pipeline_layout(&[&graphics_descriptor_layout], &[])?;

        let graphics_pipeline =
            create_graphics_pipeline(context, &graphics_pipeline_layout, base.swapchain.format)?;

        let mut camera = Camera::base(base.swapchain.extent);
        camera.position.z = 2.0;
        camera.z_far = 100.0;

        let gui = Gui::new(
            context,
            base.swapchain.format,
            &base.window,
            base.num_frames,
        )?;

        Ok(Self {
            particle_count: MAX_PARTICLE_COUNT / 20,
            particle_size: MIN_PARTICLE_SIZE,
            attractor_position: [0.0; 3],
            new_attractor_position: None,
            attractor_strength: MAX_ATTRACTOR_STRENGTH / 10,
            color1: [1.0, 0.0, 0.0, 1.0],
            color2: [0.0, 1.0, 0.0, 1.0],
            color3: [0.0, 0.0, 1.0, 1.0],
            attractor_center: [0.0; 3],

            particles_buffer,
            compute_ubo_buffer,
            _compute_descriptor_pool: compute_descriptor_pool,
            _compute_descriptor_layout: compute_descriptor_layout,
            compute_descriptor_set,
            compute_pipeline_layout,
            compute_pipeline,
            graphics_ubo_buffer,
            _graphics_descriptor_pool: graphics_descriptor_pool,
            _graphics_descriptor_layout: graphics_descriptor_layout,
            graphics_descriptor_set,
            graphics_pipeline_layout,
            graphics_pipeline,

            gui,
            camera,
        })
    }

    fn update(
        &mut self,
        base: &mut BaseApp<Self>,
        _image_index: usize,
        delta_time: Duration,
    ) -> Result<()> {
        self.camera.update(&base.controls, delta_time);

        self.attractor_center = self
            .new_attractor_position
            .take()
            .unwrap_or(self.attractor_center);

        self.compute_ubo_buffer.copy_data_to_buffer(&[ComputeUbo {
            attractor_center: [
                self.attractor_center[0],
                self.attractor_center[1],
                self.attractor_center[2],
                0.0,
            ],
            color1: self.color1,
            color2: self.color2,
            color3: self.color3,
            attractor_strength: self.attractor_strength,
            particle_count: self.particle_count,
            elapsed: delta_time.as_secs_f32(),
        }])?;

        self.graphics_ubo_buffer
            .copy_data_to_buffer(&[GraphicsUbo {
                view_proj_matrix: self.camera.projection_matrix() * self.camera.view_matrix(),
                particle_size: self.particle_size,
            }])?;

        Ok(())
    }

    fn record_render_commands(
        &mut self,
        base: &mut BaseApp<Self>,
        image_index: usize,
    ) -> Result<()> {
        let buffer = &base.command_buffers[image_index];

        buffer.bind_compute_pipeline(&self.compute_pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::COMPUTE,
            &self.compute_pipeline_layout,
            0,
            &[&self.compute_descriptor_set],
        );
        buffer.dispatch(self.particle_count / DISPATCH_GROUP_SIZE_X, 1, 1);

        buffer.pipeline_buffer_barriers(&[BufferBarrier {
            buffer: &self.particles_buffer,
            src_access_mask: vk::AccessFlags2::SHADER_WRITE,
            src_stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
            dst_access_mask: vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
            dst_stage_mask: vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT,
        }]);

        buffer.swapchain_image_render_barrier(&base.swapchain.images[image_index])?;

        buffer.begin_rendering(
            &base.swapchain.views[image_index],
            None,
            base.swapchain.extent,
            vk::AttachmentLoadOp::CLEAR,
            Some([0.0, 0.0, 0.0, 1.0]),
        );
        buffer.bind_graphics_pipeline(&self.graphics_pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &self.graphics_pipeline_layout,
            0,
            &[&self.graphics_descriptor_set],
        );
        buffer.bind_vertex_buffer(&self.particles_buffer);
        buffer.set_viewport(base.swapchain.extent);
        buffer.set_scissor(base.swapchain.extent);
        buffer.draw(self.particle_count / DISPATCH_GROUP_SIZE_X * DISPATCH_GROUP_SIZE_X);

        self.gui.cmd_draw(
            buffer,
            base.swapchain.extent,
            image_index,
            &base.window,
            &base.context,
            |ctx| {
                egui::Window::new("Particles").show(ctx, |ui| {
                    ui.label("Particles");

                    ui.add(
                        egui::Slider::new(&mut self.particle_count, 0..=MAX_PARTICLE_COUNT)
                            .text("Count"),
                    );
                    ui.add(
                        egui::Slider::new(
                            &mut self.particle_size,
                            MIN_PARTICLE_SIZE..=MAX_PARTICLE_SIZE,
                        )
                        .text("Size"),
                    );

                    ui.add(
                        egui::Slider::new(
                            &mut self.attractor_strength,
                            MIN_ATTRACTOR_STRENGTH..=MAX_ATTRACTOR_STRENGTH,
                        )
                        .text("Strength"),
                    );

                    ui.label("Transform");
                    ui.columns(3, |ui| {
                        ui[0].add(
                            DragValue::new(&mut self.attractor_position[0])
                                .clamp_range(-1.0..=1.0)
                                .speed(0.01),
                        );
                        ui[1].add(
                            DragValue::new(&mut self.attractor_position[1])
                                .clamp_range(-1.0..=1.0)
                                .speed(0.01),
                        );
                        ui[2].add(
                            DragValue::new(&mut self.attractor_position[2])
                                .clamp_range(-1.0..=1.0)
                                .speed(0.01),
                        );
                    });

                    if ui.button("Apply").clicked() {
                        self.new_attractor_position = Some(self.attractor_position);
                    }
                });
            },
        )?;

        buffer.end_rendering();

        Ok(())
    }

    fn on_window_event(&mut self, base: &mut BaseApp<Self>, event: &WindowEvent) -> Result<()> {
        self.gui.handle_event(&base.window, event);

        Ok(())
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ComputeUbo {
    attractor_center: [f32; 4],
    color1: [f32; 4],
    color2: [f32; 4],
    color3: [f32; 4],
    attractor_strength: u32,
    particle_count: u32,
    elapsed: f32,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct GraphicsUbo {
    view_proj_matrix: Mat4,
    particle_size: f32,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Particle {
    // position 0, 1, 2 - pad 3
    position: [f32; 4],
    // velocity 0, 1, 2 - pad 3
    velocity: [f32; 4],
    color: [f32; 4],
}

impl Vertex for Particle {
    fn bindings() -> Vec<vk::VertexInputBindingDescription> {
        vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: 48,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attributes() -> Vec<vk::VertexInputAttributeDescription> {
        vec![
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 32,
            },
        ]
    }
}

fn create_particle_buffer(context: &Context) -> Result<Buffer> {
    let start = Instant::now();

    let num_cpus = num_cpus::get();
    let particles_per_cpu = (MAX_PARTICLE_COUNT as f32 / num_cpus as f32).ceil() as usize;
    let remaining = MAX_PARTICLE_COUNT as usize % particles_per_cpu;

    let mut handles = vec![];
    for i in 0..num_cpus {
        handles.push(std::thread::spawn(move || {
            let mut rng = rand::thread_rng();

            let particle_count = if i == num_cpus - 1 && remaining != 0 {
                remaining
            } else {
                particles_per_cpu
            };

            let mut particles = Vec::with_capacity(particle_count);

            for _ in 0..particle_count {
                let p = vec3(
                    rng.gen_range(-1.0..1.0f32),
                    rng.gen_range(-1.0..1.0f32),
                    rng.gen_range(-1.0..1.0f32),
                )
                .normalize()
                    * rng.gen_range(0.1..1.0f32);

                particles.push(Particle {
                    position: [p.x, p.y, p.z, 0.0],
                    velocity: [
                        rng.gen_range(-1.0..1.0f32),
                        rng.gen_range(-1.0..1.0f32),
                        rng.gen_range(-1.0..1.0f32),
                        0.0,
                    ],
                    color: [1.0, 1.0, 1.0, 1.0],
                });
            }

            particles
        }));
    }

    let particles = handles
        .into_iter()
        .map(|h| h.join())
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let vertex_buffer = context.create_gpu_only_buffer_from_data(
        vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::STORAGE_BUFFER,
        &particles,
    )?;

    let time = Instant::now() - start;
    log::info!("Generated particles in {time:?}");

    Ok(vertex_buffer)
}

fn create_graphics_pipeline(
    context: &Context,
    layout: &PipelineLayout,
    color_attachement_format: vk::Format,
) -> Result<GraphicsPipeline> {
    context.create_graphics_pipeline::<Particle>(
        layout,
        GraphicsPipelineCreateInfo {
            shaders: &[
                GraphicsShaderCreateInfo {
                    source: &include_bytes!("../shaders/shader.vert.spv")[..],
                    stage: vk::ShaderStageFlags::VERTEX,
                },
                GraphicsShaderCreateInfo {
                    source: &include_bytes!("../shaders/shader.frag.spv")[..],
                    stage: vk::ShaderStageFlags::FRAGMENT,
                },
            ],
            primitive_topology: vk::PrimitiveTopology::POINT_LIST,
            extent: None,
            color_attachment_format: color_attachement_format,
            color_attachment_blend: Some(vk::PipelineColorBlendAttachmentState {
                blend_enable: vk::TRUE,
                src_color_blend_factor: vk::BlendFactor::ONE,
                dst_color_blend_factor: vk::BlendFactor::ONE,
                color_blend_op: vk::BlendOp::ADD,
                src_alpha_blend_factor: vk::BlendFactor::ONE,
                dst_alpha_blend_factor: vk::BlendFactor::ZERO,
                alpha_blend_op: vk::BlendOp::ADD,
                color_write_mask: vk::ColorComponentFlags::RGBA,
            }),
            depth_attachment_format: None,
            dynamic_states: Some(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT]),
        },
    )
}
