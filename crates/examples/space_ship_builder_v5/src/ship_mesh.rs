use crate::ship::{Ship, ShipChunk, CHUNK_SIZE};
use octa_force::{
    anyhow::Result,
    log,
    vulkan::{Buffer, Context},
};
use std::mem::size_of;
use std::{iter, mem};

use crate::math::{to_1d, to_1d_i, to_3d};
use crate::node::{NodeID, EMPYT_PATTERN_INDEX};
use crate::rules::Rules;
use crate::ship_renderer::Vertex;
use crate::voxel_loader::VoxelLoader;
use block_mesh::ilattice::vector::Vector3;
use block_mesh::ndshape::{ConstShape, ConstShape3u32, Shape};
use block_mesh::{
    greedy_quads, Axis, AxisPermutation, GreedyQuadsBuffer, MergeVoxel, OrientedBlockFace,
    QuadCoordinateConfig, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG,
};
use dot_vox::Size;
use octa_force::anyhow::bail;
use octa_force::glam::{ivec3, uvec3, IVec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::{BufferUsageFlags, DeviceSize};
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    DescriptorPool, DescriptorSet, DescriptorSetLayout, WriteDescriptorSet, WriteDescriptorSetKind,
};

const NODE_SIZE_PLUS_PADDING: u32 = (CHUNK_SIZE + 2) as u32;

pub struct ShipMesh {
    pub chunks: Vec<MeshChunk>,
    pub to_drop_buffers: Vec<Vec<Buffer>>,
    pub size: IVec3,
}

pub struct MeshChunk {
    pub pos: IVec3,
    pub chunk_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: usize,

    pub descriptor_sets: Vec<DescriptorSet>,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct RenderNode(pub bool);

impl ShipMesh {
    pub fn new(images_len: usize, size: u32) -> Result<ShipMesh> {
        let mut to_drop_buffers = Vec::new();
        for _ in 0..images_len {
            to_drop_buffers.push(vec![])
        }

        Ok(ShipMesh {
            chunks: Vec::new(),
            to_drop_buffers,
            size: IVec3::ONE * size as i32,
        })
    }

    pub fn update(
        &mut self,
        ship: &Ship,
        changed_chunks: Vec<usize>,
        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        // Buffers from the last swapchain iteration are being dropped
        self.to_drop_buffers[image_index].clear();

        for chunk_index in changed_chunks.iter() {
            let chunk = &ship.chunks[*chunk_index];

            let mesh_chunk_index = self.chunks.iter().position(|c| c.pos == chunk.pos);
            if mesh_chunk_index.is_some() {
                self.chunks[mesh_chunk_index.unwrap()].update(
                    chunk,
                    context,
                    &mut self.to_drop_buffers[image_index],
                )?;
            } else {
                let new_chunk = MeshChunk::new(
                    chunk.pos,
                    chunk,
                    self.to_drop_buffers.len(),
                    context,
                    descriptor_layout,
                    descriptor_pool,
                )?;
                if new_chunk.is_some() {
                    self.chunks.push(new_chunk.unwrap())
                }
            }
        }

        Ok(())
    }

    pub fn update_from_mesh(
        &mut self,
        other_mesh: &ShipMesh,
        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        // Buffers from the last swapchain iteration are being dropped
        self.to_drop_buffers[image_index].clear();

        for (i, other_chunk) in other_mesh.chunks.iter().enumerate() {
            if self.chunks.len() <= (i) {
                let new_chunk = MeshChunk::new_from_chunk(
                    other_chunk,
                    self.to_drop_buffers.len(),
                    context,
                    descriptor_layout,
                    descriptor_pool,
                )?;
                self.chunks.push(new_chunk);
            } else {
                self.chunks[i].update_from_chunk(
                    other_chunk,
                    context,
                    &mut self.to_drop_buffers[image_index],
                )?;
            }
        }

        self.chunks.truncate(other_mesh.chunks.len());

        Ok(())
    }

