#!/bin/bash

sh ./scripts/build_release.sh "$1"
cd ./projects/$1
./target/release/"$1"
