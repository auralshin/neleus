# v0.1.12

## What's Changed
* Improved Hyperliquid market search and resolution for simpler terminal usage
* Fixed HIP-3 market analysis fallback for dex-routed candle requests
* Fixed live L2 books for markets like `flx:GAS` by using the correct routed symbol
* Kept the packaging and PyPI metadata pipeline updates from the earlier 0.1.x releases

## Platform Support
This release includes pre-built wheels for:
- **Linux**: x86_64, aarch64 (Python 3.10, 3.11, 3.12)
- **macOS**: Intel (x86_64), Apple Silicon (aarch64) (Python 3.10, 3.11, 3.12, 3.13)
- **Windows**: x86_64 (Python 3.10, 3.11, 3.12, 3.13)

## Installation
```bash
pip install --upgrade neleus
```

## Full Changelog
https://github.com/auralshin/neleus/compare/v0.1.9...v0.1.12
