#!/bin/bash

find ./crates/examples/$1/shaders -name *.spv -type f -exec rm {} \;
find ./crates/examples/$1/shaders -not -name *.spv -type f -exec glslangValidator --target-env spirv1.4 -V -o {}.spv {} \;
