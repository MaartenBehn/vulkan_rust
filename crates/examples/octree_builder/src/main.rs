use app::{log, anyhow::ensure, anyhow::Result, logger::log_init};

fn start() -> Result<()>{
    ensure!(cfg!(target_pointer_width = "64"), "Target not 64 bit");

    log_init("octree_builder.log");

    
    
    Ok(())
}
fn main() {
    let result = start();
    if result.is_err() {
        log::error!("{}", result.unwrap_err());
    }
}
