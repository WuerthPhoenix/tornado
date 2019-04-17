#!/usr/bin/env bash

PACKAGE_NAME="dto"
OUT_DIR="pkg"
RUST_TARGET_PATH="../../target"

rm -rf $OUT_DIR/
WASM32=1 cargo build --target wasm32-unknown-unknown
mkdir $OUT_DIR
wasm-bindgen $RUST_TARGET_PATH/wasm32-unknown-unknown/debug/$PACKAGE_NAME.wasm --typescript --out-dir $OUT_DIR/
