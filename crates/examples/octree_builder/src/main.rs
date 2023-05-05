use app::{anyhow::ensure, anyhow::Result, log, logger::log_init};
use octtree::{
    self,
    basic_octtree::{BasicOcttree, InitalFill},
    Tree,
};

const SAVE_FOLDER: &str = "./assets/octtree";

fn start() -> Result<()> {
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");

    log_init("octree_builder.log");

    let depth = 10;
    let mut octtree = BasicOcttree::new(depth, 11261474734820965911, InitalFill::SpareseTree);

    octtree.save(SAVE_FOLDER, 1000000)?;

    Ok(())
}

fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}
