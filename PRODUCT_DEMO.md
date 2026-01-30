# Neleus Product Demo Guide

**Duration:** 3 minutes  
**Target:** Investors, developers, potential users

---

## 🎬 Demo Script Overview

| Time | Section | What to Show |
|------|---------|--------------|
| 0:00-0:30 | **Intro & Value Prop** | Problem statement, solution |
| 0:30-1:15 | **Core: Backtest** | Run a backtest, show results |
| 1:15-1:45 | **CLI Commands** | Manage strategies, view metrics |
| 1:45-2:30 | **Streamlit Dashboard** | Risk dashboard, portfolio, backtest UI |
| 2:30-3:00 | **Wrap-up** | Key differentiators, call to action |

---

## Pre-Demo Setup

### Terminal Setup (2 terminals ready)
```bash
# Terminal 1: Project root
cd /Users/auralshin/projects/neleus
source .venv/bin/activate

# Terminal 2: For running backtest
cd /Users/auralshin/projects/neleus
source .venv/bin/activate
```

### Start the Streamlit Dashboard
```bash
# Run before demo starts
cd /Users/auralshin/projects/neleus
source .venv/bin/activate
python -m neleus.cli.main ui
```

### Browser Setup
- Tab 1: `http://localhost:8501` (Streamlit Dashboard)

---

## 📺 Demo Script (What to Say & Do)

### Section 1: Intro (0:00 - 0:30)

**WHAT TO SAY:**
> "Neleus is a professional-grade trading framework that solves a critical problem: 
> writing a strategy once and running it unchanged from research to production.
> 
> Unlike other tools, Neleus has a high-performance Rust core for execution 
> with Python for strategy development - giving you both speed AND flexibility."

**WHAT TO SHOW:**
- Show the logo/README briefly
- Quick scroll through the architecture in docs

**KEY POINTS TO HIGHLIGHT:**
- 🔴 "One codebase, many modes" - backtest → paper → live
- 🔴 Rust core = deterministic, fast, reliable
- 🔴 Python layer = easy strategy development

---

### Section 2: Backtest Demo (0:30 - 1:15)

**TERMINAL COMMANDS:**
```bash
# Show CLI help
python -m neleus.cli.main --help

# Show strategy commands
python -m neleus.cli.main strategy list

# Run a backtest
cd examples
python momentum_backtest.py
```

**WHAT TO SAY:**
> "Let me show you a momentum strategy backtest. 
> This strategy uses a 20-bar lookback with stop-loss and take-profit risk management.
> 
> Notice how the strategy code is pure Python, but execution happens in the Rust core.
> Same strategy can run in paper trading or live - zero code changes."

**KEY OUTPUT TO HIGHLIGHT:**
- 🔴 Backtest results: Total Return, Sharpe Ratio, Max Drawdown
- 🔴 Trade count, win rate
- 🔴 Speed of execution (Rust performance)

**WHAT TO SHOW IN CODE** (briefly):
```python
# examples/momentum_backtest.py - Line 19-75
class MomentumStrategy(Strategy):
    def __init__(self, lookback=20, threshold=0.02, ...):
        # Risk management parameters
        self.use_stop_loss = True
        self.stop_loss_pct = 0.02  # 2% stop loss
```

**KEY POINTS:**
- 🔴 Risk management built-in (stop-loss, take-profit, trailing stops)
- 🔴 Clean Strategy API - `on_start()`, `on_bar()`, `on_stop()`
- 🔴 Works with multiple venues (Hyperliquid, Lighter)

---

### Section 3: CLI Commands (1:15 - 1:45)

**TERMINAL COMMANDS:**
```bash
# Show managed service commands
python -m neleus.cli.main agents --help
python -m neleus.cli.main metrics --help

# List deployed agents (demo data)
python -m neleus.cli.main agents list

# Show agent metrics
python -m neleus.cli.main metrics summary

# Send a signal (external AI integration)
python -m neleus.cli.main signals send -i ETH-PERP -t entry -d long -c 0.85 -s ml_model_v2
```

**WHAT TO SAY:**
> "Neleus isn't just for backtesting - it's a fully managed service for trading bots.
> 
> With our CLI, you can manage agents, monitor metrics, and integrate external AI signals.
> Think of it as CI/CD for trading - deploy, monitor, iterate."

**KEY POINTS TO HIGHLIGHT:**
- 🔴 Agent lifecycle: deploy, start, stop, pause, restart
- 🔴 External signal integration (AI/ML models, TradingView, custom sources)
- 🔴 Real-time metrics and P&L tracking

---

### Section 4: Streamlit Dashboard (1:45 - 2:30)

**BROWSER - Streamlit Dashboard (`http://localhost:8501`):**

**WHAT TO SHOW:**
1. **Overview Page** (landing page)
   - Total Return, Sharpe Ratio, Max Drawdown metrics
   - Equity curve chart
   - Quick action buttons
   - System status (Rust Core active)

2. **Risk Dashboard** (click "⚠️ Risk Dashboard" in sidebar)
   - Sharpe, Sortino, Calmar ratios
   - Drawdown analysis chart
   - Return distribution with VaR lines
   - Detailed risk metrics (VaR 95%/99%, CVaR)
   - Risk limits monitoring

3. **Portfolio Manager** (click "💼 Portfolio")
   - Asset allocation pie chart
   - Portfolio performance chart
   - Open positions table
   - P&L breakdown by strategy

