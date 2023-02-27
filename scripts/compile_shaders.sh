#!/bin/bash

find ./crates/*/shaders -not -name *.spv -type f -exec glslangValidator --target-env spirv1.4 -V -o {}.spv {} \;
