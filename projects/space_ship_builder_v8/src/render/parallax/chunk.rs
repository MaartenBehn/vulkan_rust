use block_mesh::ndshape::ConstShape3u32;
use block_mesh::{
    greedy_quads, Axis, AxisPermutation, GreedyQuadsBuffer, MergeVoxel, OrientedBlockFace,
    QuadCoordinateConfig, Voxel, VoxelVisibility,
};
use log::error;
use octa_force::anyhow::bail;
use octa_force::egui::emath::Numeric;
use octa_force::glam::{ivec3, uvec3, IVec3};
use octa_force::vulkan::ash::vk::{BufferUsageFlags, DeviceSize};
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    DescriptorPool, DescriptorSet, DescriptorSetLayout, WriteDescriptorSet, WriteDescriptorSetKind,
};
use octa_force::{
    anyhow::Result,
    log,
    vulkan::{Buffer, Context},
};
use std::mem::size_of;
use std::{iter, mem};

use crate::render::parallax::renderer::Vertex;

pub const MIN_VERTICES: usize = 8;
pub const MIN_INDICES: usize = 20;

pub struct ParallaxData {
    pub pos: IVec3,
    pub size: IVec3,
    
    pub chunk_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: usize,

    pub descriptor_sets: Vec<DescriptorSet>,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct RenderNode(pub bool);

impl ParallaxData {
    
