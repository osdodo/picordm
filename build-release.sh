#!/bin/bash

set -e

VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
APP_NAME="picordm"

OS=$(uname -s)
case "$OS" in
    Darwin)
        OS_NAME="macos"
        HASH_CMD="shasum -a 256"
        ;;
    Linux)
        OS_NAME="linux"
        HASH_CMD="sha256sum"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        OS_NAME="windows"
        HASH_CMD="sha256sum"
        ;;
    *)
        OS_NAME=$(echo "$OS" | tr '[:upper:]' '[:lower:]')
        HASH_CMD="sha256sum"
        ;;
esac

if command -v rustc &> /dev/null; then
    RUST_TARGET=$(rustc -vV | grep "host:" | cut -d' ' -f2)
    case "$RUST_TARGET" in
        *aarch64*|*arm64*)
            ARCH_NAME="arm64"
            ;;
        *x86_64*)
            ARCH_NAME="x86_64"
            ;;
        *)
            ARCH_NAME=$(echo "$RUST_TARGET" | cut -d'-' -f1)
            ;;
    esac
else
    ARCH=$(uname -m)
    if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
        ARCH_NAME="arm64"
    elif [ "$ARCH" = "x86_64" ]; then
        ARCH_NAME="x86_64"
    else
        ARCH_NAME="$ARCH"
    fi
fi

echo "Building release version for $OS_NAME-$ARCH_NAME..."
cargo build --release

echo "Creating release package..."
cd target/release

if [ "$OS_NAME" = "windows" ]; then
    BINARY_NAME="${APP_NAME}.exe"
else
    BINARY_NAME="$APP_NAME"
fi

RELEASE_DIR="${APP_NAME}-v${VERSION}-${OS_NAME}-${ARCH_NAME}"
mkdir -p "$RELEASE_DIR"

cp "$BINARY_NAME" "$RELEASE_DIR/"

if [ "$OS_NAME" = "windows" ]; then
    if command -v zip &> /dev/null; then
        zip -r "${RELEASE_DIR}.zip" "$RELEASE_DIR"
        ARCHIVE_FILE="${RELEASE_DIR}.zip"
    else
        tar -czf "${RELEASE_DIR}.tar.gz" "$RELEASE_DIR"
        ARCHIVE_FILE="${RELEASE_DIR}.tar.gz"
    fi
else
    tar -czf "${RELEASE_DIR}.tar.gz" "$RELEASE_DIR"
    ARCHIVE_FILE="${RELEASE_DIR}.tar.gz"
fi

echo ""
echo "Build complete!"
echo "Release file: target/release/${ARCHIVE_FILE}"
echo ""
echo "SHA256:"
$HASH_CMD "${ARCHIVE_FILE}"

rm -rf "$RELEASE_DIR"
