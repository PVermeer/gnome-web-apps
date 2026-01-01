#!/bin/bash

set -e

echo -e "\n==== Running release package ====\n"

cargo run --bin=release --locked

echo -e "\n==== Done ====\n"
