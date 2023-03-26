
use std::{fs::{File, self}, io::Write};

use app::{anyhow::Result, log};
use indicatif::ProgressBar;

use crate::{Tree, file::{metadata::{BatchMetadata, Metadata}, self}};

pub fn save_tree(tree: &mut impl Tree, folder_path: &str, batch_size: usize) -> Result<()> {

    log::info!("Saving Tree:");

    create_dir(folder_path)?;

    let mut nodes_in_file = 0;
    let mut file_counter = 0;
    let mut file: Option<File> = None;

    let mut batches: Vec<BatchMetadata> = Vec::new();

    let bar = ProgressBar::new(tree.get_size());

    for i in 0..tree.get_size() {
        bar.set_position(i);

        let node = &tree.get_node_by_index(i as usize)?;
        let node_id = node.get_node_id();
        let needed_file_counter = node_id / (batch_size as u64);

        // Create new file
        if  needed_file_counter != file_counter || file.is_none() {
            
            // Push Batch
            if needed_file_counter != 0{
                let file_start_id = file_counter * (batch_size as u64);
                let file_end_id = (file_counter + 1) * (batch_size as u64) - 1;

                batches.push(BatchMetadata::new(file_counter, file_start_id, file_end_id, nodes_in_file));
            } 

            file_counter = needed_file_counter;

            file = Some(File::create(format!("{folder_path}/{file_counter}.nodes"))?);
            nodes_in_file = 0;
        }

        // Write to file
        let data = unsafe { 
            any_as_u8_slice(node) 
        };
        file.as_ref().unwrap().write(data)?;
        nodes_in_file += 1;

    }

    // Push last Batch
    let file_start_id = file_counter * (batch_size as u64);
    let file_end_id = u64::min((file_counter + 1) * (batch_size as u64) - 1, tree.get_max_size());

    batches.push(BatchMetadata::new(file_counter, file_start_id, file_end_id, nodes_in_file));


    let metadata = Metadata::new(tree, batches, batch_size);
    metadata.save(folder_path)?;
    
    Ok(())
}

fn create_dir(folder_path: &str) -> Result<()> {
    let _ = fs::remove_dir_all(folder_path);
    fs::create_dir(folder_path)?;

    Ok(())
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
    )
}

