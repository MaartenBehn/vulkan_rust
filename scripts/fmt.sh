#!/bin/bash

find ./projects/$1/* -name *.rs -type f -exec rustfmt {} \;
