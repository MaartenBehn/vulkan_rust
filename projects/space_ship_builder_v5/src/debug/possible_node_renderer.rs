use crate::math::to_1d_i;
use crate::ship::{Ship, ShipChunk};
use crate::ship_mesh::{MeshChunk, RenderNode, ShipMesh};
use crate::ship_renderer::{ShipRenderer, RENDER_MODE_BUILD};
use octa_force::anyhow::Result;
use octa_force::glam::{ivec3, vec4, IVec3, Vec3};
use octa_force::vulkan::ash::vk;
use octa_force::vulkan::{Buffer, CommandBuffer, Context, DescriptorPool, DescriptorSetLayout};

pub struct DebugPossibleNodeRenderer {
    mesh: ShipMesh,
}

impl DebugPossibleNodeRenderer {
    pub fn new(image_len: usize, ship: &Ship) -> Result<Self> {
        Ok(DebugPossibleNodeRenderer {
            mesh: ShipMesh::new(image_len, IVec3::ONE * 128, ship.nodes_per_chunk)?,
        })
    }

    pub fn update(
        &mut self,
        ship: &Ship,
        image_index: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<()> {
        self.mesh.to_drop_buffers[image_index].clear();

        for chunk in ship.chunks.iter() {
            let mesh_chunk_index = self.mesh.chunks.iter().position(|c| c.pos == chunk.pos);

            if mesh_chunk_index.is_some() {
                Self::update_possible_node_debug(
                    &mut self.mesh.chunks[mesh_chunk_index.unwrap()],
                    self.mesh.size,
                    ship,
                    chunk,
                    context,
                    &mut self.mesh.to_drop_buffers[image_index],
                )?;
            } else {
                let new_chunk = Self::new_possible_node_debug(
                    chunk.pos,
                    self.mesh.size,
                    self.mesh.render_size,
                    ship,
                    chunk,
                    self.mesh.to_drop_buffers.len(),
                    context,
                    descriptor_layout,
                    descriptor_pool,
                )?;
                if new_chunk.is_some() {
                    self.mesh.chunks.push(new_chunk.unwrap())
                }
            }
        }

        Ok(())
    }

    fn new_possible_node_debug(
        pos: IVec3,
        size: IVec3,
        render_size: IVec3,

        ship: &Ship,
        ship_chunk: &ShipChunk,

        images_len: usize,
        context: &Context,
        descriptor_layout: &DescriptorSetLayout,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Option<MeshChunk>> {
        let wave_debug_node_id_bits = Self::get_chunk_node_id_bits_debug(ship_chunk, size, ship);

        MeshChunk::new_from_data(
            pos,
            size,
            render_size,
            &wave_debug_node_id_bits,
            &ship_chunk.render_nodes,
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
                                }

                                let pattern_pos = ivec3(ix, iy, iz) + node_pos;
                                let index = to_1d_i(pattern_pos, size) as usize;

                                let (node_id, _) = possible_pattern[pattern_counter];
                                node_debug_node_id_bits[index] = node_id.into();

                                if node_id.is_none() {
                                    let one_cell_size = Vec3::ONE / pattern_block_size.as_vec3();
                                    let p = pattern_pos.as_vec3() * one_cell_size;
                                    self.add_cube(p, p + one_cell_size, vec4(0.0, 1.0, 0.0, 1.0));
                                }

                                pattern_counter += 1;
                            }
                        }
                    }
                }
            }
        }

        node_debug_node_id_bits
    }

    fn update_possible_node_debug(
        chunk: &mut MeshChunk,

        size: IVec3,

        ship: &Ship,
        ship_chunk: &ShipChunk,

        context: &Context,
        to_drop_buffers: &mut Vec<Buffer>,
    ) -> Result<()> {
        let wave_debug_node_id_bits = Self::get_chunk_node_id_bits_debug(ship_chunk, size, ship);

        chunk.update_from_data(
            &wave_debug_node_id_bits,
            &ship_chunk.render_nodes,
            context,
            to_drop_buffers,
        )
    }

    pub fn render(&mut self, buffer: &CommandBuffer, renderer: &ShipRenderer, image_index: usize) {
        buffer.bind_graphics_pipeline(&renderer.pipeline);
        buffer.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &renderer.pipeline_layout,
            0,
            &[&renderer.static_descriptor_sets[image_index]],
        );

        renderer.render_ship_mesh(buffer, image_index, &self.mesh, RENDER_MODE_BUILD)
    }
}
