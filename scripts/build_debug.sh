#!/bin/bash

sh ./scripts/fmt.sh $1
sh ./scripts/compile_shaders_selected.sh $1

cd ./crates/examples/"$1"/ || exit
rm ./target/debug/"$1"
cargo build