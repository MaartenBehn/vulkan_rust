#!/bin/bash

sh ./scripts/compile_shaders_selected.sh $1

rm ./target/release/$1

export RUST_LOG=INFO

cargo build --bin $1 -p $1 --release

export RUST_LOG=