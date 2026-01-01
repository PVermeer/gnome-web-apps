#!/bin/bash

set -e

echo -e "\n==== Building release package ====\n"

cargo build --bin=release --locked

echo -e "\n==== Done ====\n"
