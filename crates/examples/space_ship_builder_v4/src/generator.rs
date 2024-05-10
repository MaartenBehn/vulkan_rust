use crate::math::{get_neighbors, to_1d_i};
use crate::node::NodeID;
use crate::ship::Ship;
use crate::voxel_loader::VoxelLoader;
use neuroflow::activators::Type::Tanh;
use neuroflow::data::DataSet;
use neuroflow::FeedForward;
use octa_force::glam::{ivec3, IVec3, UVec3};
use octa_force::log::debug;

pub struct Generator {
    nn: FeedForward,
    node_id_table: Vec<NodeID>,
    input_size: usize,
    output_size: usize,
}

impl Generator {
    pub fn new(voxel_loader: &VoxelLoader) -> Self {
        let mut node_id_table = Vec::new();

        let input_size = (voxel_loader.block_names.len() - 1) * 27 + 3;

        let mut input_datasets = Vec::new();
        let mut output_indecies = Vec::new();
        for (node_pos, id) in voxel_loader.node_positions.iter() {
            // Input
            let mut input_data = vec![0.0; input_size];
            let block_pos = (*node_pos / 8) * 8;

            for (neigbor_index, neighbor_offset) in get_neighbors().iter().enumerate() {
                let pos = block_pos.as_ivec3() + *neighbor_offset * 8;

                let block_index = if pos.is_negative_bitmask() != 0 {
                    None
                } else {
                    voxel_loader.block_positions.get(&pos.as_uvec3())
                };

                if !block_index.is_none() {
                    input_data[(*block_index.unwrap() - 1) * 27 + neigbor_index] = 1.0;
                }
            }

            let node_pos_in_block = (*node_pos % 8) / 4;
            input_data[input_size - 3] = node_pos_in_block.x as f64;
            input_data[input_size - 2] = node_pos_in_block.y as f64;
            input_data[input_size - 1] = node_pos_in_block.z as f64;

            input_datasets.push(input_data);

            // Output
            let r = node_id_table.iter().position(|test_id| *id == *test_id);
            let table_index = if r.is_some() {
                r.unwrap()
            } else {
                node_id_table.push(id.to_owned());
                node_id_table.len() - 1
            };

            output_indecies.push(table_index);
        }

        let output_size = node_id_table.len();

        let mut data: DataSet = DataSet::new();
        let mut out_dataset = Vec::new();
        for (in_data, out_index) in input_datasets.iter().zip(output_indecies.iter()) {
            let mut out_data = vec![0.0; output_size];
            out_data[*out_index] = 1.0;

            data.push(in_data, &out_data);

            out_dataset.push(out_data);
        }

        let mut nn = FeedForward::new(&[
            input_size as i32,
            input_size as i32,
            input_size as i32,
            output_size as i32,
            output_size as i32,
        ]);

        // Here, we set the necessary parameters and train the neural network by our DataSet with 50 000 iterations
        nn.activation(Tanh).learning_rate(0.01).train(&data, 1000);

        /*
        for (in_data, out_data) in input_datasets.iter().zip(out_dataset.iter()) {
            let res = nn.calc(in_data);

            debug!("Exp: {:?}", out_data);
            debug!("Got: {:?}", out_data);
        }
         */

        Generator {
            nn,
            node_id_table,
            input_size,
            output_size,
        }
    }

    pub fn generate_node(&mut self, ship: &Ship, node_pos: IVec3) -> NodeID {
        let mut input_data = vec![0.0; self.input_size];
        let block_pos = Ship::get_block_pos_of_node_pos(node_pos);

        for (neigbor_index, neighbor_offset) in get_neighbors().iter().enumerate() {
            let pos = block_pos + *neighbor_offset;

            let chunk_pos = ship.get_chunk_pos_of_node_pos(pos);
            let chunk_index = ship.get_chunk_index(chunk_pos);
            if chunk_index.is_err() {
                continue;
            }
            let in_chunk_block_pos = ship.get_in_chunk_pos_of_block_pos(pos);
            let in_chunk_block_index =
                to_1d_i(in_chunk_block_pos, IVec3::ONE * ship.block_size) as usize;
            let block_index = ship.chunks[chunk_index.unwrap()].blocks[in_chunk_block_index];

            if block_index != 0 {
                input_data[(block_index - 1) * 27 + neigbor_index] = 1.0;
            }
        }

        let node_pos_in_block = node_pos % 2;
        input_data[self.input_size - 3] = node_pos_in_block.x as f64;
        input_data[self.input_size - 2] = node_pos_in_block.y as f64;
        input_data[self.input_size - 1] = node_pos_in_block.z as f64;

        // debug!("Imput: {:?}", input_data);

        let res = self.nn.calc(&input_data);

        let mut max_index = 0;
        let mut max_out = 0.0;
        for (i, out) in res.into_iter().enumerate() {
            if max_out < *out {
                max_out = *out;
                max_index = i;
            }
        }

        let node_id = self.node_id_table[max_index];
        debug!("Generated: {:?} with {:.2} percent.", node_id, max_out);

        node_id
    }
}
