#!/bin/bash

sh ./scripts/fmt.sh
sh ./scripts/compile_shaders_selected.sh $1

rm ./target/release/$1

cargo build --bin $1 -p $1 --release