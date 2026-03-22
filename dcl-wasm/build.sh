#!/bin/bash
# Build DCL WASM module and prepare for web demo

echo "Building WASM module with wasm-pack..."
cd "$(dirname "$0")"
wasm-pack build --target web --out-dir www/pkg

if [ $? -eq 0 ]; then
    echo "✓ Build successful!"
    echo ""
    echo "To run the demo:"
    echo "  1. cd www"
    echo "  2. python -m http.server 8080"
    echo "  3. Open http://localhost:8080 in your browser"
else
    echo "✗ Build failed"
    exit 1
fi
