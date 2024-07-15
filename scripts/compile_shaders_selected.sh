#!/bin/bash

find ./projects/$1/shaders -name *.spv -type f -exec rm {} \;
find ./projects/$1/shaders -not -name *.spv -type f -exec glslangValidator -l --target-env spirv1.4 -V -o {}.spv {} \;
