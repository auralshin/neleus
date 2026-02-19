# Environment Variables & Secrets Reference
## Quick Lookup Guide

---

## 🎯 What You Need

### Minimum Setup (Local Build Only)

| Tool | Check | Install |
|------|-------|---------|
| Python 3.10+ | `python --version` | https://python.org |
| Rust | `rustc --version` | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Maturin | `maturin --version` | `pip install maturin` |

### For Publishing

| Tool | Check | Install |
|------|-------|---------|
| Twine | `twine --version` | `pip install twine` |
| PyPI account | - | https://pypi.org/account/register/ |
| PyPI token | - | PyPI → Account Settings → API tokens |

---

## 🔐 GitHub Secrets Setup

### Step-by-Step

```
1. Go to: https://github.com/YOUR_USERNAME/neleus
2. Click: Settings tab
3. Click: Secrets and variables → Actions
4. Click: New repository secret
```

### Required Secrets

| Secret Name | Value | Where to Get |
|-------------|-------|--------------|
| `PYPI_TOKEN` | `pypi-AgEIcHlwaS5vcmc...` | PyPI.org → Account Settings → API tokens |
| `TEST_PYPI_TOKEN` | `pypi-AgEIcHlwaS5vcmc...` | TestPyPI.org → Account Settings → API tokens |

### Getting PyPI Tokens

```
PyPI (Production):
1. Go to: https://pypi.org/manage/account/
2. Scroll to: "API tokens"
3. Click: "Add API token"
4. Name: "neleus-github-actions"
5. Scope: "Entire account" (first time) or "Project: neleus" (after first release)
6. Click: "Add token"
7. Copy token (starts with pypi-AgE...)
   ⚠️ SAVE IT NOW! You can't see it again
8. Paste into GitHub secret: PYPI_TOKEN

TestPyPI (Testing):
1. Go to: https://test.pypi.org/manage/account/
2. Same steps as above
3. Save as: TEST_PYPI_TOKEN
```

### Visual: GitHub UI Path

```
GitHub.com
  └─ Your Repository (neleus)
      └─ Settings
          └─ Secrets and variables
              └─ Actions
                  └─ Repository secrets
                      ├─ PYPI_TOKEN ← Add here
                      └─ TEST_PYPI_TOKEN ← Add here
```

---

## 💻 Local Environment Setup

### File: `~/.pypirc`

**Location:** Your home directory (`~/` or `/Users/yourname/`)

**Create it:**
```bash
cp python/.pypirc.template ~/.pypirc
chmod 600 ~/.pypirc
nano ~/.pypirc  # or use your favorite editor
```

**Contents:**
```ini
[distutils]
index-servers =
    pypi
    testpypi

[pypi]
username = __token__
password = pypi-YOUR-ACTUAL-TOKEN-HERE

[testpypi]
repository = https://test.pypi.org/legacy/
username = __token__
password = pypi-YOUR-TEST-TOKEN-HERE
```

**Replace:**
- `pypi-YOUR-ACTUAL-TOKEN-HERE` → Your real PyPI token
- `pypi-YOUR-TEST-TOKEN-HERE` → Your real TestPyPI token

**Security:**
```bash
# Verify permissions
ls -la ~/.pypirc
# Should show: -rw------- (means only you can read/write)

# If not, fix it:
chmod 600 ~/.pypirc
```

---

## 📊 Complete Setup Matrix

### For Building Locally

| Component | Status | Command to Check | Fix/Install |
|-----------|--------|------------------|-------------|
| Python 3.10+ | Required | `python --version` | Download from python.org |
| Rust | Required | `rustc --version` | `curl https://sh.rustup.rs -sSf \| sh` |
| Maturin | Required | `maturin --version` | `pip install maturin` |

### For Publishing Locally

| Component | Status | Location | Setup |
|-----------|--------|----------|-------|
| Twine | Required | System | `pip install twine` |
| PyPI account | Required | pypi.org | Register + verify email + enable 2FA |
| PyPI token | Required | PyPI settings | Generate → Copy → Paste in ~/.pypirc |
| ~/.pypirc file | Required | Home dir | Copy template, edit, chmod 600 |

### For GitHub Actions

| Component | Status | Location | Setup |
|-----------|--------|----------|-------|
| PYPI_TOKEN secret | Required | GitHub repo settings | Add in Secrets → Actions |
| TEST_PYPI_TOKEN | Recommended | GitHub repo settings | Add in Secrets → Actions |
| Workflow file | Already created | `.github/workflows/build-wheels.yml` | Already exists ✓ |

---

## 🚀 Setup Workflows

### Workflow 1: Local Development (Just Building)

```bash
# 1. Install tools
pip install maturin

# 2. Build
cd python
./build_wheels.sh

# Done! Wheel is in dist/
```

**No tokens needed for building!**

---

### Workflow 2: Local Publishing

