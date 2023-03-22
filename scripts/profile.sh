#!/bin/bash

sh ./scripts/build_release.sh $1 

perf record --call-graph dwarf ./target/release/$1

hotspot ./perf.data