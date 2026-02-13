#!/bin/bash
# Neleus Binary Wheel Publishing Script
# ONLY publishes pre-built binary wheels - NO SOURCE CODE

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

cd "$(dirname "$0")"

echo "🚀 Neleus Binary Package Publisher"
echo "===================================="
echo ""

# Check if wheels exist
if [ ! -d "dist" ] || [ -z "$(ls -A dist/*.whl 2>/dev/null)" ]; then
    echo -e "${RED}Error: No wheel files found in dist/${NC}"
    echo "Run ./build_wheels.sh first"
    exit 1
fi

# Double-check no source code
echo -e "${YELLOW}Security check: Verifying no source code in wheels...${NC}"
for wheel in dist/*.whl; do
    echo "Checking: $wheel"
    unzip -l "$wheel" | grep -E '\.rs$|Cargo\.toml' && {
        echo -e "${RED}ABORT: Rust source found in $wheel${NC}"
        exit 1
    } || {
        echo -e "${GREEN}✓ Clean${NC}"
    }
done

echo ""
echo -e "${YELLOW}Choose publishing target:${NC}"
echo "1) TestPyPI (testing)"
echo "2) PyPI (production)"
echo "3) Private package index"
read -p "Enter choice [1-3]: " choice

case $choice in
    1)
        echo -e "${YELLOW}Publishing to TestPyPI...${NC}"
        # Ensure you have credentials configured in ~/.pypirc
        python -m twine upload --repository testpypi dist/*.whl
        echo -e "${GREEN}✓ Published to TestPyPI${NC}"
        echo "Test with: pip install --index-url https://test.pypi.org/simple/ neleus"
        ;;
    2)
        echo -e "${RED}⚠️  WARNING: Publishing to production PyPI${NC}"
        read -p "Are you sure? Type 'yes' to confirm: " confirm
        if [ "$confirm" = "yes" ]; then
            python -m twine upload dist/*.whl
            echo -e "${GREEN}✓ Published to PyPI${NC}"
            echo "Install with: pip install neleus"
        else
            echo "Cancelled"
            exit 0
        fi
        ;;
    3)
        read -p "Enter private package index URL: " repo_url
        read -p "Enter repository name (from ~/.pypirc): " repo_name
        python -m twine upload --repository "$repo_name" dist/*.whl
        echo -e "${GREEN}✓ Published to private index${NC}"
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✅ Publishing complete${NC}"
echo -e "${YELLOW}Remember:${NC}"
echo "- Binary wheels are platform-specific"
echo "- Build on Linux, macOS, and Windows separately for full coverage"
echo "- Never commit dist/ directory to git"
