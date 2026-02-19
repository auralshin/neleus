
# ENVIRONMENT SETUP - VISUAL GUIDE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## WHERE EVERYTHING GOES

┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                          🏠 LOCAL MACHINE                              ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

📁 ~/.pypirc (Home Directory)
├─ Location: /Users/yourname/.pypirc  (or C:\Users\yourname\.pypirc on Windows)
├─ Purpose: PyPI authentication credentials
├─ Permissions: 600 (read/write for you only)
└─ Contents:
   ```
   [pypi]
   username = __token__
   password = pypi-AgEIcHlwaS5vcmcCJ...  ← Your actual token from PyPI
   ```

📁 Project: /Users/auralshin/projects/neleus/python/
├─ build_wheels.sh          ← Run to build
├─ publish_wheels.sh        ← Run to publish
├─ check_environment.sh     ← Run to verify setup
├─ .pypirc.template         ← Template (copy to ~/)
└─ dist/                    ← Build output (do NOT commit!)

System Tools:
├─ Python 3.10+             ← python --version
├─ Rust                     ← rustc --version
├─ Maturin                  ← pip install maturin
└─ Twine                    ← pip install twine


┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                       🌐 PYPI.ORG (Website)                            ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

https://pypi.org
├─ Account Settings
│  ├─ Enable 2FA ✓
│  └─ API tokens
│     └─ Create new token
│        ├─ Name: "neleus-github-actions"
│        ├─ Scope: "Entire account" or "Project: neleus"
│        └─ Result: pypi-AgEIcHlwaS5vcmcCJ...
│           ├─ Copy this ← You'll need it!
│           ├─ Paste in ~/.pypirc (local)
│           └─ Paste in GitHub secrets
│
└─ Your Packages
   └─ neleus ← Will appear after first publish


┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃                      🐙 GITHUB.COM (Repository)                        ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

https://github.com/YOUR_USERNAME/neleus
├─ Settings
│  └─ Secrets and variables
│     └─ Actions
│        └─ Repository secrets
│           ├─ PYPI_TOKEN ← Paste token here
│           │  Value: pypi-AgEIcHlwaS5vcmcCJ...
│           │
│           └─ TEST_PYPI_TOKEN (optional)
│              Value: pypi-AgEIcHlwaS5vcmcCJ...
│
├─ .github/workflows/build-wheels.yml ← Already created ✓
│  Uses: ${{ secrets.PYPI_TOKEN }}
│
└─ Actions Tab
   └─ Build Binary Wheels ← Runs automatically on release
      └─ Manual trigger: "Run workflow" button

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## STEP-BY-STEP SETUP

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 1: Install Local Tools                                            │
└─────────────────────────────────────────────────────────────────────────┘

Terminal Commands:
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Python packages
pip install maturin twine

# Verify
./check_environment.sh
```

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 2: Get PyPI Token                                                 │
└─────────────────────────────────────────────────────────────────────────┘

Web Browser:
1. Go to: https://pypi.org/account/register/
2. Create account + verify email
3. Enable 2FA (required!)
4. Go to: https://pypi.org/manage/account/
5. Scroll to "API tokens"
6. Click "Add API token"
   Name: neleus-github-actions
   Scope: Entire account
7. Click "Add token"
8. Copy the token: pypi-AgEIcHlwaS5vcmcCJ...
   ⚠️ SAVE IT NOW! You can't see it again

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 3: Configure Local Machine                                        │
└─────────────────────────────────────────────────────────────────────────┘

Terminal Commands:
```bash
# Copy template
cp python/.pypirc.template ~/.pypirc

# Edit and add your token
nano ~/.pypirc
# Change: password = pypi-YOUR-PRODUCTION-TOKEN-HERE
# To:     password = pypi-AgEIcHlwaS5vcmcCJ... (your actual token)

# Save and set permissions
chmod 600 ~/.pypirc

# Verify
ls -la ~/.pypirc  # Should show: -rw-------
```

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 4: Configure GitHub                                               │
└─────────────────────────────────────────────────────────────────────────┘

Web Browser:
1. Go to: https://github.com/YOUR_USERNAME/neleus
2. Click: Settings tab
3. Click: Secrets and variables → Actions
4. Click: New repository secret
   Name: PYPI_TOKEN
   Value: pypi-AgEIcHlwaS5vcmcCJ... (paste your token)
5. Click: Add secret
6. Done! ✓

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 5: Test Everything                                                │
└─────────────────────────────────────────────────────────────────────────┘

Terminal Commands:
```bash
# Check setup
cd python
./check_environment.sh

# Build wheels
./build_wheels.sh

# Test publish to TestPyPI (safe!)
./publish_wheels.sh
# Choose option 1: TestPyPI

# If that works, try production
./publish_wheels.sh
# Choose option 2: PyPI
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## WHAT EACH FILE/SECRET DOES

┌──────────────────────┬──────────────────────────────────────────────────┐
│ Component            │ Purpose                                          │
├──────────────────────┼──────────────────────────────────────────────────┤
│ ~/.pypirc            │ Local: Credentials for `twine upload`           │
│ PYPI_TOKEN (GitHub)  │ GitHub Actions: Auto-publish on release         │
│ build_wheels.sh      │ Builds binary wheels + security checks          │
│ publish_wheels.sh    │ Interactive publishing tool                     │
│ check_environment.sh │ Verifies your setup is correct                  │
└──────────────────────┴──────────────────────────────────────────────────┘

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## SECURITY REMINDERS

✅ DO:
- Keep ~/.pypirc permissions at 600
- Use tokens (not passwords)
- Rotate tokens every 90 days
- Use project-scoped tokens after first release

❌ DON'T:
- Commit ~/.pypirc to git (already in .gitignore)
- Share your tokens
- Use "Entire account" scope long-term
- Commit dist/ directory

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## TESTING YOUR SETUP

Run this single command:
```bash
cd python && ./check_environment.sh
```

Expected output when ready:
```
✓ Python 3.10+
✓ Rust installed
✓ Cargo installed
✓ Maturin installed
✓ Twine installed
✓ ~/.pypirc exists with correct permissions (600)
✓ .pypirc is ignored
✓ dist/ is ignored
✓ All scripts executable
✅ All checks passed! Your environment is ready.
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## QUICK COMMANDS

Local Build:
```bash
cd python
./build_wheels.sh
```

Local Publish:
```bash
./publish_wheels.sh  # Interactive menu
```

GitHub Actions:
```bash
# Option 1: Create a release on GitHub
# Option 2: Actions → Build Binary Wheels → Run workflow
```

Check Status:
```bash
./check_environment.sh     # Local setup
./QUICK_REFERENCE.sh       # All commands
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## NEED MORE HELP?

📖 Detailed Guides:
- ENVIRONMENT_SETUP.md      ← Complete environment guide (this is comprehensive!)
- ENV_QUICK_REFERENCE.md    ← Quick lookup reference
- BINARY_DISTRIBUTION.md    ← Full distribution guide
- RELEASE_CHECKLIST.md      ← Pre-release security checklist
- SETUP_SUMMARY.md          ← What was set up and why

🔧 Tools:
- ./check_environment.sh    ← Diagnose setup issues
- ./QUICK_REFERENCE.sh      ← Show all commands
- ./build_wheels.sh         ← Build with security checks
- ./publish_wheels.sh       ← Interactive publisher

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Created: 2026-02-13
Version: 1.0