```bash
# 1. Install tools
pip install maturin twine

# 2. Create PyPI account + token
# Go to: https://pypi.org/account/register/
# Then: https://pypi.org/manage/account/ → API tokens

# 3. Configure ~/.pypirc
cp python/.pypirc.template ~/.pypirc
nano ~/.pypirc  # Add your token
chmod 600 ~/.pypirc

# 4. Build and publish
cd python
./build_wheels.sh
./publish_wheels.sh  # Choose PyPI option
```

---

### Workflow 3: GitHub Actions (Automated)

```bash
# 1. Get PyPI token
# Go to: https://pypi.org/manage/account/ → API tokens
# Copy the token (pypi-AgE...)

# 2. Add to GitHub
# Go to: GitHub repo → Settings → Secrets → Actions
# New secret: PYPI_TOKEN = (paste token)

# 3. Enable auto-publish (optional)
# Edit: .github/workflows/build-wheels.yml
# Change: if: false → if: true

# 4. Trigger build
# Option A: Create a release on GitHub
# Option B: Go to Actions tab → Build Binary Wheels → Run workflow

# Done! GitHub builds wheels for Linux, macOS, Windows
```

---

## 🔍 Verification Commands

### Check Your Local Setup

```bash
# Run automated checker
cd python
./check_environment.sh

# Or manual checks:
python --version     # Should be 3.10+
rustc --version      # Should exist
maturin --version    # Should exist
twine --version      # Should exist
ls -la ~/.pypirc     # Should exist with -rw-------
cat ~/.pypirc        # Should have your token (not placeholder)
```

### Check GitHub Setup

```bash
# You can't directly check secrets, but verify they exist:
# 1. Go to: https://github.com/YOUR_USERNAME/neleus/settings/secrets/actions
# 2. You should see:
#    - PYPI_TOKEN ✓
#    - TEST_PYPI_TOKEN ✓ (optional)
```

### Test the Workflow

```bash
# 1. Build locally first
cd python
./build_wheels.sh

# 2. Test publish to TestPyPI
./publish_wheels.sh  # Choose option 1

# 3. Verify you can install
pip install --index-url https://test.pypi.org/simple/ neleus

# 4. If that works, you're ready for production!
```

---

## ❌ Common Mistakes

### Mistake 1: Token in Wrong Place
```
❌ Token in .pypirc but variable name wrong
✅ Must be: password = pypi-YOUR-TOKEN

❌ Token has quotes: password = "pypi-..."
✅ No quotes: password = pypi-...

❌ Token has spaces or newlines
✅ Single line, no spaces
```

### Mistake 2: GitHub Secret Name Wrong
```
❌ Secret named: PYPI-TOKEN (hyphen)
✅ Must be: PYPI_TOKEN (underscore)

❌ Secret named: PyPI_Token (mixed case)
✅ Must be: PYPI_TOKEN (all caps)
```

### Mistake 3: Wrong Token Scope
```
❌ Token scope: "Read packages"
✅ Must be: "Upload packages" or "Entire account"
```

### Mistake 4: 2FA Not Enabled
```
❌ PyPI account without 2FA
✅ Enable 2FA first, then create token
```

---

## 📝 Minimal Setup Checklist

### Local Development (No Publishing)

- [ ] Python 3.10+ installed
- [ ] Rust installed
- [ ] Maturin installed: `pip install maturin`
- [ ] Run: `./build_wheels.sh`

### Local Publishing

- [ ] All "Local Development" requirements above
- [ ] Twine installed: `pip install twine`
- [ ] PyPI account created
- [ ] 2FA enabled on PyPI
- [ ] API token generated
- [ ] `~/.pypirc` created and configured
- [ ] `~/.pypirc` permissions set to 600
- [ ] Run: `./publish_wheels.sh`

### GitHub Actions

- [ ] PyPI account + token
- [ ] Token added as `PYPI_TOKEN` secret in GitHub
- [ ] (Optional) TestPyPI token as `TEST_PYPI_TOKEN`
- [ ] Push code to GitHub
- [ ] Create a release or manually trigger workflow

---

## 🆘 Quick Help

**Can't build?**
```bash
./check_environment.sh  # Diagnosis tool
```

**Can't publish?**
```bash
# Check credentials
cat ~/.pypirc | grep password
# Should show: password = pypi-AgE... (real token, not placeholder)

# Test twine
twine check dist/*.whl
```

**GitHub Actions failing?**
```
1. Go to: Actions tab in your repo
2. Click failed workflow
3. Read error message
4. Usually: secret not set or workflow syntax error
```

---

## 📚 Full Documentation

- **Complete guide:** [ENVIRONMENT_SETUP.md](ENVIRONMENT_SETUP.md)
- **Quick commands:** `./QUICK_REFERENCE.sh`
- **Build process:** [BINARY_DISTRIBUTION.md](BINARY_DISTRIBUTION.md)
- **Release checklist:** [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)
- **Check setup:** `./check_environment.sh`

---

**Last Updated:** 2026-02-13
