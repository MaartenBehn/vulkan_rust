use crate::ship::{Ship, ShipChunk, Wave, CHUNK_WAVE_LEN};
use octa_force::{
    anyhow::Result,
    log,
    vulkan::{Buffer, Context},
};
use std::iter;
use std::mem::size_of;

use crate::math::{to_1d, to_3d};
use crate::node::{NodeID, EMPYT_PATTERN_INDEX};
use crate::ship_renderer::Vertex;
use block_mesh::ilattice::vector::Vector3;
use block_mesh::ndshape::{ConstShape, ConstShape3u32, Shape};
use block_mesh::{
    greedy_quads, Axis, AxisPermutation, GreedyQuadsBuffer, MergeVoxel, OrientedBlockFace,
    QuadCoordinateConfig, Voxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG,
};
use octa_force::anyhow::bail;
use octa_force::glam::{ivec3, uvec3, IVec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::ash::vk::BufferUsageFlags;
use octa_force::vulkan::gpu_allocator::MemoryLocation;
use octa_force::vulkan::{
    DescriptorPool, DescriptorSet, DescriptorSetLayout, WriteDescriptorSet, WriteDescriptorSetKind,
};

const CHUNK_SIZE: u32 = 16;
type ChunkShape = ConstShape3u32<{ CHUNK_SIZE + 2 }, { CHUNK_SIZE + 2 }, { CHUNK_SIZE + 2 }>;

pub struct ShipMesh {
    pub chunks: Vec<MeshChunk>,
}

pub struct MeshChunk {
    pub pos: IVec3,
    pub chunk_buffer: Buffer,

    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub vertex_buffer_size: usize,
    pub index_buffer_size: usize,
    pub index_count: usize,

    pub descriptor_sets: Vec<DescriptorSet>,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct RenderNode(pub bool);

impl ShipMesh {
    pub fn new() -> Result<ShipMesh> {
        Ok(ShipMesh { chunks: Vec::new() })
    }

    pub fn update(
        &mut self,
        ship: &Ship,
        changed_chunks: Vec<usize>,

        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        for chunk_index in changed_chunks.iter() {
            let chunk = &ship.chunks[*chunk_index];
            let mesh_chunk_index = self.chunks.iter().position(|c| c.pos == chunk.pos);
            if mesh_chunk_index.is_some() {
                self.chunks[mesh_chunk_index.unwrap()].update(chunk, context)?;
            } else {
                let new_chunk = MeshChunk::new(
                    chunk.pos,
                    chunk,
                    images_len,
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
        let (vertecies, indecies) = Self::create_mesh(ship_chunk);
        let vertex_size = vertecies.len();
        let index_size = indecies.len();

        if vertex_size == 0 || index_size == 0 {
            return Ok(None);
        }

        let chunk_buffer = Self::create_chunk_buffer(context, &ship_chunk.node_id_bits)?;
        let vertx_buffer = Self::create_vertex_buffer(context, vertecies)?;
        let index_buffer = Self::create_index_buffer(context, indecies)?;

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

        Ok(Some(MeshChunk {
            pos,
            chunk_buffer,

            vertex_buffer: vertx_buffer,
            index_buffer,
            vertex_buffer_size: vertex_size,
            index_buffer_size: index_size,
            index_count: index_size,

            descriptor_sets,
        }))
    }

    pub fn update(&mut self, ship_chunk: &ShipChunk, context: &Context) -> Result<()> {
        self.chunk_buffer
            .copy_data_to_buffer(&ship_chunk.node_id_bits)?;

        let (vertecies, indecies) = Self::create_mesh(ship_chunk);
        let vertex_size = vertecies.len();
        let index_size = indecies.len();

        if vertex_size > self.vertex_buffer_size {
            self.vertex_buffer = Self::create_vertex_buffer(context, vertecies)?;
            self.vertex_buffer_size = vertex_size;
            log::trace!("Chunk Vertex Buffer increased.");
        } else {
            self.vertex_buffer.copy_data_to_buffer(&vertecies)?;
        }

        if index_size > self.index_buffer_size {
            self.index_buffer = Self::create_index_buffer(context, indecies)?;
            self.index_buffer_size = index_size;
            log::trace!("Chunk Index Buffer increased.");
        } else {
            self.index_buffer.copy_data_to_buffer(&indecies)?;
        }

        self.index_count = index_size;

        Ok(())
    }

    fn create_chunk_buffer(context: &Context, node_bits: &[u32; CHUNK_WAVE_LEN]) -> Result<Buffer> {
        let chunk_buffer = context.create_buffer(
            BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
            (node_bits.len() * size_of::<u32>()) as _,
        )?;
        chunk_buffer.copy_data_to_buffer(node_bits)?;

        Ok(chunk_buffer)
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

    fn create_mesh(chunk: &ShipChunk) -> (Vec<Vertex>, Vec<u16>) {
        let mut buffer = GreedyQuadsBuffer::new(chunk.node_voxels.len());
        greedy_quads(
            &chunk.node_voxels,
            &ChunkShape {},
            [0; 3],
            [CHUNK_SIZE + 1; 3],
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

    fn create_vertex_buffer(context: &Context, vertecies: Vec<Vertex>) -> Result<Buffer> {
        let vertex_buffer = context.create_buffer(
            BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (vertecies.len() * size_of::<Vertex>()) as _,
        )?;
        vertex_buffer.copy_data_to_buffer(&vertecies)?;

        Ok(vertex_buffer)
    }

    fn create_index_buffer(context: &Context, indecies: Vec<u16>) -> Result<Buffer> {
        let index_buffer = context.create_buffer(
            BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
            (indecies.len() * size_of::<u16>()) as _,
        )?;
        index_buffer.copy_data_to_buffer(&indecies)?;

        Ok(index_buffer)
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
