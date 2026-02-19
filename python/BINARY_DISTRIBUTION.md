# Neleus Binary Distribution Guide
## Protecting Company IP

This document outlines how to distribute the Neleus CLI as binary-only packages, protecting your Rust source code.

## 🔒 Security Strategy

The Neleus package is distributed as **pre-compiled binary wheels only**. This means:
- ✅ Users get a working CLI tool
- ✅ Rust code is compiled into binary form (.so, .pyd, .dylib)
- ✅ No source code is included in distributed packages
- ✅ Company IP remains protected

## 📦 Building Binary Wheels

### Prerequisites
```bash
pip install maturin twine
```

### Build for Your Platform
```bash
cd python
./build_wheels.sh
```

This creates a platform-specific wheel in `dist/` directory.

### Build for Multiple Platforms

To support all users, build on each target platform:

**macOS (Intel):**
```bash
./build_wheels.sh
# Produces: neleus-0.1.0-cp310-cp310-macosx_10_12_x86_64.whl
```

**macOS (Apple Silicon):**
```bash
./build_wheels.sh
# Produces: neleus-0.1.0-cp310-cp310-macosx_11_0_arm64.whl
```

**Linux:**
```bash
./build_wheels.sh
# Produces: neleus-0.1.0-cp310-cp310-manylinux_2_28_x86_64.whl
```

**Windows:**
```bash
maturin build --release --strip
# Produces: neleus-0.1.0-cp310-cp310-win_amd64.whl
```

### Using GitHub Actions (Recommended)

Create `.github/workflows/build-wheels.yml` for automated multi-platform builds:

```yaml
name: Build Wheels

on:
  release:
    types: [created]
  workflow_dispatch:

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        python-version: ['3.10', '3.11', '3.12']
    
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v4
        with:
          python-version: ${{ matrix.python-version }}
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Build wheel
        run: |
          pip install maturin
          cd python
          maturin build --release --strip
      
      - name: Verify no source in wheel
        shell: bash
        run: |
          wheel=$(ls python/target/wheels/*.whl)
          unzip -l "$wheel" | grep -E '\.rs$|Cargo\.toml' && exit 1 || echo "Clean"
      
      - uses: actions/upload-artifact@v3
        with:
          name: wheels
          path: python/target/wheels/*.whl
```

## 🚀 Publishing

### Option 1: PyPI (Public)

```bash
cd python
./publish_wheels.sh
# Choose option 2 for PyPI
```

**Setup PyPI credentials:**
```bash
# Create ~/.pypirc
cat > ~/.pypirc << EOF
[pypi]
username = __token__
password = pypi-YOUR-TOKEN-HERE
EOF
```

### Option 2: Private Package Index

For complete privacy, use a private package index:

**Using AWS CodeArtifact:**
```bash
aws codeartifact login --tool pip --domain your-domain --repository your-repo
cd python
python -m twine upload --repository codeartifact dist/*.whl
```

**Using Google Artifact Registry:**
```bash
gcloud artifacts print-settings python --repository=your-repo --location=us-central1
cd python
twine upload --repository-url https://us-central1-python.pkg.dev/... dist/*.whl
```

**Using Gemfury (Simple Private PyPI):**
```bash
curl -F package=@dist/neleus-0.1.0-*.whl https://YOUR-TOKEN@push.fury.io/your-account/
```

### Option 3: Direct Distribution

For enterprise customers, distribute wheels directly:

```bash
# Users install from local file
pip install neleus-0.1.0-cp310-cp310-macosx_11_0_arm64.whl

# Or from a URL
pip install https://your-company.com/dist/neleus-0.1.0-*.whl
```

## 🔐 Security Checklist

Before publishing, verify:

- [ ] Run `./build_wheels.sh` - it checks for source code
- [ ] Manually inspect wheel contents:
  ```bash
  unzip -l dist/neleus-*.whl | less
  ```
- [ ] Verify no `.rs` files present
- [ ] Verify no `Cargo.toml` present
- [ ] Verify no `../crates/` references
- [ ] Test installation in a clean environment
- [ ] Never commit `dist/` directory to git
- [ ] Never upload source code to public repos

## 📥 User Installation

Users install the binary package normally:

```bash
# From PyPI
pip install neleus

# From private index
pip install --extra-index-url https://your-private-index.com/simple/ neleus

# Verify installation
neleus --version
neleus --help
```

## 🛠️ Maintenance

### Updating the Package

1. Update version in `pyproject.toml`
2. Build new wheels: `./build_wheels.sh`
3. Test locally: `pip install dist/*.whl`
4. Publish: `./publish_wheels.sh`

### Supporting New Python Versions

Update `pyproject.toml`:
```toml
requires-python = ">=3.10,<3.14"
```

Build for the new version on each platform.

## ⚠️ Important Notes

1. **Binary wheels are platform-specific** - build on every target OS
2. **No source distribution (sdist)** - users cannot build from source
3. **Keep Rust code private** - never push to public repos
4. **License considerations** - ensure your license allows binary-only distribution
5. **Dependencies** - maturin bundles Rust code, Python deps installed separately

## 🎯 Recommended Workflow

For production releases:

1. **Development:** Test locally with `maturin develop`
2. **Build:** Use GitHub Actions for multi-platform wheels
3. **Test:** Install wheels in clean environments
4. **Security Review:** Verify no source code in any wheel
5. **Publish:** Push only to trusted package indexes
6. **Document:** Provide installation instructions to users

## 📞 Support

For issues with binary distribution:
- Check wheel contents with `unzip -l`
- Verify platform compatibility
- Ensure maturin is up to date
- Test in a fresh Python environment
