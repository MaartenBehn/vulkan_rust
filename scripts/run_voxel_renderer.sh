#!/bin/bash

sh ./scripts/run.sh octtree_builder

sh ./scripts/build_debug.sh voxel_renderer
./projects/voxel_renderer/target/debug/voxel_renderer