use app::{anyhow::ensure, anyhow::Result, log, logger::log_init};

use crate::template::build_template_tree;

mod template;

fn start() -> Result<()> {
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");
    log_init("octree_builder.log");

    let depth = 8;

    let template_save_path = "./assets/template_tree";
    let tmeplate_page_size = 65536;
    build_template_tree(template_save_path, depth, tmeplate_page_size)?;

    Ok(())
}

fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}