    pub fn update_node_debug(
        &mut self,
        ship: &Ship,
        render_nodes: &Vec<RenderNode>,

        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        // Buffers from the last swapchain iteration are being dropped
        self.to_drop_buffers[image_index].clear();

        for chunk in ship.chunks.iter() {
            let mesh_chunk_index = self.chunks.iter().position(|c| c.pos == chunk.pos);

            if mesh_chunk_index.is_some() {
                self.chunks[mesh_chunk_index.unwrap()].update_node_debug(
                    ship,
                    chunk,
                    render_nodes,
                    context,
                    &mut self.to_drop_buffers[image_index],
                    self.size,
                )?;
            } else {
                let new_chunk = MeshChunk::new_wave_debug(
                    chunk.pos,
                    self.size,
                    ship,
                    chunk,
                    render_nodes,
                    self.to_drop_buffers.len(),
                    context,
                    descriptor_layout,
                    descriptor_pool,
                )?;
                if new_chunk.is_some() {
                    self.chunks.push(new_chunk.unwrap())
                }
            }
        }

        Ok(())
    }
}

impl MeshChunk {
    pub fn new(
        pos: IVec3,
        ship_chunk: &ShipChunk,
        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Option<MeshChunk>> {
        Self::new_from_data(
            pos,
            &ship_chunk.node_id_bits,
            &ship_chunk.node_voxels,
            images_len,
            context,
            descriptor_layout,
            descriptor_pool,
        )
    }

    fn new_from_data(
        pos: IVec3,
        node_id_bits: &[u32],
        render_nodes: &[RenderNode],
        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Option<MeshChunk>> {
        let (vertecies, indecies) = Self::create_mesh(render_nodes);
        let vertex_size = vertecies.len();
        let index_size = indecies.len();

        if vertex_size == 0 || index_size == 0 {
            return Ok(None);
        }

        let chunk_buffer = Self::create_buffer_from_data(
            context,
            node_id_bits,
            BufferUsageFlags::STORAGE_BUFFER,
            (node_id_bits.len() * size_of::<u32>()) as _,
        )?;
        let vertx_buffer = Self::create_buffer_from_data(
            context,
            &vertecies,
            BufferUsageFlags::VERTEX_BUFFER,
            (vertecies.len() * size_of::<Vertex>()) as _,
        )?;
        let index_buffer = Self::create_buffer_from_data(
            context,
            &indecies,
            BufferUsageFlags::INDEX_BUFFER,
            (indecies.len() * size_of::<u16>()) as _,
        )?;
        let descriptor_sets = Self::create_descriptor_sets(
            &chunk_buffer,
            images_len,
            descriptor_layout,
            descriptor_pool,
        )?;

        Ok(Some(MeshChunk {
            pos,
            chunk_buffer,

            vertex_buffer: vertx_buffer,
            index_buffer,
            index_count: index_size,

            descriptor_sets,
        }))
    }

    pub fn new_from_chunk(
        chunk: &MeshChunk,
        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Self> {
        let chunk_buffer = Self::create_buffer_from_buffer(
            context,
            &chunk.chunk_buffer,
            BufferUsageFlags::STORAGE_BUFFER,
        )?;
        let vertex_buffer = Self::create_buffer_from_buffer(
            context,
            &chunk.vertex_buffer,
            BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = Self::create_buffer_from_buffer(
            context,
            &chunk.index_buffer,
            BufferUsageFlags::INDEX_BUFFER,
        )?;

        context.execute_one_time_commands(|cmd_buffer| {
            cmd_buffer.copy_buffer(&chunk.chunk_buffer, &chunk_buffer);
            cmd_buffer.copy_buffer(&chunk.vertex_buffer, &vertex_buffer);
            cmd_buffer.copy_buffer(&chunk.index_buffer, &index_buffer);
        })?;

        let descriptor_sets = Self::create_descriptor_sets(
            &chunk_buffer,
            images_len,
            descriptor_layout,
            descriptor_pool,
        )?;

        Ok(MeshChunk {
            pos: chunk.pos,
            chunk_buffer,
            vertex_buffer,
            index_buffer,
            index_count: chunk.index_count,
            descriptor_sets,
        })
    }

    pub fn new_wave_debug(
        pos: IVec3,
        size: IVec3,

        ship: &Ship,
        ship_chunk: &ShipChunk,
        render_nodes: &Vec<RenderNode>,

        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Option<MeshChunk>> {
        let wave_debug_node_id_bits = Self::get_chunk_node_id_bits_debug(ship_chunk, size, ship);

        Self::new_from_data(
            pos,
            &wave_debug_node_id_bits,
            render_nodes,
            images_len,
            context,
            descriptor_layout,
            descriptor_pool,
        )
    }

    fn get_chunk_node_id_bits_debug(ship_chunk: &ShipChunk, size: IVec3, ship: &Ship) -> Vec<u32> {
        let mut node_debug_node_id_bits = vec![0; size.element_product() as usize];
        let pattern_block_size = size / ship.nodes_per_chunk;

        for x in 0..ship.nodes_per_chunk.x {
            for y in 0..ship.nodes_per_chunk.y {
                for z in 0..ship.nodes_per_chunk.z {
                    let node_pos = ivec3(x, y, z);
                    let node_index = ship.get_node_index(node_pos);
                    let r = ship_chunk.nodes[node_index].to_owned();
                    if r.is_none() {
                        continue;
                    }

                    let mut pattern_counter = 0;
                    let possible_pattern = r.unwrap();
                    let node_pos = node_pos * pattern_block_size;

                    'iter: for iz in 0..pattern_block_size.x {
                        for iy in 0..pattern_block_size.y {
                            for ix in 0..pattern_block_size.z {
                                if possible_pattern.len() <= pattern_counter {
                                    break 'iter;
                                } else if possible_pattern[pattern_counter].is_none() {
                                    pattern_counter += 1;

                                    if possible_pattern.len() <= pattern_counter {
                                        break 'iter;
                                    }
                                }

                                let pattern_pos = ivec3(ix, iy, iz) + node_pos;
                                let index =
                                    to_1d_i(pattern_pos, ship.nodes_per_chunk * pattern_block_size)
                                        as usize;

                                let node = possible_pattern[pattern_counter];
                                node_debug_node_id_bits[index] = node.into();
                                pattern_counter += 1;
                            }
                        }
                    }
                }
            }
        }

        node_debug_node_id_bits
    }

    pub fn update(
        &mut self,
        ship_chunk: &ShipChunk,
        context: &Context,
        to_drop_buffers: &mut Vec<Buffer>,
    ) -> Result<()> {
        self.update_from_data(
            &ship_chunk.node_id_bits,
            &ship_chunk.node_voxels,
            context,
            to_drop_buffers,
        )
    }

    pub fn update_from_data(
        &mut self,
        node_id_bits: &[u32],
        render_nodes: &[RenderNode],
        context: &Context,
        to_drop_buffers: &mut Vec<Buffer>,
    ) -> Result<()> {
        self.chunk_buffer.copy_data_to_buffer(node_id_bits)?;

        let (vertecies, indecies) = Self::create_mesh(render_nodes);
        let vertex_size = (vertecies.len() * size_of::<Vertex>()) as DeviceSize;
        let index_size = (indecies.len() * size_of::<u16>()) as DeviceSize;

        if vertex_size > self.vertex_buffer.size {
            let mut buffer = Self::create_buffer_from_data(
                context,
                &vertecies,
                BufferUsageFlags::VERTEX_BUFFER,
                (vertecies.len() * size_of::<Vertex>()) as _,
            )?;

            mem::swap(&mut self.vertex_buffer, &mut buffer);
            to_drop_buffers.push(buffer);

            log::trace!("Chunk Vertex Buffer increased.");
        } else {
            self.vertex_buffer.copy_data_to_buffer(&vertecies)?;
        }

        if index_size > self.index_buffer.size {
            let mut buffer = Self::create_buffer_from_data(
                context,
                &indecies,
                BufferUsageFlags::INDEX_BUFFER,
                (indecies.len() * size_of::<u16>()) as _,
            )?;
            mem::swap(&mut self.index_buffer, &mut buffer);
            to_drop_buffers.push(buffer);

            log::trace!("Chunk Index Buffer increased.");
        } else {
            self.index_buffer.copy_data_to_buffer(&indecies)?;
        }

        self.index_count = indecies.len();

        Ok(())
    }

    pub fn update_from_chunk(
        &mut self,
        chunk: &MeshChunk,
        context: &Context,
        to_drop_buffers: &mut Vec<Buffer>,
    ) -> Result<()> {
        self.pos = chunk.pos;

        if self.vertex_buffer.size < chunk.vertex_buffer.size {
            let mut buffer = Self::create_buffer_from_buffer(
                context,
                &chunk.vertex_buffer,
                BufferUsageFlags::VERTEX_BUFFER,
            )?;
            mem::swap(&mut self.vertex_buffer, &mut buffer);
            to_drop_buffers.push(buffer);
        }

        if self.index_buffer.size < chunk.index_buffer.size {
            let mut buffer = Self::create_buffer_from_buffer(
                context,
                &chunk.index_buffer,
                BufferUsageFlags::INDEX_BUFFER,
            )?;
            mem::swap(&mut self.index_buffer, &mut buffer);
            to_drop_buffers.push(buffer);
        }

        context.execute_one_time_commands(|cmd_buffer| {
            cmd_buffer.copy_buffer(&chunk.chunk_buffer, &self.chunk_buffer);
            cmd_buffer.copy_buffer(&chunk.vertex_buffer, &self.vertex_buffer);
            cmd_buffer.copy_buffer(&chunk.index_buffer, &self.index_buffer);
        })?;

        self.index_count = chunk.index_count;

        Ok(())
    }

    pub fn update_node_debug(
        &mut self,

        ship: &Ship,
        ship_chunk: &ShipChunk,
        render_nodes: &Vec<RenderNode>,

        context: &Context,
        to_drop_buffers: &mut Vec<Buffer>,
        size: IVec3,
    ) -> Result<()> {
        let wave_debug_node_id_bits = Self::get_chunk_node_id_bits_debug(ship_chunk, size, ship);

        self.update_from_data(
            &wave_debug_node_id_bits,
            render_nodes,
            context,
            to_drop_buffers,
        )
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

    fn create_mesh(render_nodes: &[RenderNode]) -> (Vec<Vertex>, Vec<u16>) {
        let mut buffer = GreedyQuadsBuffer::new(render_nodes.len());
        let shape: ConstShape3u32<
            NODE_SIZE_PLUS_PADDING,
            NODE_SIZE_PLUS_PADDING,
            NODE_SIZE_PLUS_PADDING,
        > = ConstShape3u32 {};

        greedy_quads(
            render_nodes,
            &shape,
            [0; 3],
            [NODE_SIZE_PLUS_PADDING - 1; 3],
            &Self::RIGHT_HANDED_Z_UP_CONFIG.faces,
            &mut buffer,
        );

        let num_quads = buffer.quads.num_quads();
        if num_quads == 0 {
            return (Vec::new(), Vec::new());
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

        (vertecies, indecies)
    }

    fn create_buffer_from_data<T: Copy>(
        context: &Context,
        data: &[T],
        usage: BufferUsageFlags,
        size: DeviceSize,
    ) -> Result<Buffer> {
        let buffer = context.create_buffer(
            usage | BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
            size,
        )?;
        buffer.copy_data_to_buffer(data)?;

        Ok(buffer)
    }

    fn create_buffer_from_buffer(
        context: &Context,
        other_buffer: &Buffer,
        usage: BufferUsageFlags,
    ) -> Result<Buffer> {
        let buffer = context.create_buffer(
            usage | BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuOnly,
            other_buffer.size,
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
