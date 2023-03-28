#!/bin/bash

sh ./scripts/run.sh octtree_builder

sh ./scripts/build_release.sh voxel_renderer 
./target/release/voxel_renderer