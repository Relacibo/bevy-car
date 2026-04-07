#!/bin/sh
set -e

# Erstelle dist-Ordner, falls nicht vorhanden
mkdir -p dist

echo "Copying static files..."
cp -r public/* dist/ 2>/dev/null || true

export PACKAGE_NAME=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name')

JOBS_FLAG=""
if [ -n "$CARGO_JOBS" ]; then
    JOBS_FLAG="-j $CARGO_JOBS"
    echo "Using $CARGO_JOBS parallel jobs for build..."
fi

FEATURES_FLAG=""
if [ -n "$CARGO_FEATURES" ]; then
    FEATURES_FLAG="--features=$CARGO_FEATURES"
    echo "Features: $FEATURES_FLAG"
fi

echo "Building WASM target for $PACKAGE_NAME..."
cargo build \
  $JOBS_FLAG \
  $FEATURES_FLAG \
  --release \
  --target wasm32-unknown-unknown \
  --no-default-features

echo "Running wasm-bindgen..."
wasm-bindgen \
  --no-typescript \
  --target web \
  --out-dir ./dist/ \
  --out-name "$PACKAGE_NAME" \
  "./target/wasm32-unknown-unknown/release/$PACKAGE_NAME.wasm"

echo "Build complete! Output in ./dist/"
