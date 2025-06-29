#!/bin/bash

# Build script for RNotes

echo "Building RNotes with Git integration..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "📍 Binary location: target/release/rnotes"
    echo ""
    echo "🔄 Git Integration Features:"
    echo "  • Manual push/pull operations (no auto-commits)"
    echo "  • GitHub CLI authentication support"  
    echo "  • Press 'g' to push, 'p' to pull"
    echo "  • Hidden .git directory in file tree"
    echo ""
    echo "📋 Prerequisites for Git integration:"
    echo "  • Install GitHub CLI: sudo apt install gh"
    echo "  • Authenticate: gh auth login"
    echo ""
    echo "To install globally, run:"
    echo "  sudo cp target/release/rnotes /usr/local/bin/"
    echo ""
    echo "To run the application:"
    echo "  ./target/release/rnotes"
    echo "  or"
    echo "  cargo run"
    echo ""
    echo "📖 For Git setup instructions, see GIT_SETUP.md"
else
    echo "❌ Build failed!"
    exit 1
fi
