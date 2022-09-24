#!/bin/bash

set -e

pushd $(git rev-parse --show-toplevel)/volvelle-wasm
wasm-pack build --out-dir ../www/pkg --target no-modules
cargo test
popd

