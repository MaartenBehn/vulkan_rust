use app::{log, anyhow::ensure, anyhow::Result, logger::log_init};
use octtree::Octtree;

const MAX_DEPTH: u16 = 20;

fn start() -> Result<()>{
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");

    log_init("octree_helper.log");


    log::info!("Max tree size:");

    let mut print_bigger = true;
    for i in 0..MAX_DEPTH{

        let size = Octtree::get_max_tree_size(i);

        if size > u32::MAX as u64 && print_bigger{
            log::info!("Bigger than u32");
            print_bigger = false;
        }

        log::info!("{i}: {}", size);
    }

    log::info!("Max node size:");

    print_bigger = true;
    for i in 0..MAX_DEPTH{
        let size = Octtree::get_node_size(i);

        if size > u32::MAX as u64 && print_bigger{
            log::info!("Bigger than u32");
            print_bigger = false;
        }

        log::info!("{i}: {}", size);
    }

    Ok(())
}
fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}
