#!/bin/bash
# Build musl libc from source for RISC-V with debug symbols
# This allows debugging LARVa with a known libc build

set -e

MUSL_VERSION="1.2.5"
MUSL_DIR="$(pwd)/musl-${MUSL_VERSION}"
INSTALL_DIR="$(pwd)/musl-riscv64"
JOBS=$(nproc)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Building musl ${MUSL_VERSION} for RISC-V ===${NC}"

# Check for cross compiler
if ! command -v riscv64-linux-gnu-gcc &> /dev/null && ! command -v riscv64-unknown-linux-gnu-gcc &> /dev/null; then
    echo -e "${RED}Error: No RISC-V cross compiler found${NC}"
    echo "Please install one of:"
    echo "  - riscv64-linux-gnu-gcc"
    echo "  - riscv64-unknown-linux-gnu-gcc"
    exit 1
fi

# Determine cross compiler prefix
if command -v riscv64-linux-gnu-gcc &> /dev/null; then
    CROSS_PREFIX="riscv64-linux-gnu-"
else
    CROSS_PREFIX="riscv64-unknown-linux-gnu-"
fi

echo "Using cross compiler: ${CROSS_PREFIX}gcc"

# Download musl if not present
if [ ! -d "$MUSL_DIR" ]; then
    echo -e "${YELLOW}Downloading musl ${MUSL_VERSION}...${NC}"
    wget -q "https://musl.libc.org/releases/musl-${MUSL_VERSION}.tar.gz"
    tar -xzf "musl-${MUSL_VERSION}.tar.gz"
    rm "musl-${MUSL_VERSION}.tar.gz"
fi

cd "$MUSL_DIR"

# Clean previous build
if [ -d "$INSTALL_DIR" ]; then
    echo -e "${YELLOW}Removing previous install...${NC}"
    rm -rf "$INSTALL_DIR"
fi

# Configure with debug symbols
echo -e "${YELLOW}Configuring musl...${NC}"
./configure \
    --prefix="$INSTALL_DIR" \
    --target=riscv64 \
    --disable-shared \
    --enable-debug \
    --enable-optimize=no \
    CFLAGS="-g -O0 -fno-omit-frame-pointer" \
    CC="${CROSS_PREFIX}gcc" \
    AR="${CROSS_PREFIX}ar" \
    RANLIB="${CROSS_PREFIX}ranlib"

# Build
echo -e "${YELLOW}Building musl (this may take a while)...${NC}"
make -j"$JOBS"

# Install
echo -e "${YELLOW}Installing to ${INSTALL_DIR}...${NC}"
make install

echo -e "${GREEN}=== Build complete! ===${NC}"
echo ""
echo "Musl installed to: $INSTALL_DIR"
echo ""
echo "To build test programs with this musl:"
echo "  export PATH=\"$INSTALL_DIR/bin:\$PATH\""
echo "  riscv64-linux-musl-gcc -g -O0 hello.c -o hello.elf"
echo ""
echo "The test/ directory contains a hello-argv test program."
