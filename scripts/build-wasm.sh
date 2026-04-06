#!/bin/sh
set -e

echo "Copying static files..."
cp -r public dist

export PACKAGE_NAME=cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name'

echo "Building WASM target..."
cargo build \
  --release \
  --target wasm32-unknown-unknown \
  --no-default-features \
  --features web

echo "Running wasm-bindgen..."
wasm-bindgen \
  --no-typescript \
  --target web \
  --out-dir ./dist/ \
  --out-name "$PACKAGE_NAME" \
  ./target/wasm32-unknown-unknown/release/$PACKAGE_NAME.wasm

echo "Build complete! Output in ./dist/"