    pub fn new(
        pos: IVec3,
        size: IVec3,
        num_nodes: usize,
        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<ParallaxData> {
        
        let chunk_buffer = Self::create_buffer(
            context,
            BufferUsageFlags::STORAGE_BUFFER,
            (num_nodes * size_of::<Vertex>()) as _,
        )?;
        let vertex_buffer = Self::create_buffer(
            context,
            BufferUsageFlags::VERTEX_BUFFER,
            (MIN_VERTICES * size_of::<Vertex>()) as _,
        )?;
        let index_buffer = Self::create_buffer(
            context,
            BufferUsageFlags::INDEX_BUFFER,
            (MIN_INDICES * size_of::<u16>()) as _,
        )?;
        let descriptor_sets = Self::create_descriptor_sets(
            &chunk_buffer,
            images_len,
            descriptor_layout,
            descriptor_pool,
        )?;

        Ok(ParallaxData {
            pos, 
            size,
            chunk_buffer,
            vertex_buffer,
            index_buffer,
            index_count: 0,

            descriptor_sets,
        })
    }
    
    pub fn update(
        &mut self,
        size: IVec3,
        node_id_bits: &[u32],
        render_nodes: &[RenderNode],
        context: &Context,
        to_drop_buffers: &mut Vec<Buffer>,
    ) -> Result<()> {
        self.chunk_buffer.copy_data_to_buffer(node_id_bits)?;

        let (vertecies, indecies) = Self::create_mesh(size, render_nodes)?;
        let vertex_size = (vertecies.len() * size_of::<Vertex>()) as DeviceSize;
        let index_size = (indecies.len() * size_of::<u16>()) as DeviceSize;

        if vertex_size > self.vertex_buffer.size {
            let mut buffer = Self::create_buffer(
                context,
                BufferUsageFlags::VERTEX_BUFFER,
                (vertecies.len() * size_of::<Vertex>()) as _,
            )?;

            mem::swap(&mut self.vertex_buffer, &mut buffer);
            to_drop_buffers.push(buffer);

            log::trace!("Chunk Vertex Buffer increased.");
        }
        self.vertex_buffer.copy_data_to_buffer(&vertecies)?;

        if index_size > self.index_buffer.size {
            let mut buffer = Self::create_buffer(
                context,
                BufferUsageFlags::INDEX_BUFFER,
                (indecies.len() * size_of::<u16>()) as _,
            )?;
            mem::swap(&mut self.index_buffer, &mut buffer);
            to_drop_buffers.push(buffer);

            log::trace!("Chunk Index Buffer increased.");
        }
        self.index_buffer.copy_data_to_buffer(&indecies)?;
        

        self.index_count = indecies.len();

        Ok(())
    }
    
    pub const RIGHT_HANDED_Z_UP_CONFIG: QuadCoordinateConfig = QuadCoordinateConfig {
        // Y is always in the V direction when it's not the normal. When Y is the
        // normal, right-handedness determines that we must use Yzx permutations.
        faces: [
            OrientedBlockFace::new(-1, AxisPermutation::Xzy),
            OrientedBlockFace::new(-1, AxisPermutation::Zxy),
            OrientedBlockFace::new(-1, AxisPermutation::Yzx),
            OrientedBlockFace::new(1, AxisPermutation::Xzy),
            OrientedBlockFace::new(1, AxisPermutation::Zxy),
            OrientedBlockFace::new(1, AxisPermutation::Yzx),
        ],
        u_flip_face: Axis::X,
    };

    fn run_greedy<const SIZE: u32>(render_nodes: &[RenderNode], buffer: &mut GreedyQuadsBuffer) {
        let shape: ConstShape3u32<SIZE, SIZE, SIZE> = ConstShape3u32 {};

        greedy_quads(
            render_nodes,
            &shape,
            [0; 3],
            [SIZE - 1; 3],
            &Self::RIGHT_HANDED_Z_UP_CONFIG.faces,
            buffer,
        );
    }

    fn create_mesh(
        size: IVec3,
        render_nodes: &[RenderNode],
    ) -> Result<(Vec<Vertex>, Vec<u16>)> {
        let mut buffer = GreedyQuadsBuffer::new(render_nodes.len());

        if size == (IVec3::ONE * 16) {
            Self::run_greedy::<18>(render_nodes, &mut buffer);
        } else if size == (IVec3::ONE * 32) {
            Self::run_greedy::<34>(render_nodes, &mut buffer);
        } else if size == (IVec3::ONE * 4) {
            Self::run_greedy::<6>(render_nodes, &mut buffer);
        } else if size == (IVec3::ONE * 8) {
            Self::run_greedy::<10>(render_nodes, &mut buffer);
        } else if size == (IVec3::ONE * 64) {
            Self::run_greedy::<66>(render_nodes, &mut buffer);
        } else {
            bail!("Chunk Size {size} not implemented!")
        }

        let num_quads = buffer.quads.num_quads();
        if num_quads == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let num_vertecies = num_quads * 4;
        let num_indecies = num_quads * 6;
        let mut vertecies = Vec::with_capacity(num_vertecies);
        let mut indecies: Vec<u16> = Vec::with_capacity(num_indecies);
        let mut index_counter = 0;
        buffer
            .quads
            .groups
            .iter()
            .zip(Self::RIGHT_HANDED_Z_UP_CONFIG.faces.iter())
            .for_each(|(group, of)| {
                group.iter().for_each(|uf| {
                    vertecies.extend(
                        of.quad_mesh_positions(uf, 1.0)
                            .into_iter()
                            .zip(iter::repeat(of.signed_normal()).take(4))
                            .map(|(p, n)| {
                                let pos = uvec3(
                                    p[0].round() as u32 - 1,
                                    p[1].round() as u32 - 1,
                                    p[2].round() as u32 - 1,
                                );
                                let normal = ivec3(n.x, n.y, n.z);
                                Vertex::new(pos, normal)
                            }),
                    );
                    indecies.extend(
                        of.quad_mesh_indices(index_counter)
                            .into_iter()
                            .map(|i| i as u16),
                    );
                    index_counter += 4;
                });
            });

        Ok((vertecies, indecies))
    }

    fn create_buffer(
        context: &Context,
        usage: BufferUsageFlags,
        size: DeviceSize,
    ) -> Result<Buffer> {
        let buffer = context.create_buffer(
            usage | BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
            size,
        )?;

        Ok(buffer)
    }
    
    fn create_descriptor_sets(
        chunk_buffer: &Buffer,
        images_len: usize,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Vec<DescriptorSet>> {
        let mut descriptor_sets = Vec::new();
        for _ in 0..images_len {
            let render_descriptor_set = descriptor_pool.allocate_set(descriptor_layout)?;

            render_descriptor_set.update(&[WriteDescriptorSet {
                binding: 0,
                kind: WriteDescriptorSetKind::StorageBuffer {
                    buffer: &chunk_buffer,
                },
            }]);
            descriptor_sets.push(render_descriptor_set);
        }

        Ok(descriptor_sets)
    }
}

impl Voxel for RenderNode {
    fn get_visibility(&self) -> VoxelVisibility {
        if self.0 {
            VoxelVisibility::Opaque
        } else {
            VoxelVisibility::Empty
        }
    }
}

impl MergeVoxel for RenderNode {
    type MergeValue = bool;
    fn merge_value(&self) -> Self::MergeValue {
        true
    }
}
