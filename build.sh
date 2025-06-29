#!/bin/bash

# Build script for RNotes

echo "Building RNotes..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "📍 Binary location: target/release/rnotes"
    echo ""
    echo "To install globally, run:"
    echo "  sudo cp target/release/rnotes /usr/local/bin/"
    echo ""
    echo "To run the application:"
    echo "  ./target/release/rnotes"
    echo "  or"
    echo "  cargo run"
else
    echo "❌ Build failed!"
    exit 1
fi
