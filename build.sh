#!/bin/sh

set -e

cd volvelle-wasm
wasm-pack build --out-dir ../www/pkg --target no-modules
cd ..

