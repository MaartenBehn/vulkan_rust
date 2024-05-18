#!/bin/bash

sh ./scripts/fmt.sh $1
sh ./scripts/compile_shaders_selected.sh $1

cd ./projects/"$1"/ || exit
rm ./target/debug/"$1"
cargo build