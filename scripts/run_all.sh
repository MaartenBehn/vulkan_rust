#!/bin/bash

for d in ./projects/*; do
    echo $(basename $d)
    sh ./scripts/run.sh $(basename $d)
done
