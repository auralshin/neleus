# Environment Setup Guide
## GitHub Actions & Local Development

This guide explains all environment variables, secrets, and configurations needed for building and publishing Neleus binary wheels.

---

## 📋 Table of Contents

1. [GitHub Actions Setup](#github-actions-setup)
2. [Local Development Setup](#local-development-setup)
3. [PyPI Token Setup](#pypi-token-setup)
4. [Private Package Index Setup](#private-package-index-setup)
5. [Troubleshooting](#troubleshooting)

---

## 🔧 GitHub Actions Setup

### Required Secrets

Configure these in your GitHub repository settings.

#### 1. Navigate to Repository Settings

```
Your Repo → Settings → Secrets and variables → Actions
```

#### 2. Add Required Secrets

| Secret Name | Description | Required For | How to Get |
|-------------|-------------|--------------|------------|
| `PYPI_TOKEN` | PyPI API token | Publishing to PyPI | [See PyPI Token Setup](#pypi-token-setup) |
| `TEST_PYPI_TOKEN` | TestPyPI API token | Testing before production | [See PyPI Token Setup](#pypi-token-setup) |

#### 3. Optional Secrets (for private indexes)

| Secret Name | Description | Example Value |
|-------------|-------------|---------------|
| `PRIVATE_REPO_URL` | Private package index URL | `https://pypi.company.com/simple/` |
| `PRIVATE_REPO_USERNAME` | Username for private index | `your-username` |
| `PRIVATE_REPO_PASSWORD` | Password/token for private index | `your-token-or-password` |
| `AWS_ACCESS_KEY_ID` | AWS credentials (if using CodeArtifact) | `AKIA...` |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key | `wJa...` |
| `GCP_CREDENTIALS` | GCP service account JSON (if using Artifact Registry) | `{"type":"service_account",...}` |

### Step-by-Step: Adding Secrets to GitHub

1. **Go to your repository on GitHub**
   ```
   https://github.com/YOUR_USERNAME/neleus
   ```

2. **Click Settings tab**

3. **In left sidebar:**
   - Click "Secrets and variables"
   - Click "Actions"

4. **Click "New repository secret"**

5. **For PyPI:**
   - Name: `PYPI_TOKEN`
   - Value: `pypi-AgE...` (your token from PyPI)
   - Click "Add secret"

6. **For TestPyPI (recommended for testing):**
   - Name: `TEST_PYPI_TOKEN`
   - Value: `pypi-AgE...` (your token from TestPyPI)
   - Click "Add secret"

### Using Secrets in Workflow

The workflow already references these secrets. To enable automatic publishing:

**Edit `.github/workflows/build-wheels.yml`:**

```yaml
# Change this line from:
if: false  # Change to 'true' to enable

# To:
if: true  # Enabled!
```

### Environment Protection (Recommended)

For extra security, use GitHub Environments:

1. **Settings → Environments → New environment**
2. **Name it:** `pypi`
3. **Add protection rules:**
   - ✅ Required reviewers (select team members)
   - ✅ Wait timer (optional: 5-10 minutes)
4. **Add environment secrets:**
   - Add `PYPI_TOKEN` here instead of repository secrets
5. **Update workflow:**
   ```yaml
   publish-pypi:
     environment: pypi  # Uncomment this line in the workflow
   ```

Now publishing requires manual approval! 🛡️

---

## 💻 Local Development Setup

### 1. Install Required Tools

```bash
# Python 3.10+
python --version

# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Maturin (Python-Rust build tool)
pip install maturin

# Twine (for publishing)
pip install twine
```

### 2. PyPI Credentials File

Create `~/.pypirc` for local publishing:

```bash
# Copy template
cp python/.pypirc.template ~/.pypirc

# Edit with your credentials
nano ~/.pypirc
```

**File contents:**

```ini
[distutils]
index-servers =
    pypi
    testpypi
    private

# Production PyPI
[pypi]
username = __token__
password = pypi-YOUR-PRODUCTION-TOKEN-HERE

# Test PyPI (for testing)
[testpypi]
repository = https://test.pypi.org/legacy/
username = __token__
password = pypi-YOUR-TEST-TOKEN-HERE

# Private index (optional)
[private]
repository = https://your-private-index.com/simple/
username = your-username
password = your-password
```

**Security:**

```bash
# Set restrictive permissions
chmod 600 ~/.pypirc

# Verify it's not in git
cat .gitignore | grep pypirc  # Should show .pypirc
```

### 3. Environment Variables (Optional)

For scripting or CI/CD, you can use environment variables:

**Create `.env` file in project root:**

```bash
# PyPI
export TWINE_USERNAME=__token__
export TWINE_PASSWORD=pypi-YOUR-TOKEN-HERE

# Or for TestPyPI
export TWINE_REPOSITORY=testpypi
export TWINE_USERNAME=__token__
export TWINE_PASSWORD=pypi-YOUR-TEST-TOKEN-HERE

# Private index (if applicable)
export TWINE_REPOSITORY_URL=https://your-index.com/simple/
export TWINE_USERNAME=your-username
export TWINE_PASSWORD=your-password
```

**Load before publishing:**

```bash
source .env
twine upload dist/*.whl
```

**⚠️ NEVER commit `.env` to git!**

```bash
# Verify it's in .gitignore
grep -E "^\.env$" .gitignore
```

### 4. AWS CodeArtifact (if using)

```bash
# Configure AWS CLI
aws configure
# Enter: Access Key ID, Secret Access Key, Region

# Login to CodeArtifact
aws codeartifact login \
  --tool pip \
  --domain your-domain \
  --repository your-repo \
  --region us-east-1

# This updates ~/.pypirc automatically
```

### 5. Google Artifact Registry (if using)

```bash
# Install gcloud CLI
# Follow: https://cloud.google.com/sdk/docs/install

# Authenticate
gcloud auth login

# Configure for Python
gcloud artifacts print-settings python \
  --repository=your-repo \
  --location=us-central1 \
  --project=your-project

# Add credentials
gcloud auth application-default login
```

---

## 🔑 PyPI Token Setup

### Getting Your PyPI Token

#### For Production PyPI:

1. **Create account:** https://pypi.org/account/register/
2. **Verify email**
3. **Enable 2FA** (required for API tokens)
4. **Go to Account Settings:** https://pypi.org/manage/account/
5. **Scroll to "API tokens"**
6. **Click "Add API token"**
   - Token name: `neleus-github-actions` (or any name)
   - Scope: 
     - "Entire account" (for first upload)
     - OR "Project: neleus" (after first upload)
7. **Copy token** (starts with `pypi-AgE...`)
   - ⚠️ Save it now! You can't see it again
8. **Store securely:**
   - GitHub: Add as `PYPI_TOKEN` secret
   - Local: Add to `~/.pypirc`

#### For TestPyPI (Testing):

1. **Create account:** https://test.pypi.org/account/register/
2. **Same process as above**
3. **Token name:** `neleus-testing`
4. **Save as:** `TEST_PYPI_TOKEN`

### Token Scopes

| Scope | Use Case | Security |
|-------|----------|----------|
| **Entire account** | First time publishing new package | Less secure, can access all projects |
| **Specific project** | After package exists | More secure, only this package |

**Best Practice:**
1. Use "Entire account" for first release
2. Delete it immediately after
3. Create new project-scoped token
4. Update GitHub secrets with new token

---

## 🏢 Private Package Index Setup

### Option 1: AWS CodeArtifact

**Required Environment Variables:**

```bash
# GitHub Secrets
AWS_ACCESS_KEY_ID=AKIA...
AWS_SECRET_ACCESS_KEY=wJa...
AWS_DEFAULT_REGION=us-east-1
CODEARTIFACT_DOMAIN=your-domain
CODEARTIFACT_REPOSITORY=your-repo
```

**Local Setup:**

```bash
# Configure AWS CLI
aws configure

# Login (updates ~/.pypirc)
aws codeartifact login \
  --tool twine \
  --domain your-domain \
  --repository your-repo
```

**Publishing:**

```bash
# Upload using Twine
twine upload \
  --repository codeartifact \
  dist/*.whl
```

### Option 2: Google Artifact Registry

**Required Environment Variables:**

```bash
# GitHub Secrets
GCP_CREDENTIALS='{"type":"service_account",...}'
GCP_PROJECT=your-project
GCP_LOCATION=us-central1
GCP_REPOSITORY=your-repo
```

**Local Setup:**

```bash
# Authenticate
gcloud auth login

# Configure pip
gcloud artifacts print-settings python \
  --repository=your-repo \
  --location=us-central1
```

**Publishing:**

```bash
twine upload \
  --repository-url https://us-central1-python.pkg.dev/PROJECT/REPO/ \
  dist/*.whl
```

### Option 3: Azure Artifacts

**Required Environment Variables:**

```bash
# GitHub Secrets
AZURE_DEVOPS_ORG=your-org
AZURE_DEVOPS_PROJECT=your-project
AZURE_ARTIFACTS_FEED=your-feed
AZURE_DEVOPS_PAT=your-personal-access-token
```

**Local Setup:**

```bash
# Install Azure CLI
# Follow: https://docs.microsoft.com/en-us/cli/azure/install-azure-cli

# Login
az login

# Get feed URL
az artifacts universal publish \
  --organization your-org \
  --feed your-feed \
  --name neleus \
  --version 0.1.0 \
  --path dist/*.whl
```

### Option 4: Gemfury (Simple Private PyPI)

**Required Environment Variables:**

```bash
# GitHub Secrets
FURY_TOKEN=your-token

# Or just add to ~/.pypirc
[fury]
repository = https://pypi.fury.io/YOUR-ACCOUNT/
username = YOUR-TOKEN
password = ""
```

**Publishing:**

```bash
# Using curl (simplest)
curl -F package=@dist/neleus-0.1.0-*.whl \
  https://YOUR-TOKEN@push.fury.io/YOUR-ACCOUNT/

# Or using Twine
twine upload --repository fury dist/*.whl
```

---

## 🔍 Verification Checklist

### Local Environment

```bash
# Check Python
python --version  # Should be 3.10+

# Check Rust
rustc --version   # Should be 1.70+

# Check Maturin
maturin --version # Should be 1.0+

# Check Twine
twine --version   # Should be 4.0+

# Verify .pypirc exists and has correct permissions
ls -la ~/.pypirc  # Should show -rw------- (600)

# Test PyPI credentials
twine check dist/*.whl  # If you have wheels built
```

### GitHub Secrets

```bash
# You can't view secrets, but verify they're set:
# Go to: Settings → Secrets and variables → Actions
# You should see:
# - PYPI_TOKEN ✓
# - TEST_PYPI_TOKEN ✓
```

---

## 🐛 Troubleshooting

### "Invalid API token"

**Problem:** PyPI rejects your token

**Solutions:**
1. Check token starts with `pypi-`
2. Verify 2FA is enabled on PyPI account
3. Check token scope (needs upload permission)
4. Token might be expired - create new one
5. Ensure no extra spaces in `.pypirc`

### "Package already exists"

**Problem:** Version already published

**Solutions:**
1. Bump version in `pyproject.toml`
2. Can't replace published versions (PyPI security)
3. Use TestPyPI for testing versions

### "Authentication failed"

**Problem:** Credentials not found

**Solutions:**
```bash
# Check .pypirc exists
ls ~/.pypirc

# Check format
cat ~/.pypirc

# Check permissions
chmod 600 ~/.pypirc

# Try with environment variables instead
export TWINE_USERNAME=__token__
export TWINE_PASSWORD=pypi-YOUR-TOKEN
```

### "Wheel not found"

**Problem:** Build artifacts missing

**Solutions:**
```bash
# Build first
cd python
./build_wheels.sh

# Check output
ls -la dist/

# Should see: neleus-0.1.0-*.whl
```

### GitHub Actions Failures

**Problem:** Workflow fails

**Solutions:**
1. Check Actions tab for error message
2. Verify secrets are set correctly
3. Check workflow syntax (YAML indentation)
4. Enable debug logging:
   ```yaml
   env:
     ACTIONS_RUNNER_DEBUG: true
     ACTIONS_STEP_DEBUG: true
   ```

---

## 📝 Quick Reference

### Common Commands

```bash
# Build locally
cd python && ./build_wheels.sh

# Publish to TestPyPI
./publish_wheels.sh  # Choose option 1

# Publish to PyPI
./publish_wheels.sh  # Choose option 2

# Manual publish
twine upload dist/*.whl

# Check wheel
twine check dist/*.whl

# Verify token works
twine upload --repository testpypi dist/*.whl --verbose
```

### Environment Variable Precedence

1. Command line: `twine upload --username ... --password ...`
2. Environment variables: `TWINE_USERNAME`, `TWINE_PASSWORD`
3. Config file: `~/.pypirc`

---

## 🔐 Security Best Practices

1. **Never commit credentials:**
   ```bash
   # Add to .gitignore
   .pypirc
   .env
   .env.*
   ```

2. **Use tokens, not passwords:**
   - Tokens can be revoked
   - Tokens have limited scope
   - Tokens can expire

3. **Restrict token scope:**
   - Use project-specific tokens when possible
   - Don't give "Entire account" access unless needed

4. **Rotate tokens regularly:**
   - Every 90 days for production
   - After team member leaves
   - If potentially compromised

5. **Use GitHub Environments:**
   - Require manual approval for production
   - Limit who can approve
   - Use wait timers

6. **Protect .pypirc:**
   ```bash
   chmod 600 ~/.pypirc
   ```

---

## ✅ Final Checklist

### Before First Release

- [ ] PyPI account created and verified
- [ ] 2FA enabled on PyPI
- [ ] API token generated (both PyPI and TestPyPI)
- [ ] Token added to GitHub secrets (`PYPI_TOKEN`)
- [ ] Local `~/.pypirc` configured
- [ ] `.pypirc` has correct permissions (600)
- [ ] `.pypirc` in `.gitignore`
- [ ] Test build runs: `./build_wheels.sh`
- [ ] Test publish to TestPyPI works
- [ ] Verify installation from TestPyPI
- [ ] GitHub Actions workflow tested (create draft release)

### For Each Release

- [ ] Version bumped in `pyproject.toml`
- [ ] Build script runs successfully
- [ ] Wheels pass security checks
- [ ] Test installation locally
- [ ] Follow `RELEASE_CHECKLIST.md`
- [ ] Publish to TestPyPI first
- [ ] Test from TestPyPI
- [ ] Publish to production
- [ ] Verify installation works

---

## 📞 Need Help?

- **PyPI Issues:** https://github.com/pypi/support
- **GitHub Actions:** https://docs.github.com/en/actions
- **Maturin Docs:** https://www.maturin.rs/
- **Twine Docs:** https://twine.readthedocs.io/

---

**Created:** 2026-02-13  
**Last Updated:** 2026-02-13  
**Version:** 1.0