4. **Backtest Runner** (click "🔬 Backtest")
   - Strategy selection from examples/
   - Configuration panel (coin, timeframe, capital)
   - Run backtest button → shows results
   - Equity curve and trade log

5. **Coming Soon Pages** (click "🚀 Live Trading" or "🤖 Agent Deploy")
   - Show the "Coming Soon" placeholder
   - List planned features

**WHAT TO SAY:**
> "The Streamlit dashboard gives you complete visibility into your trading operation.
> 
> The Risk Dashboard shows comprehensive risk metrics - Sharpe, Sortino, VaR, 
> maximum drawdown, and real-time risk limit monitoring.
> 
> The Portfolio Manager lets you see asset allocation, positions, and P&L breakdown.
> 
> And the Backtest Runner lets you run backtests directly from the UI - 
> select a strategy, configure parameters, and get results with one click.
> 
> Live trading and agent deployment are coming soon - the infrastructure is ready."

**KEY POINTS TO HIGHLIGHT:**
- 🔴 Professional risk metrics (Sharpe, Sortino, VaR, CVaR)
- 🔴 Interactive Plotly charts
- 🔴 Run backtests from the UI
- 🔴 Clear "Coming Soon" for future features

---

### Section 5: Wrap-up (2:30 - 3:00)

**WHAT TO SAY:**
> "To summarize, Neleus offers:
> 
> 1. **One Codebase** - Same strategy from research to production
> 2. **Rust Performance** - Microsecond execution, deterministic backtests
> 3. **Python Ergonomics** - Easy strategy development, familiar tools
> 4. **Managed Service** - CI/CD for trading bots, external signal integration
> 5. **Production Ready** - Risk management, monitoring, alerting
> 
> We support Hyperliquid and Lighter today, with more venues coming.
> 
> Neleus is open source - try it at github.com/auralshin/neleus"

**KEY DIFFERENTIATORS TO EMPHASIZE:**
- 🔴 Not just backtesting - full trading infrastructure
- 🔴 External AI/signal integration (unique feature)
- 🔴 Rust + Python hybrid architecture
- 🔴 Professional risk management

---

## 🎯 Key Messages to Reinforce

| Message | Evidence |
|---------|----------|
| **Performance** | Rust core, microsecond execution |
| **Reliability** | Deterministic backtests, reproducible results |
| **Flexibility** | Python strategies, multiple venues |
| **Production-Ready** | Risk management, monitoring, alerts |
| **Unique** | External signal integration, managed service |

---

## ⚠️ Demo Tips

1. **Have terminals pre-configured** - Don't type long commands live
2. **Pre-start the UI server** - Dashboard should be ready
3. **Use the demo data** - It shows realistic P&L and metrics
4. **Highlight the fast execution** - Mention "Rust core" when running backtest
5. **Keep code scrolling brief** - Show structure, not every line

---

## 📁 Key Files to Show During Demo

| File | Purpose | When to Show |
|------|---------|--------------|
| [examples/momentum_backtest.py](examples/momentum_backtest.py) | Strategy example | Section 2 - Line 19-75 |
| [python/neleus/cli/main.py](python/neleus/cli/main.py) | CLI structure | Section 3 - Just mention |
| [python/neleus/ui/streamlit_app.py](python/neleus/ui/streamlit_app.py) | Streamlit dashboard | Section 4 - Just mention |
| [crates/core-engine/src/lib.rs](crates/core-engine/src/lib.rs) | Rust core | Section 1 - Briefly |

---

## 🔧 Backup Commands (If Something Breaks)

```bash
# Rebuild Python package if imports fail
cd crates/pybridge && maturin develop --release && cd ../..

# Kill stuck server
pkill -f "uvicorn"

# Restart UI
.venv/bin/python -m neleus.cli.main ui

# Test imports
python -c "from neleus import Strategy; print('OK')"
```

---

## 📊 Architecture Diagram (for slides)

```
┌─────────────────────────────────────────────────────────────────┐
│                         NELEUS                                   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   Python Layer                            │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │  │
│  │  │ Strategies  │  │  CLI Tools  │  │  Web Dashboard  │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   Rust Core                               │  │
│  │  ┌───────────┐  ┌───────────┐  ┌─────────────────────┐  │  │
│  │  │  Engine   │  │ Backtest  │  │   Venue Adapters    │  │  │
│  │  │ (Orders,  │  │  (Replay, │  │ (Hyperliquid,       │  │  │
│  │  │ Positions)│  │   Sim)    │  │  Lighter, ...)      │  │  │
│  │  └───────────┘  └───────────┘  └─────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Managed Service Layer                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │  │
│  │  │ Orchestrator│  │ Signal Hub  │  │ Agent Monitor   │  │  │
│  │  │ (Deploy/    │  │ (AI/Webhook │  │ (Metrics/       │  │  │
│  │  │  Lifecycle) │  │  Signals)   │  │  Alerts)        │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

##  Pre-Demo Checklist

- [ ] Virtual environment activated
- [ ] UI server running (`neleus ui`)
- [ ] Browser tabs open (/, /managed)
- [ ] Terminal history cleared
- [ ] Example backtest tested
- [ ] Demo data showing correctly
- [ ] Screen recording ready

---

**Good luck with your demo! 🚀**
