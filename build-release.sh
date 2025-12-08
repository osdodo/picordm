#!/bin/bash

# Build script for macOS release
set -e

VERSION="0.1.0"
APP_NAME="picordm"

# Detect architecture
ARCH=$(uname -m)
if [ "$ARCH" = "arm64" ]; then
    ARCH_NAME="aarch64"
elif [ "$ARCH" = "x86_64" ]; then
    ARCH_NAME="x86_64"
else
    ARCH_NAME="$ARCH"
fi

echo "🔨 Building release version for $ARCH_NAME..."
cargo build --release

echo "📦 Creating release package..."
cd target/release

# Create release directory
RELEASE_DIR="${APP_NAME}-v${VERSION}-macos-${ARCH_NAME}"
mkdir -p "$RELEASE_DIR"

# Copy binary file
cp "$APP_NAME" "$RELEASE_DIR/"

# Create tar.gz package
tar -czf "${RELEASE_DIR}.tar.gz" "$RELEASE_DIR"

# Calculate SHA256
echo ""
echo "✅ Build complete!"
echo "📍 Release file: target/release/${RELEASE_DIR}.tar.gz"
echo ""
echo "🔐 SHA256:"
shasum -a 256 "${RELEASE_DIR}.tar.gz"

# Clean up temporary directory
rm -rf "$RELEASE_DIR"

echo ""
echo "📤 Next steps:"
echo "1. Create a new release on GitHub: https://github.com/osdodo/picordm/releases/new"
echo "2. Tag version: v${VERSION}"
echo "3. Upload: target/release/${RELEASE_DIR}.tar.gz"
echo "4. Copy the SHA256 hash above for your Homebrew formula"
