#!/bin/bash

for d in ./crates/examples/*; do
    echo $(basename $d)
    sh ./scripts/run.sh $(basename $d)
done
