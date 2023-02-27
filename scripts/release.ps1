$env:RUST_LOG="INFO"

cargo build --bin $args[0] -p $args[0] --release

$env:RUST_LOG=""