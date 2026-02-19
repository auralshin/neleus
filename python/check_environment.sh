#!/bin/bash
# Quick Setup Script for Local Environment
# Run this to verify your local setup is ready

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔═══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Neleus Local Environment Setup Checker                ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Track issues
ISSUES=0

# Check Python
echo -n "Checking Python version... "
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
    MAJOR=$(echo $PYTHON_VERSION | cut -d'.' -f1)
    MINOR=$(echo $PYTHON_VERSION | cut -d'.' -f2)
    if [ "$MAJOR" -ge 3 ] && [ "$MINOR" -ge 10 ]; then
        echo -e "${GREEN}✓ Python $PYTHON_VERSION${NC}"
    else
        echo -e "${RED}✗ Python $PYTHON_VERSION (need 3.10+)${NC}"
        ISSUES=$((ISSUES + 1))
    fi
else
    echo -e "${RED}✗ Python not found${NC}"
    ISSUES=$((ISSUES + 1))
fi

# Check Rust
echo -n "Checking Rust toolchain... "
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    echo -e "${GREEN}✓ Rust $RUST_VERSION${NC}"
else
    echo -e "${RED}✗ Rust not installed${NC}"
    echo -e "${YELLOW}  Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    ISSUES=$((ISSUES + 1))
fi

# Check Cargo
echo -n "Checking Cargo... "
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version | cut -d' ' -f2)
    echo -e "${GREEN}✓ Cargo $CARGO_VERSION${NC}"
else
    echo -e "${RED}✗ Cargo not found${NC}"
    ISSUES=$((ISSUES + 1))
fi

# Check Maturin
echo -n "Checking Maturin... "
if command -v maturin &> /dev/null || python3 -m maturin --version &> /dev/null; then
    MATURIN_VERSION=$(maturin --version 2>/dev/null || python3 -m maturin --version)
    echo -e "${GREEN}✓ Maturin installed${NC}"
else
    echo -e "${YELLOW}⚠ Maturin not installed${NC}"
    echo -e "${YELLOW}  Install: pip install maturin${NC}"
    ISSUES=$((ISSUES + 1))
fi

# Check Twine
echo -n "Checking Twine... "
if python3 -m twine --version &> /dev/null 2>&1; then
    TWINE_VERSION=$(python3 -m twine --version 2>&1)
    echo -e "${GREEN}✓ Twine installed${NC}"
else
    echo -e "${YELLOW}⚠ Twine not installed${NC}"
    echo -e "${YELLOW}  Install: pip install twine${NC}"
    ISSUES=$((ISSUES + 1))
fi

echo ""
echo -e "${BLUE}Configuration Files:${NC}"

# Check .pypirc
echo -n "Checking ~/.pypirc... "
if [ -f ~/.pypirc ]; then
    PERMS=$(stat -f "%p" ~/.pypirc 2>/dev/null || stat -c "%a" ~/.pypirc 2>/dev/null)
    PERMS_SHORT=${PERMS: -3}
    if [ "$PERMS_SHORT" = "600" ]; then
        echo -e "${GREEN}✓ Exists with correct permissions (600)${NC}"
    else
        echo -e "${YELLOW}⚠ Exists but permissions are $PERMS_SHORT (should be 600)${NC}"
        echo -e "${YELLOW}  Fix: chmod 600 ~/.pypirc${NC}"
        ISSUES=$((ISSUES + 1))
    fi
    
    # Check for token placeholder
    if grep -q "YOUR-.*-TOKEN-HERE" ~/.pypirc 2>/dev/null; then
        echo -e "${YELLOW}  ⚠ Contains placeholder tokens - needs configuration${NC}"
        ISSUES=$((ISSUES + 1))
    fi
else
    echo -e "${RED}✗ Not found${NC}"
    echo -e "${YELLOW}  Create: cp python/.pypirc.template ~/.pypirc${NC}"
    echo -e "${YELLOW}  Then edit and add your PyPI tokens${NC}"
    ISSUES=$((ISSUES + 1))
fi

# Check gitignore
echo -n "Checking .gitignore... "
if grep -q "^\.pypirc$" ../.gitignore 2>/dev/null; then
    echo -e "${GREEN}✓ .pypirc is ignored${NC}"
else
    echo -e "${YELLOW}⚠ .pypirc might not be in .gitignore${NC}"
    ISSUES=$((ISSUES + 1))
fi

if grep -q "^dist/$\|^/dist/$" ../.gitignore 2>/dev/null; then
    echo -e "${GREEN}✓ dist/ is ignored${NC}"
else
    echo -e "${YELLOW}⚠ dist/ might not be in .gitignore${NC}"
    ISSUES=$((ISSUES + 1))
fi

echo ""
echo -e "${BLUE}Build Scripts:${NC}"

# Check build scripts exist and are executable
for script in build_wheels.sh publish_wheels.sh QUICK_REFERENCE.sh; do
    echo -n "Checking $script... "
    if [ -f "$script" ]; then
        if [ -x "$script" ]; then
            echo -e "${GREEN}✓ Exists and executable${NC}"
        else
            echo -e "${YELLOW}⚠ Exists but not executable${NC}"
            echo -e "${YELLOW}  Fix: chmod +x $script${NC}"
            ISSUES=$((ISSUES + 1))
        fi
    else
        echo -e "${RED}✗ Not found${NC}"
        ISSUES=$((ISSUES + 1))
    fi
done

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"

if [ $ISSUES -eq 0 ]; then
    echo -e "${GREEN}✅ All checks passed! Your environment is ready.${NC}"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    echo "1. Build wheels: ./build_wheels.sh"
    echo "2. Publish: ./publish_wheels.sh"
else
    echo -e "${YELLOW}⚠️  Found $ISSUES issue(s) - see above for fixes${NC}"
    echo ""
    echo -e "${BLUE}Quick fixes:${NC}"
    echo "pip install maturin twine"
    echo "cp .pypirc.template ~/.pypirc && chmod 600 ~/.pypirc"
    echo "# Then edit ~/.pypirc with your PyPI tokens"
fi

echo ""
echo -e "${BLUE}Documentation:${NC}"
echo "• Full guide: ENVIRONMENT_SETUP.md"
echo "• Quick reference: ./QUICK_REFERENCE.sh"
echo "• Release checklist: RELEASE_CHECKLIST.md"
echo ""
