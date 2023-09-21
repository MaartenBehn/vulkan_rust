use app::{anyhow::ensure, anyhow::Result, log, logger::log_init};
use octtree_v2::{
    converter::convert_template_to_tree, template::TemplateTreeReader, tree::TreeBuilder,
    util::create_dir,
};

use crate::template::build_template_tree;

mod template;

fn start() -> Result<()> {
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");
    log_init("octree_builder.log");

    let depth = 11;

    let template_save_path = "./assets/template_tree";
    let tmeplate_page_size = 1048576;
    build_template_tree(template_save_path, depth, tmeplate_page_size)?;

    let tree_save_path = "./assets/tree";
    let tree_page_size = 65536;
    let reader = TemplateTreeReader::new(template_save_path.to_owned())?;
    let builder = TreeBuilder::new(tree_save_path.to_owned(), tree_page_size)?;
    convert_template_to_tree(reader, builder)?;
    create_dir(&template_save_path)?;

    Ok(())
}

fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}
