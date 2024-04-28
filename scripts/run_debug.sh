#!/bin/bash

sh ./scripts/build_debug.sh "$1"
./crates/examples/$1/target/debug/"$1"