use app::{anyhow::ensure, anyhow::Result, log, logger::log_init};

use crate::octtree::Octtree;

mod octtree;

fn start() -> Result<()> {
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");

    log_init("octree_builder.log");

    let depth = 10;
    let save_path = "./assets/octtree";
    Octtree::build(save_path, depth)?;

    Ok(())
}

fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}
