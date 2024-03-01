use octa_force::{anyhow::ensure, anyhow::Result, log, logger::log_init};
use octtree_v2::{
    builder::Builder, converter::convert_template_to_tree, reader::Reader, util::create_dir,
};

use crate::template::build_template_tree;

mod template;

const BUILD_TEMPLATE: bool = true;
const DELETE_TEMPLATE: bool = true;

fn start() -> Result<()> {
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");
    log_init("octree_builder.log");

    let depth = 12;

    let template_save_path = "./assets/template_tree";
    let tmeplate_page_size = 1048576;
    if BUILD_TEMPLATE {
        build_template_tree(template_save_path, depth, tmeplate_page_size)?;
    }

    let tree_save_path = "./assets/tree";
    let tree_page_size = 1048576;
    let reader = Reader::new(template_save_path.to_owned(), 100)?;
    let builder = Builder::new(tree_save_path.to_owned(), tree_page_size, depth)?;
    convert_template_to_tree(reader, builder)?;

    if DELETE_TEMPLATE {
        create_dir(&template_save_path)?;
    }

    Ok(())
}

fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}
