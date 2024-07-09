#!/bin/bash

sh ./scripts/run.sh octtree_builder_v2

sh ./scripts/build_release.sh voxel_renderer_v2
./projects/voxel_renderer_v2/target/release/voxel_renderer_v2