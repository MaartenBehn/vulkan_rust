use octtree::{
    self,
    basic_octtree::{BasicOcttree, InitalFill},
    Tree,
};

const SAVE_FOLDER: &str = "./libs/octtree/assets/octtree";

fn main() {
    let depth = 8;
    let mut octtree = BasicOcttree::new(depth, 11261474734820965911, InitalFill::SpareseTree);

    let _ = octtree.save(SAVE_FOLDER, 1000000);
}
