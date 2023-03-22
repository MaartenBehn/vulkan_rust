#!/bin/bash

sh ./scripts/compile_shaders_selected.sh $1
sh ./scripts/build_debug.sh $1 
./target/debug/$1
