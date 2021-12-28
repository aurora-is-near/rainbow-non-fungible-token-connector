#!/bin/bash
set -e
cd "`dirname $0`"
source ./flags.sh
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/near_metadata.wasm ../res/near_metadata.wasm
