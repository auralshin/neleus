#!/bin/bash
# Quick Reference: Neleus Binary Distribution

cat << 'EOF'
╔═══════════════════════════════════════════════════════════════════════════╗
║                   NELEUS BINARY DISTRIBUTION QUICK REFERENCE              ║
║                          (Protecting Company IP)                          ║
╚═══════════════════════════════════════════════════════════════════════════╝

📦 BUILD BINARY WHEELS
──────────────────────────────────────────────────────────────────────────────
  cd python
  ./build_wheels.sh
  
  Output: dist/*.whl (platform-specific binary)

🚀 PUBLISH TO PACKAGE INDEX
──────────────────────────────────────────────────────────────────────────────
  cd python
  ./publish_wheels.sh
  
  Options:
    1. TestPyPI (testing)
    2. PyPI (production)
    3. Private index

🔐 SECURITY CHECKLIST
──────────────────────────────────────────────────────────────────────────────
  ✓ Run build_wheels.sh (auto-checks for source)
  ✓ Verify wheel: unzip -l dist/*.whl | grep -E '\.rs|Cargo'
  ✓ Test in clean environment: pip install dist/*.whl
  ✓ Never commit dist/ or .pypirc to git
  ✓ Only distribute .whl files, NEVER .tar.gz (source)

🌍 CROSS-PLATFORM BUILDS
──────────────────────────────────────────────────────────────────────────────
  Option A - Manual:
    Build on each OS (Linux, macOS, Windows)
  
  Option B - Automated:
    GitHub Actions: .github/workflows/build-wheels.yml
    Trigger: Create a release or run manually

👥 USER INSTALLATION
──────────────────────────────────────────────────────────────────────────────
  # From PyPI
  pip install neleus
  
  # From private index
  pip install --extra-index-url https://your-index/simple/ neleus
  
  # From local wheel
  pip install neleus-0.1.0-*.whl

⚠️  WHAT TO NEVER DO
──────────────────────────────────────────────────────────────────────────────
  ✗ Don't run: python setup.py sdist (creates source distribution)
  ✗ Don't run: maturin build without --strip flag
  ✗ Don't commit: dist/, build/, *.whl to git
  ✗ Don't upload: source code to public repositories
  ✗ Don't share: Rust source or ../crates/ directory

📁 KEY FILES
──────────────────────────────────────────────────────────────────────────────
  python/
    ├── build_wheels.sh          ← Build script
    ├── publish_wheels.sh        ← Publishing script
    ├── BINARY_DISTRIBUTION.md   ← Full documentation
    ├── MANIFEST.in              ← Excludes Rust source
    ├── .pypiignore              ← Extra protection
    └── pyproject.toml           ← Config (binary-only)

🔧 TROUBLESHOOTING
──────────────────────────────────────────────────────────────────────────────
  Problem: "maturin: command not found"
  Solution: pip install maturin

  Problem: Wheel too large
  Solution: Ensure --strip flag is used (removes debug symbols)

  Problem: Import error after install
  Solution: Check Python version compatibility in pyproject.toml

  Problem: Want to verify wheel contents
  Solution: unzip -l dist/neleus-*.whl | less

📚 MORE INFO
──────────────────────────────────────────────────────────────────────────────
  See: python/BINARY_DISTRIBUTION.md

EOF
