# Release Security Checklist
## Neleus Binary Distribution

Use this checklist before EVERY release to ensure no source code leaks.

## Pre-Build Checks

- [ ] Version updated in `pyproject.toml`
- [ ] CHANGELOG updated (if applicable)
- [ ] All tests passing: `pytest`
- [ ] Code formatted: `black . && ruff check .`
- [ ] No uncommitted changes related to build

## Build Process

- [ ] Clean previous builds: `rm -rf dist/ build/ target/wheels/`
- [ ] Run build script: `./build_wheels.sh`
- [ ] Build completed without errors
- [ ] Wheel files created in `dist/` or `target/wheels/`

## Security Verification (CRITICAL)

### Automated Checks
- [ ] Build script security check passed (auto-run)
- [ ] No `.rs` files in wheel: `unzip -l dist/*.whl | grep -v '\.rs'`
- [ ] No `Cargo.toml` in wheel: `unzip -l dist/*.whl | grep -v 'Cargo'`
- [ ] No `crates/` directory references: `unzip -l dist/*.whl | grep -v 'crates/'`

### Manual Inspection
```bash
# Extract and inspect wheel contents
cd dist
mkdir -p inspect
unzip -q *.whl -d inspect
cd inspect
```

- [ ] Manually verify no `.rs` files: `find . -name "*.rs"`
- [ ] Manually verify no `Cargo.toml`: `find . -name "Cargo.toml"`
- [ ] Check for source directories: `ls -la`
- [ ] Only see: Python files, compiled binaries (.so, .pyd), metadata

Expected contents:
```
neleus/
  __init__.py
  cli/
    *.py files
  ui/
    *.html, *.css, *.js
  neleus_core.*.so  (or .pyd on Windows)  ← Compiled Rust code
  *.py files
neleus-0.1.0.dist-info/
  metadata files
```

### File Size Check
- [ ] Wheel size reasonable (typically 2-10 MB compressed)
- [ ] Not suspiciously large (may indicate source inclusion)
- [ ] Run: `ls -lh dist/*.whl`

## Installation Test

- [ ] Create fresh virtual environment: `python -m venv test_env`
- [ ] Activate: `source test_env/bin/activate`
- [ ] Install wheel: `pip install dist/*.whl`
- [ ] Test CLI: `neleus --version`
- [ ] Test CLI: `neleus --help`
- [ ] Test import: `python -c "import neleus; print(neleus.__version__)"`
- [ ] Run quick test: `python -c "from neleus import Strategy; print('OK')"`
- [ ] Deactivate and remove: `deactivate && rm -rf test_env`

## Cross-Platform Verification (if applicable)

If using GitHub Actions or building on multiple platforms:

- [ ] Linux wheel built successfully
- [ ] macOS (Intel) wheel built successfully
- [ ] macOS (ARM) wheel built successfully
- [ ] Windows wheel built successfully
- [ ] All wheels passed security checks
- [ ] Downloaded and spot-checked artifacts

## Publishing Preparation

- [ ] `.pypirc` configured with correct credentials
- [ ] Decision made: PyPI, TestPyPI, or private index?
- [ ] Twine installed: `pip install twine`
- [ ] Check package metadata: `twine check dist/*`

### TestPyPI (Recommended First)
- [ ] Publish to TestPyPI: `./publish_wheels.sh` → option 1
- [ ] Install from TestPyPI: `pip install --index-url https://test.pypi.org/simple/ neleus`
- [ ] Test installation works correctly
- [ ] Verify no source code accessible

## Production Publishing

- [ ] All above checks completed and passed
- [ ] Final approval obtained (if required)
- [ ] Backup of wheels created: `cp -r dist/ dist-backup-$(date +%Y%m%d)/`
- [ ] Run publish script: `./publish_wheels.sh`
- [ ] Choose production option (PyPI or private index)
- [ ] Confirm upload successful
- [ ] Verify package visible on index
- [ ] Test installation from production: `pip install neleus`

## Post-Release

- [ ] Tag release in git: `git tag v0.1.0`
- [ ] Push tag: `git push origin v0.1.0`
- [ ] Create GitHub release with notes
- [ ] Attach wheels to GitHub release (optional)
- [ ] Update documentation with new version
- [ ] Announce release (internally)
- [ ] Clean up local build artifacts: `rm -rf dist/ build/`
- [ ] Verify dist/ not in git: `git status`

## Emergency Rollback (if needed)

If source code is discovered after publishing:

- [ ] Immediately yank package version: `twine upload --skip-existing` won't help
- [ ] Contact PyPI support to remove package
- [ ] Audit build process for failure
- [ ] Re-verify all security measures
- [ ] Build and test new version
- [ ] Increment version number
- [ ] Repeat full checklist

## Quick Command Reference

```bash
# Security checks
unzip -l dist/*.whl | grep -E '\.rs$|Cargo\.toml'  # Should show nothing
find dist/ -name "*.whl" -exec unzip -l {} \; | grep -c "\.rs"  # Should be 0

# Clean verification
python -m zipfile -l dist/*.whl | grep -vE '__pycache__|\.pyc' | less

# Test install
python -m venv temp && source temp/bin/activate && pip install dist/*.whl && neleus --version && deactivate && rm -rf temp
```

## Notes

- This checklist should be completed by at least ONE person before release
- For critical releases, have TWO people verify independently
- Keep a record of checklist completion (date, person, version)
- Update this checklist as new threats or checks are identified

---

**Release Version:** _________  
**Date:** _________  
**Verified By:** _________  
**Signature:** _________  

✅ All checks completed - SAFE TO RELEASE
