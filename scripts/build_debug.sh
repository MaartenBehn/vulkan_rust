#!/bin/bash

sh ./scripts/compile_shaders_selected.sh $1

rm ./target/debug/$1

export VK_LAYER_PATH=$VULKAN_SDK/Bin
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation

cargo build --bin $1 -p $1

export VK_LAYER_PATH=
export VK_INSTANCE_LAYERS=
