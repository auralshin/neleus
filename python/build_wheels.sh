#!/bin/bash
# Neleus Binary Wheel Build Script
# This script builds platform-specific binary wheels WITHOUT source code
# to protect company IP

set -e

echo "🔒 Building Neleus binary wheels (IP-protected)"
echo "================================================"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Change to python directory
cd "$(dirname "$0")"

# Check if maturin is installed
if ! command -v maturin &> /dev/null; then
    echo -e "${RED}Error: maturin is not installed${NC}"
    echo "Install with: pip install maturin"
    exit 1
fi

# Clean previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"
rm -rf dist/ build/ *.egg-info
rm -rf target/wheels/*

# Build the wheel for the current platform
echo -e "${YELLOW}Building binary wheel...${NC}"
maturin build --release --strip

# Verify no source code is in the wheel
echo -e "${YELLOW}Verifying wheel contents...${NC}"
WHEEL_FILE=$(ls target/wheels/*.whl | head -n 1)

if [ -z "$WHEEL_FILE" ]; then
    echo -e "${RED}Error: No wheel file generated${NC}"
    exit 1
fi

echo "Checking wheel: $WHEEL_FILE"
unzip -l "$WHEEL_FILE" | grep -E '\.rs$|Cargo\.toml' && {
    echo -e "${RED}ERROR: Rust source code found in wheel!${NC}"
    exit 1
} || {
    echo -e "${GREEN}✓ No Rust source code detected${NC}"
}

# Copy to dist directory
mkdir -p dist
cp target/wheels/*.whl dist/

echo -e "${GREEN}✓ Binary wheel built successfully${NC}"
echo "Wheel location: dist/"
ls -lh dist/

echo ""
echo -e "${YELLOW}⚠️  IMPORTANT NOTES:${NC}"
echo "1. This wheel only works on your current platform"
echo "2. To support multiple platforms, build on each platform separately"
echo "3. Never distribute source code or upload to public repos"
echo "4. Use trusted PyPI or private package index"
