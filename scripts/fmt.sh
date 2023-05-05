#!/bin/bash

find ./crates/examples/$1/* -name *.rs -type f -exec rustfmt {} \;
