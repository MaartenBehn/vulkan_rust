use crate::{
    file::{
        metadata::{BatchMetadata, Metadata},
        util,
    },
    octtree_node::OcttreeNode,
    Tree,
};

use std::{
    fs::{self, File},
    io::Write,
};

use indicatif::ProgressBar;
use app::{anyhow::Result, log};

pub fn save_tree(tree: &mut impl Tree, folder_path: &str, batch_size: usize) -> Result<()> {
    log::info!("Saving Tree:");

    let _ = create_dir(folder_path);

    let bar = ProgressBar::new(tree.get_size());

    let mut batches: Vec<BatchMetadata> = Vec::new();

    let mut file_counter = 0;
    let mut index = 0;
    let mut nodes_in_file = 0;

    loop {
        bar.set_position(index);

        // Getting nodes for File
        let mut nodes: Vec<OcttreeNode> = Vec::new();
        let (next_file_counter, done) = loop {
            let node = tree.get_node_by_index(index as usize)?;
            let node_id = node.get_node_id();
            let needed_file_counter = node_id / (batch_size as u64);

            if index >= tree.get_size() - 1 {
                break (0, true);
            }

            if needed_file_counter != file_counter {
                break (needed_file_counter, false);
            }

            nodes.push(node);
            index += 1;
            nodes_in_file += 1;
        };

        // Saving to file
        let mut file = File::create(format!("{folder_path}/{file_counter}.nodes"))?;
        let buffer = unsafe { util::vec_as_u8_slice(&nodes) };
        file.write_all(buffer)?;

        // Saving Metadata
        let file_start_id = file_counter * (batch_size as u64);
        let file_end_id = u64::min(
            (file_counter + 1) * (batch_size as u64) - 1,
            tree.get_max_size() - 1,
        );
        batches.push(BatchMetadata::new(
            file_counter,
            file_start_id,
            file_end_id,
            nodes_in_file,
        ));

        nodes_in_file = 0;

        if done {
            break;
        }

        file_counter = next_file_counter;
    }

    let metadata = Metadata::new(tree, batches, batch_size);
    metadata.save(folder_path)?;

    Ok(())
}

fn create_dir(folder_path: &str) -> Result<()> {
    let _ = fs::remove_dir_all(folder_path);
    fs::create_dir(folder_path)?;

    Ok(())
}
