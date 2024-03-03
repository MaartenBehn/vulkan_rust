#!/bin/bash

sh ./scripts/build_debug.sh $1

perf record --call-graph dwarf ./target/debug/$1

hotspot ./perf.data