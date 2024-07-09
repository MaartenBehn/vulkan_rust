#!/bin/bash

sh ./scripts/build_debug.sh "$1"
cd ./projects/$1
./target/debug/"$1"