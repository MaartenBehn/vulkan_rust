#!/bin/bash

sh ./scripts/fmt.sh $1
sh ./scripts/compile_shaders_selected.sh $1

rm target/debug/$1
cargo build --bin $1 -p $1
