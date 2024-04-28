#!/bin/bash

sh ./scripts/build_release.sh "$1"
./crates/examples/$1/target/release/"$1"
