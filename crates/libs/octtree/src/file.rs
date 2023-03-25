use std::{fs::{File, rename, remove_dir_all, create_dir}, io::Write};

use crate::Octtree;
use app::anyhow::{Result, Ok};
use indicatif::ProgressBar;

const CURRENT_FILE_NAME: &str = "current";

impl Octtree {

    pub fn save_to_file(&self, folder_path: &str, nodes_per_file: u64) -> Result<()> {

        let mut node_in_file = 0;

        let mut file_start_id = 0;
        let mut file: Option<File> = None;

        let bar = ProgressBar::new(self.size);

        let _ = remove_dir_all(folder_path);
        create_dir(folder_path)?;

        for i in 0..self.size {

            bar.set_position(i);

            let node = &self.nodes[i as usize];

            if node_in_file == 0 {
                file_start_id = node.get_node_id();
                
                file = Some(File::create(format!("{folder_path}/{CURRENT_FILE_NAME}.nodes"))?);
            }
            
            let data = unsafe { 
                any_as_u8_slice(node) 
            };

            file.as_ref().unwrap().write(data)?;


            node_in_file += 1;
            if node_in_file >= nodes_per_file || i >= self.size - 1{
                node_in_file = 0;

                if file.is_some() {
                    file = None;

                    let file_end_id = self.nodes[(i - 1) as usize].get_node_id();
                    let final_file_name = format!("{folder_path}/{file_start_id}_{file_end_id}.nodes");

                    rename(format!("{folder_path}/{CURRENT_FILE_NAME}.nodes"), final_file_name)?;
                }   
            }
        }

        Ok(())
    }
}

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::core::mem::size_of::<T>(),
    )
}