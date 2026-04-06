#!/bin/sh
set -e

export PACKAGE_NAME="bevy-car"
export RUSTFLAGS='--cfg getrandom_backend="wasm_js" -C opt-level=z'

echo "Building WASM target..."
cargo build \
  --release \
  --target wasm32-unknown-unknown

echo "Running wasm-bindgen..."
wasm-bindgen \
  --no-typescript \
  --target web \
  --out-dir ./dist/ \
  --out-name "$PACKAGE_NAME" \
  ./target/wasm32-unknown-unknown/release/$PACKAGE_NAME.wasm

echo "Creating index.html..."
cat > ./dist/index.html << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Bevy Car - Parking Game</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            width: 100vw;
            height: 100vh;
            overflow: hidden;
            background: #1a1a1a;
            display: flex;
            justify-content: center;
            align-items: center;
        }
        #bevy-canvas {
            width: 100%;
            height: 100%;
            display: block;
        }
    </style>
</head>
<body>
    <canvas id="bevy-canvas"></canvas>
    <script type="module">
        import init from './bevy-car.js';
        init();
    </script>
</body>
</html>
EOF

echo "Build complete! Output in ./dist/"
