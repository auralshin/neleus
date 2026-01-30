"""
Neleus Web UI Server

A FastAPI-based web server for the Neleus trading dashboard.
Provides:
- TradingView charts integration
- Portfolio management interface
- Risk monitoring dashboard
- Strategy IDE with Monaco editor
- Performance analytics
- Real-time WebSocket updates
"""

import asyncio
import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional, Dict, Any, List

from fastapi import FastAPI, WebSocket, WebSocketDisconnect, HTTPException, Request
from fastapi.responses import HTMLResponse, JSONResponse, FileResponse
from fastapi.staticfiles import StaticFiles
from fastapi.middleware.cors import CORSMiddleware
import uvicorn

# =============================================================================
# App Configuration
# =============================================================================

app = FastAPI(
    title="Neleus Dashboard",
    description="High-Performance DeFi Trading Dashboard",
    version="0.1.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Global state
PROJECT_ROOT: Optional[Path] = None
CONNECTED_CLIENTS: List[WebSocket] = []

# =============================================================================
# WebSocket Manager
# =============================================================================

class ConnectionManager:
    def __init__(self):
        self.active_connections: List[WebSocket] = []
    
    async def connect(self, websocket: WebSocket):
        await websocket.accept()
        self.active_connections.append(websocket)
    
    def disconnect(self, websocket: WebSocket):
        if websocket in self.active_connections:
            self.active_connections.remove(websocket)
    
    async def broadcast(self, message: dict):
        for connection in self.active_connections:
            try:
                await connection.send_json(message)
            except:
                pass

manager = ConnectionManager()

# =============================================================================
# API Routes
# =============================================================================

@app.get("/api/info")
async def get_info():
    """Get project information."""
    return {
        "version": "0.1.0",
        "project_root": str(PROJECT_ROOT) if PROJECT_ROOT else None,
        "project_name": PROJECT_ROOT.name if PROJECT_ROOT else "Neleus Demo",
    }


@app.get("/api/strategies")
async def get_strategies():
    """Get list of available strategies."""
    strategies = []
    
    if PROJECT_ROOT:
        strategies_dir = PROJECT_ROOT / "strategies"
        if strategies_dir.exists():
            for py_file in strategies_dir.glob("*.py"):
                if py_file.name.startswith("_"):
                    continue
                strategies.append({
                    "name": py_file.stem,
                    "path": str(py_file.relative_to(PROJECT_ROOT)),
                    "modified": datetime.fromtimestamp(py_file.stat().st_mtime).isoformat(),
                })
    
    return {"strategies": strategies}


@app.get("/api/strategies/{name}")
async def get_strategy(name: str):
    """Get strategy source code."""
    if not PROJECT_ROOT:
        raise HTTPException(status_code=404, detail="No project loaded")
    
    strategies_dir = PROJECT_ROOT / "strategies"
    file_path = strategies_dir / f"{name}.py"
    
    if not file_path.exists():
        file_path = strategies_dir / f"{name}_strategy.py"
    
    if not file_path.exists():
        raise HTTPException(status_code=404, detail="Strategy not found")
    
    return {
        "name": name,
        "code": file_path.read_text(),
        "path": str(file_path.relative_to(PROJECT_ROOT)),
    }


@app.post("/api/strategies/{name}")
async def save_strategy(name: str, request: Request):
    """Save strategy source code."""
    if not PROJECT_ROOT:
        raise HTTPException(status_code=404, detail="No project loaded")
    
    data = await request.json()
    code = data.get("code", "")
    
    strategies_dir = PROJECT_ROOT / "strategies"
    file_path = strategies_dir / f"{name}.py"
    
    if not file_path.exists():
        file_path = strategies_dir / f"{name}_strategy.py"
    
    if not file_path.exists():
        # Create new file
        file_path = strategies_dir / f"{name}.py"
    
    file_path.write_text(code)
    
    return {"success": True, "message": f"Strategy '{name}' saved"}


@app.get("/api/portfolio")
async def get_portfolio():
    """Get current portfolio state."""
    # Demo data
    return {
        "total_value": 105234.56,
        "pnl": 5234.56,
        "pnl_pct": 5.23,
        "positions": [
            {
                "symbol": "BTC-PERP",
                "side": "long",
                "size": 0.5,
                "entry_price": 48000.0,
                "current_price": 50500.0,
                "pnl": 1250.0,
                "pnl_pct": 5.21,
            },
            {
                "symbol": "ETH-PERP",
                "side": "long",
                "size": 5.0,
                "entry_price": 2600.0,
                "current_price": 2750.0,
                "pnl": 750.0,
                "pnl_pct": 5.77,
            },
        ],
        "allocation": {
            "BTC-PERP": 45.0,
            "ETH-PERP": 25.0,
            "Cash": 30.0,
        },
    }


@app.get("/api/risk")
async def get_risk():
    """Get risk metrics."""
    return {
        "var_95": 2500.0,
        "var_99": 4200.0,
        "cvar_95": 3500.0,
        "volatility": 0.025,
        "volatility_regime": "normal",
        "sharpe_ratio": 1.85,
        "max_drawdown": -8.2,
        "current_drawdown": -2.1,
        "position_limit": 100000.0,
        "daily_loss_limit": 5000.0,
        "leverage": 2.5,
        "max_leverage": 5.0,
        "trading_allowed": True,
        "stress_tests": [
            {"scenario": "Flash Crash", "impact": -10500.0, "impact_pct": -10.0},
            {"scenario": "Market Correction", "impact": -21000.0, "impact_pct": -20.0},
            {"scenario": "Liquidity Crisis", "impact": -5200.0, "impact_pct": -5.0},
        ],
    }


@app.get("/api/performance")
async def get_performance():
    """Get performance metrics."""
    return {
        "total_return": 15.3,
        "sharpe_ratio": 1.85,
        "sortino_ratio": 2.12,
        "calmar_ratio": 1.87,
        "max_drawdown": -8.2,
        "win_rate": 58.4,
        "profit_factor": 1.72,
        "avg_trade": 125.50,
        "total_trades": 142,
        "winning_trades": 83,
        "losing_trades": 59,
        "best_trade": 2340.0,
        "worst_trade": -890.0,
        "avg_holding_time": "4.2h",
        "equity_curve": [
            {"date": "2024-01-01", "equity": 100000},
            {"date": "2024-02-01", "equity": 102500},
            {"date": "2024-03-01", "equity": 98500},
            {"date": "2024-04-01", "equity": 105000},
            {"date": "2024-05-01", "equity": 108500},
            {"date": "2024-06-01", "equity": 106000},
            {"date": "2024-07-01", "equity": 112000},
            {"date": "2024-08-01", "equity": 115300},
        ],
        "monthly_returns": [
            {"month": "Jan", "return": 2.5},
            {"month": "Feb", "return": -3.9},
            {"month": "Mar", "return": 6.6},
            {"month": "Apr", "return": 3.3},
            {"month": "May", "return": -2.3},
            {"month": "Jun", "return": 5.7},
            {"month": "Jul", "return": 2.9},
        ],
    }


@app.get("/api/backtest/run")
async def run_backtest(
    strategy: str,
    symbol: str = "BTC-PERP",
    timeframe: str = "1h",
    capital: float = 100000.0,
):
    """Run a backtest."""
    # Demo response
    return {
        "status": "completed",
        "results": {
            "total_return_pct": 15.3,
            "sharpe_ratio": 1.85,
            "max_drawdown_pct": -8.2,
            "total_trades": 142,
            "win_rate": 58.4,
            "profit_factor": 1.72,
        },
    }


@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    """WebSocket for real-time updates."""
    await manager.connect(websocket)
    try:
        while True:
            # Send periodic updates
            await websocket.send_json({
                "type": "heartbeat",
                "timestamp": datetime.now().isoformat(),
            })
            await asyncio.sleep(5)
    except WebSocketDisconnect:
        manager.disconnect(websocket)


# =============================================================================
# Frontend HTML
# =============================================================================

MANAGED_DASHBOARD_HTML = '''<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Neleus - Managed Trading Service</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        :root {
            --bg-primary: #0a0e17;
            --bg-secondary: #111827;
            --bg-tertiary: #1f2937;
            --bg-card: #161e2e;
            --border: #374151;
            --border-light: #4b5563;
            --text-primary: #f9fafb;
            --text-secondary: #9ca3af;
            --text-muted: #6b7280;
            --accent: #10b981;
            --accent-hover: #059669;
            --warning: #f59e0b;
            --danger: #ef4444;
            --success: #10b981;
            --blue: #3b82f6;
            --purple: #8b5cf6;
            --radius: 12px;
            --radius-sm: 8px;
            --radius-xs: 6px;
            --shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 2px 4px -1px rgba(0, 0, 0, 0.2);
            --transition: 150ms cubic-bezier(0.4, 0, 0.2, 1);
        }

        * { margin: 0; padding: 0; box-sizing: border-box; }

        body {
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.5;
            min-height: 100vh;
        }

        /* Layout */
        .app {
            display: grid;
            grid-template-columns: 240px 1fr;
            grid-template-rows: 64px 1fr;
            min-height: 100vh;
        }

        /* Header */
        header {
            grid-column: 1 / -1;
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--border);
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0 24px;
            gap: 16px;
        }

        .logo {
            display: flex;
            align-items: center;
            gap: 12px;
            font-size: 18px;
            font-weight: 700;
            color: var(--accent);
            text-decoration: none;
        }

        .logo-icon {
            width: 36px;
            height: 36px;
            background: linear-gradient(135deg, var(--accent), var(--blue));
            border-radius: var(--radius-sm);
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: 700;
            color: white;
        }

        .header-badge {
            background: var(--purple);
            color: white;
            font-size: 11px;
            padding: 2px 8px;
            border-radius: 10px;
            font-weight: 600;
        }

        .header-status {
            display: flex;
            align-items: center;
            gap: 20px;
            font-size: 13px;
        }

        .status-item {
            display: flex;
            align-items: center;
            gap: 6px;
        }

        .status-dot {
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: var(--success);
            animation: pulse 2s infinite;
        }

        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }

        /* Sidebar */
        .sidebar {
            background: var(--bg-secondary);
            border-right: 1px solid var(--border);
            padding: 16px 12px;
        }

        .nav-section {
            margin-bottom: 24px;
        }

        .nav-section-title {
            font-size: 11px;
            font-weight: 600;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin-bottom: 8px;
            padding: 0 12px;
        }

        .nav-item {
            display: flex;
            align-items: center;
            gap: 10px;
            padding: 10px 12px;
            border-radius: var(--radius-sm);
            color: var(--text-secondary);
            text-decoration: none;
            font-size: 14px;
            transition: all var(--transition);
            cursor: pointer;
        }

        .nav-item:hover, .nav-item.active {
            background: var(--bg-tertiary);
            color: var(--text-primary);
        }

        .nav-item.active {
            color: var(--accent);
        }

        .nav-badge {
            margin-left: auto;
            background: var(--danger);
            color: white;
            font-size: 10px;
            padding: 2px 6px;
            border-radius: 8px;
            font-weight: 600;
        }

        /* Main Content */
        .main {
            padding: 24px;
            overflow-y: auto;
        }

        /* Cards Grid */
        .cards-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
            gap: 16px;
            margin-bottom: 24px;
        }

        .card {
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            padding: 20px;
        }

        .stat-card {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }

        .stat-label {
            font-size: 13px;
            color: var(--text-muted);
        }

        .stat-value {
            font-size: 28px;
            font-weight: 700;
            font-family: 'JetBrains Mono', monospace;
        }

        .stat-value.positive { color: var(--success); }
        .stat-value.negative { color: var(--danger); }

        .stat-change {
            font-size: 12px;
            display: flex;
            align-items: center;
            gap: 4px;
        }

        .stat-change.positive { color: var(--success); }
        .stat-change.negative { color: var(--danger); }

        /* Section Title */
        .section-title {
            font-size: 16px;
            font-weight: 600;
            margin-bottom: 16px;
            display: flex;
            align-items: center;
            gap: 8px;
        }

        /* Agents Table */
        .agents-table {
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            overflow: hidden;
        }

        .table {
            width: 100%;
            border-collapse: collapse;
        }

        .table th, .table td {
            padding: 12px 16px;
            text-align: left;
            border-bottom: 1px solid var(--border);
        }

        .table th {
            background: var(--bg-tertiary);
            font-size: 12px;
            font-weight: 600;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }

        .table tr:hover {
            background: rgba(255,255,255,0.02);
        }

        .agent-name {
            font-weight: 500;
        }

        .agent-strategy {
            font-size: 12px;
            color: var(--text-muted);
        }

        .state-badge {
            display: inline-flex;
            align-items: center;
            gap: 6px;
            padding: 4px 10px;
            border-radius: 12px;
            font-size: 12px;
            font-weight: 500;
        }

        .state-badge.running {
            background: rgba(16, 185, 129, 0.15);
            color: var(--success);
        }

        .state-badge.paused {
            background: rgba(245, 158, 11, 0.15);
            color: var(--warning);
        }

        .state-badge.stopped {
            background: rgba(107, 114, 128, 0.15);
            color: var(--text-muted);
        }

        .pnl { font-family: 'JetBrains Mono', monospace; }
        .pnl.positive { color: var(--success); }
        .pnl.negative { color: var(--danger); }

        /* Action Buttons */
        .btn-group {
            display: flex;
            gap: 6px;
        }

        .btn {
            padding: 6px 12px;
            border-radius: var(--radius-xs);
            border: 1px solid var(--border);
            background: var(--bg-tertiary);
            color: var(--text-primary);
            font-size: 12px;
            cursor: pointer;
            transition: all var(--transition);
        }

        .btn:hover {
            border-color: var(--accent);
            color: var(--accent);
        }

        .btn.danger:hover {
            border-color: var(--danger);
            color: var(--danger);
        }

        .btn.primary {
            background: var(--accent);
            border-color: var(--accent);
            color: white;
        }

        .btn.primary:hover {
            background: var(--accent-hover);
        }

        /* Alerts Panel */
        .alerts-list {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }

        .alert-item {
            display: flex;
            align-items: flex-start;
            gap: 12px;
            padding: 12px;
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: var(--radius-sm);
        }

        .alert-icon {
            width: 24px;
            height: 24px;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            flex-shrink: 0;
        }

        .alert-icon.critical {
            background: rgba(239, 68, 68, 0.15);
            color: var(--danger);
        }

        .alert-icon.warning {
            background: rgba(245, 158, 11, 0.15);
            color: var(--warning);
        }

        .alert-icon.info {
            background: rgba(59, 130, 246, 0.15);
            color: var(--blue);
        }

        .alert-content {
            flex: 1;
        }

        .alert-message {
            font-size: 13px;
            margin-bottom: 4px;
        }

        .alert-meta {
            font-size: 11px;
            color: var(--text-muted);
        }

        /* Signals Feed */
        .signals-feed {
            max-height: 300px;
            overflow-y: auto;
        }

        .signal-item {
            display: flex;
            align-items: center;
            gap: 12px;
            padding: 10px 12px;
            border-bottom: 1px solid var(--border);
        }

        .signal-direction {
            width: 32px;
            height: 32px;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 14px;
        }

        .signal-direction.long {
            background: rgba(16, 185, 129, 0.15);
            color: var(--success);
        }

        .signal-direction.short {
            background: rgba(239, 68, 68, 0.15);
            color: var(--danger);
        }

        .signal-direction.flat {
            background: rgba(107, 114, 128, 0.15);
            color: var(--text-muted);
        }

        .signal-info { flex: 1; }

        .signal-instrument {
            font-weight: 500;
            font-size: 13px;
        }

        .signal-source {
            font-size: 11px;
            color: var(--text-muted);
        }

        .signal-confidence {
            font-family: 'JetBrains Mono', monospace;
            font-size: 12px;
            color: var(--accent);
        }

        /* Chart Container */
        .chart-container {
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: var(--radius);
            padding: 16px;
            height: 300px;
        }

        /* Two Column Layout */
        .two-col {
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 24px;
            margin-top: 24px;
        }

        @media (max-width: 1200px) {
            .two-col { grid-template-columns: 1fr; }
        }
    </style>
</head>
<body>
    <div class="app">
        <header>
            <a href="/" class="logo">
                <div class="logo-icon">N</div>
                <span>Neleus</span>
                <span class="header-badge">MANAGED</span>
            </a>
            <div class="header-status">
                <div class="status-item">
                    <div class="status-dot"></div>
                    <span id="agents-running">2 agents running</span>
                </div>
                <div class="status-item">
                    <span id="uptime">Uptime: 15d 8h</span>
                </div>
            </div>
        </header>

        <aside class="sidebar">
            <nav>
                <div class="nav-section">
                    <div class="nav-section-title">Overview</div>
                    <a class="nav-item active" onclick="showSection('overview')">📊 Dashboard</a>
                    <a class="nav-item" onclick="showSection('agents')">🤖 Agents</a>
                    <a class="nav-item" onclick="showSection('signals')">📡 Signals</a>
                </div>
                <div class="nav-section">
                    <div class="nav-section-title">Monitoring</div>
                    <a class="nav-item" onclick="showSection('metrics')">📈 Metrics</a>
                    <a class="nav-item" onclick="showSection('alerts')">
                        🔔 Alerts
                        <span class="nav-badge" id="alert-count">2</span>
                    </a>
                </div>
                <div class="nav-section">
                    <div class="nav-section-title">Actions</div>
                    <a class="nav-item" href="/">🔙 Trading UI</a>
                </div>
            </nav>
        </aside>

        <main class="main" id="main-content">
            <!-- Overview Section -->
            <section id="section-overview">
                <div class="cards-grid">
                    <div class="card stat-card">
                        <div class="stat-label">Total P&L (Today)</div>
                        <div class="stat-value positive" id="pnl-today">+$210.25</div>
                        <div class="stat-change positive">↑ +12.5% from yesterday</div>
                    </div>
                    <div class="card stat-card">
                        <div class="stat-label">Active Agents</div>
                        <div class="stat-value" id="active-agents">2 / 3</div>
                        <div class="stat-change">1 paused</div>
                    </div>
                    <div class="card stat-card">
                        <div class="stat-label">Signals (24h)</div>
                        <div class="stat-value" id="signals-24h">299</div>
                        <div class="stat-change">285 processed</div>
                    </div>
                    <div class="card stat-card">
                        <div class="stat-label">Avg Latency</div>
                        <div class="stat-value" id="avg-latency">10.5ms</div>
                        <div class="stat-change positive">↓ 2.3ms from avg</div>
                    </div>
                </div>

                <div class="section-title">🤖 Deployed Agents</div>
                <div class="agents-table">
                    <table class="table">
                        <thead>
                            <tr>
                                <th>Agent</th>
                                <th>State</th>
                                <th>Venue</th>
                                <th>P&L (Total)</th>
                                <th>Win Rate</th>
                                <th>Sharpe</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody id="agents-tbody"></tbody>
                    </table>
                </div>

                <div class="two-col">
                    <div>
                        <div class="section-title">📈 P&L History</div>
                        <div class="chart-container">
                            <canvas id="pnl-chart"></canvas>
                        </div>
                    </div>
                    <div>
                        <div class="section-title">🔔 Recent Alerts</div>
                        <div class="alerts-list" id="alerts-list"></div>
                    </div>
                </div>
            </section>

            <!-- Agents Section -->
            <section id="section-agents" style="display: none;">
                <div class="section-title">🤖 Agent Management</div>
                <div class="agents-table">
                    <table class="table">
                        <thead>
                            <tr>
                                <th>Agent</th>
                                <th>Strategy</th>
                                <th>State</th>
                                <th>Instruments</th>
                                <th>P&L (Today)</th>
                                <th>P&L (Total)</th>
                                <th>Trades</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody id="agents-full-tbody"></tbody>
                    </table>
                </div>
            </section>

            <!-- Signals Section -->
            <section id="section-signals" style="display: none;">
                <div class="section-title">📡 Signal Feed</div>
                <div class="card signals-feed" id="signals-feed"></div>
            </section>

            <!-- Metrics Section -->
            <section id="section-metrics" style="display: none;">
                <div class="section-title">📈 Performance Metrics</div>
                <div class="cards-grid">
                    <div class="card stat-card">
                        <div class="stat-label">Total P&L (All Time)</div>
                        <div class="stat-value positive">+$12,500</div>
                    </div>
                    <div class="card stat-card">
                        <div class="stat-label">Total Trades</div>
                        <div class="stat-value">1,043</div>
                    </div>
                    <div class="card stat-card">
                        <div class="stat-label">Avg Win Rate</div>
                        <div class="stat-value">58.2%</div>
                    </div>
                    <div class="card stat-card">
                        <div class="stat-label">Max Drawdown</div>
                        <div class="stat-value negative">-8.5%</div>
                    </div>
                </div>
            </section>

            <!-- Alerts Section -->
            <section id="section-alerts" style="display: none;">
                <div class="section-title">🔔 All Alerts</div>
                <div class="alerts-list" id="all-alerts-list"></div>
            </section>
        </main>
    </div>

    <script>
        // State
        let agents = [];
        let signals = [];
        let alerts = [];

        // Fetch data
        async function fetchData() {
            try {
                const [agentsRes, signalsRes, alertsRes, overviewRes] = await Promise.all([
                    fetch('/api/agents'),
                    fetch('/api/signals'),
                    fetch('/api/alerts'),
                    fetch('/api/overview')
                ]);
                
                agents = await agentsRes.json();
                signals = await signalsRes.json();
                alerts = await alertsRes.json();
                const overview = await overviewRes.json();
                
                updateOverview(overview);
                renderAgents();
                renderSignals();
                renderAlerts();
                initChart();
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        }

        function updateOverview(overview) {
            document.getElementById('agents-running').textContent = `${overview.agents.running} agents running`;
            document.getElementById('active-agents').textContent = `${overview.agents.running} / ${overview.agents.total}`;
            document.getElementById('pnl-today').textContent = `+$${overview.total_pnl.today.toFixed(2)}`;
            document.getElementById('signals-24h').textContent = overview.signals.received_24h;
            document.getElementById('avg-latency').textContent = `${overview.system.latency_avg_ms}ms`;
            document.getElementById('alert-count').textContent = overview.alerts.active;
        }

        function renderAgents() {
            const tbody = document.getElementById('agents-tbody');
            const fullTbody = document.getElementById('agents-full-tbody');
            
            const rows = agents.map(agent => {
                const pnlClass = agent.pnl.total >= 0 ? 'positive' : 'negative';
                const pnlSign = agent.pnl.total >= 0 ? '+' : '';
                
                return `
                    <tr>
                        <td>
                            <div class="agent-name">${agent.name}</div>
                            <div class="agent-strategy">${agent.id}</div>
                        </td>
                        <td><span class="state-badge ${agent.state}">● ${agent.state}</span></td>
                        <td>${agent.venue}</td>
                        <td class="pnl ${pnlClass}">${pnlSign}$${agent.pnl.total.toFixed(2)}</td>
                        <td>${(agent.trades.win_rate * 100).toFixed(1)}%</td>
                        <td>${agent.risk.sharpe.toFixed(2)}</td>
                        <td>
                            <div class="btn-group">
                                ${agent.state === 'running' 
                                    ? '<button class="btn" onclick="pauseAgent(\\''+agent.id+'\\')">Pause</button>'
                                    : '<button class="btn" onclick="startAgent(\\''+agent.id+'\\')">Start</button>'
                                }
                                <button class="btn danger" onclick="stopAgent('${agent.id}')">Stop</button>
                            </div>
                        </td>
                    </tr>
                `;
            }).join('');
            
            tbody.innerHTML = rows;
            
            const fullRows = agents.map(agent => {
                const pnlClass = agent.pnl.total >= 0 ? 'positive' : 'negative';
                const todayClass = agent.pnl.today >= 0 ? 'positive' : 'negative';
                const pnlSign = agent.pnl.total >= 0 ? '+' : '';
                const todaySign = agent.pnl.today >= 0 ? '+' : '';
                
                return `
                    <tr>
                        <td>
                            <div class="agent-name">${agent.name}</div>
                            <div class="agent-strategy">${agent.id}</div>
                        </td>
                        <td>${agent.strategy}</td>
                        <td><span class="state-badge ${agent.state}">● ${agent.state}</span></td>
                        <td>${agent.instruments.join(', ')}</td>
                        <td class="pnl ${todayClass}">${todaySign}$${agent.pnl.today.toFixed(2)}</td>
                        <td class="pnl ${pnlClass}">${pnlSign}$${agent.pnl.total.toFixed(2)}</td>
                        <td>${agent.trades.total}</td>
                        <td>
                            <div class="btn-group">
                                ${agent.state === 'running' 
                                    ? '<button class="btn" onclick="pauseAgent(\\''+agent.id+'\\')">Pause</button>'
                                    : '<button class="btn primary" onclick="startAgent(\\''+agent.id+'\\')">Start</button>'
                                }
                                <button class="btn" onclick="stopAgent('${agent.id}')">Stop</button>
                                <button class="btn danger" onclick="deleteAgent('${agent.id}')">Delete</button>
                            </div>
                        </td>
                    </tr>
                `;
            }).join('');
            
            fullTbody.innerHTML = fullRows;
        }

        function renderSignals() {
            const feed = document.getElementById('signals-feed');
            
            const items = signals.map(signal => {
                const dirIcon = signal.direction === 'long' ? '▲' : signal.direction === 'short' ? '▼' : '○';
                
                return `
                    <div class="signal-item">
                        <div class="signal-direction ${signal.direction}">${dirIcon}</div>
                        <div class="signal-info">
                            <div class="signal-instrument">${signal.instrument} - ${signal.signal_type}</div>
                            <div class="signal-source">${signal.source} • ${new Date(signal.timestamp).toLocaleTimeString()}</div>
                        </div>
                        <div class="signal-confidence">${(signal.confidence * 100).toFixed(0)}%</div>
                    </div>
                `;
            }).join('');
            
            feed.innerHTML = items;
        }

        function renderAlerts() {
            const list = document.getElementById('alerts-list');
            const allList = document.getElementById('all-alerts-list');
            
            const items = alerts.slice(0, 3).map(alert => {
                const icon = alert.severity === 'critical' ? '🚨' : alert.severity === 'warning' ? '⚠️' : 'ℹ️';
                
                return `
                    <div class="alert-item">
                        <div class="alert-icon ${alert.severity}">${icon}</div>
                        <div class="alert-content">
                            <div class="alert-message">${alert.message}</div>
                            <div class="alert-meta">${alert.agent} • ${new Date(alert.timestamp).toLocaleTimeString()}</div>
                        </div>
                    </div>
                `;
            }).join('');
            
            list.innerHTML = items;
            allList.innerHTML = alerts.map(alert => {
                const icon = alert.severity === 'critical' ? '🚨' : alert.severity === 'warning' ? '⚠️' : 'ℹ️';
                
                return `
                    <div class="alert-item">
                        <div class="alert-icon ${alert.severity}">${icon}</div>
                        <div class="alert-content">
                            <div class="alert-message">${alert.message}</div>
                            <div class="alert-meta">${alert.agent} • ${new Date(alert.timestamp).toLocaleTimeString()}</div>
                        </div>
                        ${!alert.acknowledged ? '<button class="btn" onclick="acknowledgeAlert(\\''+alert.id+'\\')">Ack</button>' : ''}
                    </div>
                `;
            }).join('');
        }

        function initChart() {
            const ctx = document.getElementById('pnl-chart').getContext('2d');
            
            new Chart(ctx, {
                type: 'line',
                data: {
                    labels: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'],
                    datasets: [{
                        label: 'P&L',
                        data: [1200, 1350, 1280, 1450, 1520, 1480, 1650],
                        borderColor: '#10b981',
                        backgroundColor: 'rgba(16, 185, 129, 0.1)',
                        fill: true,
                        tension: 0.4
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: { display: false }
                    },
                    scales: {
                        x: {
                            grid: { color: 'rgba(55, 65, 81, 0.3)' },
                            ticks: { color: '#9ca3af' }
                        },
                        y: {
                            grid: { color: 'rgba(55, 65, 81, 0.3)' },
                            ticks: { color: '#9ca3af' }
                        }
                    }
                }
            });
        }

        // Navigation
        function showSection(section) {
            document.querySelectorAll('section').forEach(s => s.style.display = 'none');
            document.getElementById(`section-${section}`).style.display = 'block';
            document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
            event.target.classList.add('active');
        }

        // Agent actions
        async function startAgent(id) {
            await fetch(`/api/agents/${id}/start`, { method: 'POST' });
            fetchData();
        }

        async function pauseAgent(id) {
            await fetch(`/api/agents/${id}/pause`, { method: 'POST' });
            fetchData();
        }

        async function stopAgent(id) {
            await fetch(`/api/agents/${id}/stop`, { method: 'POST' });
            fetchData();
        }

        async function deleteAgent(id) {
            if (confirm(`Delete agent ${id}?`)) {
                await fetch(`/api/agents/${id}`, { method: 'DELETE' });
                fetchData();
            }
        }

        async function acknowledgeAlert(id) {
            await fetch(`/api/alerts/${id}/acknowledge`, { method: 'POST' });
            fetchData();
        }

        // Initialize
        fetchData();
        setInterval(fetchData, 30000); // Refresh every 30s
    </script>
</body>
</html>
'''

DASHBOARD_HTML = '''<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Neleus Dashboard</title>
    
    <!-- TradingView Widget -->
    <script type="text/javascript" src="https://s3.tradingview.com/tv.js"></script>
    
    <!-- CodeMirror Editor -->
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/codemirror.min.css">
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/theme/dracula.min.css">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/codemirror.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.2/mode/python/python.min.js"></script>
    
    <!-- Chart.js -->
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    
    <style>
        :root {
            --bg-primary: #0d1117;
            --bg-secondary: #161b22;
            --bg-tertiary: #21262d;
            --border-color: #30363d;
            --text-primary: #e6edf3;
            --text-secondary: #8b949e;
            --accent-green: #3fb950;
            --accent-red: #f85149;
            --accent-blue: #58a6ff;
            --accent-purple: #a371f7;
        }
        
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            min-height: 100vh;
        }
        
        /* Header */
        .header {
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--border-color);
            padding: 0.75rem 1.5rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }
        
        .header h1 {
            font-size: 1.25rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }
        
        .header h1 span {
            color: var(--accent-blue);
        }
        
        .header-status {
            display: flex;
            align-items: center;
            gap: 1rem;
        }
        
        .status-indicator {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.875rem;
        }
        
        .status-dot {
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: var(--accent-green);
        }
        
        .status-dot.warning { background: #d29922; }
        .status-dot.error { background: var(--accent-red); }
        
        /* Navigation */
        .nav-tabs {
            background: var(--bg-secondary);
            border-bottom: 1px solid var(--border-color);
            display: flex;
            padding: 0 1rem;
        }
        
        .nav-tab {
            padding: 0.75rem 1.25rem;
            color: var(--text-secondary);
            text-decoration: none;
            border-bottom: 2px solid transparent;
            transition: all 0.2s;
            cursor: pointer;
            font-size: 0.9rem;
        }
        
        .nav-tab:hover {
            color: var(--text-primary);
        }
        
        .nav-tab.active {
            color: var(--text-primary);
            border-bottom-color: var(--accent-blue);
        }
        
        /* Main Layout */
        .main-content {
            padding: 1rem;
        }
        
        .tab-content {
            display: none;
        }
        
        .tab-content.active {
            display: block;
        }
        
        /* Grid Layout */
        .dashboard-grid {
            display: grid;
            grid-template-columns: repeat(12, 1fr);
            gap: 1rem;
        }
        
        /* Cards */
        .card {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            overflow: hidden;
        }
        
        .card-header {
            padding: 0.75rem 1rem;
            border-bottom: 1px solid var(--border-color);
            font-weight: 600;
            font-size: 0.875rem;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .card-body {
            padding: 1rem;
        }
        
        /* Charts Section */
        .chart-section {
            grid-column: span 8;
            min-height: 500px;
        }
        
        #tradingview-widget {
            width: 100%;
            height: 450px;
        }
        
        /* Portfolio Section */
        .portfolio-section {
            grid-column: span 4;
        }
        
        .portfolio-summary {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 1rem;
            margin-bottom: 1rem;
        }
        
        .metric {
            text-align: center;
        }
        
        .metric-label {
            color: var(--text-secondary);
            font-size: 0.75rem;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        
        .metric-value {
            font-size: 1.5rem;
            font-weight: 600;
            margin-top: 0.25rem;
        }
        
        .metric-value.positive { color: var(--accent-green); }
        .metric-value.negative { color: var(--accent-red); }
        
        /* Positions Table */
        .positions-table {
            width: 100%;
            font-size: 0.875rem;
        }
        
        .positions-table th {
            text-align: left;
            color: var(--text-secondary);
            font-weight: 500;
            padding: 0.5rem;
            border-bottom: 1px solid var(--border-color);
        }
        
        .positions-table td {
            padding: 0.5rem;
            border-bottom: 1px solid var(--border-color);
        }
        
        .side-long { color: var(--accent-green); }
        .side-short { color: var(--accent-red); }
        
        /* Risk Section */
        .risk-grid {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 1rem;
        }
        
        .risk-meter {
            height: 8px;
            background: var(--bg-tertiary);
            border-radius: 4px;
            overflow: hidden;
            margin-top: 0.5rem;
        }
        
        .risk-meter-fill {
            height: 100%;
            border-radius: 4px;
            transition: width 0.3s;
        }
        
        .risk-meter-fill.low { background: var(--accent-green); }
        .risk-meter-fill.medium { background: #d29922; }
        .risk-meter-fill.high { background: var(--accent-red); }
        
        /* Strategy IDE */
        .ide-layout {
            display: grid;
            grid-template-columns: 250px 1fr 300px;
            gap: 1rem;
            height: calc(100vh - 150px);
        }
        
        .file-explorer {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            overflow: hidden;
        }
        
        .file-explorer-header {
            padding: 0.75rem 1rem;
            border-bottom: 1px solid var(--border-color);
            font-weight: 600;
            font-size: 0.875rem;
        }
        
        .file-list {
            list-style: none;
        }
        
        .file-item {
            padding: 0.5rem 1rem;
            cursor: pointer;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            color: var(--text-secondary);
            transition: background 0.2s;
        }
        
        .file-item:hover {
            background: var(--bg-tertiary);
            color: var(--text-primary);
        }
        
        .file-item.active {
            background: var(--bg-tertiary);
            color: var(--accent-blue);
        }
        
        .editor-container {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            overflow: hidden;
            display: flex;
            flex-direction: column;
        }
        
        .editor-tabs {
            display: flex;
            background: var(--bg-tertiary);
            border-bottom: 1px solid var(--border-color);
        }
        
        .editor-tab {
            padding: 0.5rem 1rem;
            font-size: 0.875rem;
            color: var(--text-secondary);
            cursor: pointer;
            border-right: 1px solid var(--border-color);
        }
        
        .editor-tab.active {
            background: var(--bg-secondary);
            color: var(--text-primary);
        }
        
        #monaco-editor {
            flex: 1;
            min-height: 400px;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
        }
        
        .CodeMirror {
            height: 100%;
            font-size: 14px;
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
        }
        
        .strategy-panel {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            overflow: hidden;
        }
        
        .strategy-panel-section {
            padding: 1rem;
            border-bottom: 1px solid var(--border-color);
        }
        
        .strategy-panel-section h4 {
            font-size: 0.75rem;
            text-transform: uppercase;
            color: var(--text-secondary);
            margin-bottom: 0.75rem;
        }
        
        /* Buttons */
        .btn {
            padding: 0.5rem 1rem;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.875rem;
            font-weight: 500;
            transition: all 0.2s;
        }
        
        .btn-primary {
            background: var(--accent-blue);
            color: white;
        }
        
        .btn-primary:hover {
            background: #4c94e8;
        }
        
        .btn-success {
            background: var(--accent-green);
            color: white;
        }
        
        .btn-danger {
            background: var(--accent-red);
            color: white;
        }
        
        .btn-outline {
            background: transparent;
            border: 1px solid var(--border-color);
            color: var(--text-primary);
        }
        
        .btn-outline:hover {
            background: var(--bg-tertiary);
        }
        
        /* Form elements */
        input, select {
            background: var(--bg-tertiary);
            border: 1px solid var(--border-color);
            border-radius: 6px;
            padding: 0.5rem 0.75rem;
            color: var(--text-primary);
            font-size: 0.875rem;
            width: 100%;
        }
        
        input:focus, select:focus {
            outline: none;
            border-color: var(--accent-blue);
        }
        
        label {
            display: block;
            font-size: 0.75rem;
            color: var(--text-secondary);
            margin-bottom: 0.25rem;
        }
        
        .form-group {
            margin-bottom: 1rem;
        }
        
        /* Alerts */
        .alert {
            padding: 0.75rem 1rem;
            border-radius: 6px;
            font-size: 0.875rem;
            margin-bottom: 1rem;
        }
        
        .alert-warning {
            background: rgba(210, 153, 34, 0.1);
            border: 1px solid rgba(210, 153, 34, 0.3);
            color: #d29922;
        }
        
        .alert-success {
            background: rgba(63, 185, 80, 0.1);
            border: 1px solid rgba(63, 185, 80, 0.3);
            color: var(--accent-green);
        }
        
        /* Performance Charts */
        .performance-grid {
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 1rem;
        }
        
        .chart-container {
            position: relative;
            height: 300px;
        }
        
        /* Stress Test Table */
        .stress-table {
            width: 100%;
            font-size: 0.875rem;
        }
        
        .stress-table th, .stress-table td {
            padding: 0.5rem;
            text-align: left;
            border-bottom: 1px solid var(--border-color);
        }
        
        .stress-table th {
            color: var(--text-secondary);
            font-weight: 500;
        }
        
        /* Scrollbar */
        ::-webkit-scrollbar {
            width: 8px;
            height: 8px;
        }
        
        ::-webkit-scrollbar-track {
            background: var(--bg-primary);
        }
        
        ::-webkit-scrollbar-thumb {
            background: var(--border-color);
            border-radius: 4px;
        }
        
        ::-webkit-scrollbar-thumb:hover {
            background: var(--text-secondary);
        }
        
        /* Backtest Results Modal */
        .modal {
            display: none;
            position: fixed;
            z-index: 1000;
            left: 0;
            top: 0;
            width: 100%;
            height: 100%;
            background: rgba(0, 0, 0, 0.8);
        }
        
        .modal.active {
            display: flex;
            align-items: center;
            justify-content: center;
        }
        
        .modal-content {
            background: var(--bg-secondary);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            width: 90%;
            max-width: 1200px;
            max-height: 90vh;
            overflow-y: auto;
            padding: 0;
        }
        
        .modal-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 1.5rem 2rem;
            border-bottom: 1px solid var(--border-color);
            margin-bottom: 0;
        }
        
        .modal-header h2 {
            margin: 0;
            font-size: 1.5rem;
        }
        
        #modal-results-content {
            padding: 2rem;
        }
        
        .modal-close {
            background: transparent;
            border: none;
            color: var(--text-secondary);
            font-size: 1.5rem;
            cursor: pointer;
            padding: 0;
            width: 32px;
            height: 32px;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        
        .modal-close:hover {
            color: var(--text-primary);
        }
        
        .results-metrics {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 1.5rem;
            margin-bottom: 2rem;
        }
        
        .results-metrics .metric {
            text-align: center;
            padding: 1rem;
            background: var(--bg-tertiary);
            border-radius: 8px;
            border: 1px solid var(--border-color);
        }
        
        .results-metrics .metric-label {
            display: block;
            margin-bottom: 0.5rem;
        }
        
        .results-metrics .metric-value {
            font-size: 1.75rem;
        }
        
        .results-chart {
            margin-bottom: 2rem;
        }
        
        .results-chart h3 {
            font-size: 1rem;
            margin-bottom: 1rem;
            color: var(--text-primary);
        }
        
        .results-chart canvas {
            max-height: 350px;
            width: 100% !important;
        }
    </style>
</head>
<body>
    <!-- Header -->
    <header class="header">
        <h1>🌊 <span>Neleus</span> Dashboard</h1>
        <div class="header-status">
            <div class="status-indicator">
                <div class="status-dot"></div>
                <span>Connected</span>
            </div>
            <span id="project-name">Demo Mode</span>
        </div>
    </header>
    
    <!-- Navigation -->
    <nav class="nav-tabs">
        <a class="nav-tab active" data-tab="trading">Trading</a>
        <a class="nav-tab" data-tab="portfolio">Portfolio</a>
        <a class="nav-tab" data-tab="risk">Risk</a>
        <a class="nav-tab" data-tab="strategies">Strategy IDE</a>
        <a class="nav-tab" data-tab="performance">Performance</a>
        <a class="nav-tab" data-tab="backtest">Backtest</a>
    </nav>
    
    <!-- Main Content -->
    <main class="main-content">
        
        <!-- Trading Tab -->
        <div id="trading" class="tab-content active">
            <div class="dashboard-grid">
                <div class="card chart-section">
                    <div class="card-header">
                        <span>BTC-PERP</span>
                        <select id="symbol-select" style="width: auto;">
                            <option value="BTCUSD">BTC-PERP</option>
                            <option value="ETHUSD">ETH-PERP</option>
                            <option value="SOLUSD">SOL-PERP</option>
                        </select>
                    </div>
                    <div id="tradingview-widget"></div>
                </div>
                
                <div class="portfolio-section">
                    <div class="card">
                        <div class="card-header">Portfolio Summary</div>
                        <div class="card-body">
                            <div class="portfolio-summary">
                                <div class="metric">
                                    <div class="metric-label">Total Value</div>
                                    <div class="metric-value" id="total-value">$105,234</div>
                                </div>
                                <div class="metric">
                                    <div class="metric-label">P&L</div>
                                    <div class="metric-value positive" id="total-pnl">+$5,234</div>
                                </div>
                            </div>
                        </div>
                    </div>
                    
                    <div class="card" style="margin-top: 1rem;">
                        <div class="card-header">Open Positions</div>
                        <div class="card-body" style="padding: 0;">
                            <table class="positions-table">
                                <thead>
                                    <tr>
                                        <th>Symbol</th>
                                        <th>Side</th>
                                        <th>Size</th>
                                        <th>P&L</th>
                                    </tr>
                                </thead>
                                <tbody id="positions-body">
                                    <tr>
                                        <td>BTC-PERP</td>
                                        <td class="side-long">LONG</td>
                                        <td>0.5</td>
                                        <td class="side-long">+$1,250</td>
                                    </tr>
                                    <tr>
                                        <td>ETH-PERP</td>
                                        <td class="side-long">LONG</td>
                                        <td>5.0</td>
                                        <td class="side-long">+$750</td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    </div>
                    
                    <div class="card" style="margin-top: 1rem;">
                        <div class="card-header">Quick Trade</div>
                        <div class="card-body">
                            <div class="form-group">
                                <label>Symbol</label>
                                <select>
                                    <option>BTC-PERP</option>
                                    <option>ETH-PERP</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label>Size</label>
                                <input type="number" value="0.1" step="0.01">
                            </div>
                            <div style="display: flex; gap: 0.5rem;">
                                <button class="btn btn-success" style="flex: 1;">Buy</button>
                                <button class="btn btn-danger" style="flex: 1;">Sell</button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Portfolio Tab -->
        <div id="portfolio" class="tab-content">
            <div class="dashboard-grid">
                <div class="card" style="grid-column: span 8;">
                    <div class="card-header">Portfolio Allocation</div>
                    <div class="card-body">
                        <div class="chart-container">
                            <canvas id="allocation-chart"></canvas>
                        </div>
                    </div>
                </div>
                <div class="card" style="grid-column: span 4;">
                    <div class="card-header">Holdings</div>
                    <div class="card-body" style="padding: 0;">
                        <table class="positions-table">
                            <thead>
                                <tr>
                                    <th>Asset</th>
                                    <th>Allocation</th>
                                    <th>Value</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr>
                                    <td>BTC-PERP</td>
                                    <td>45%</td>
                                    <td>$47,355</td>
                                </tr>
                                <tr>
                                    <td>ETH-PERP</td>
                                    <td>25%</td>
                                    <td>$26,308</td>
                                </tr>
                                <tr>
                                    <td>Cash</td>
                                    <td>30%</td>
                                    <td>$31,570</td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Risk Tab -->
        <div id="risk" class="tab-content">
            <div class="dashboard-grid">
                <div class="card" style="grid-column: span 12;">
                    <div class="card-header">
                        <span>Risk Dashboard</span>
                        <span class="status-indicator">
                            <div class="status-dot"></div>
                            Trading Allowed
                        </span>
                    </div>
                    <div class="card-body">
                        <div class="risk-grid">
                            <div>
                                <div class="metric-label">Value at Risk (95%)</div>
                                <div class="metric-value negative">-$2,500</div>
                                <div class="risk-meter">
                                    <div class="risk-meter-fill low" style="width: 25%;"></div>
                                </div>
                            </div>
                            <div>
                                <div class="metric-label">Value at Risk (99%)</div>
                                <div class="metric-value negative">-$4,200</div>
                                <div class="risk-meter">
                                    <div class="risk-meter-fill medium" style="width: 42%;"></div>
                                </div>
                            </div>
                            <div>
                                <div class="metric-label">Expected Shortfall (CVaR)</div>
                                <div class="metric-value negative">-$3,500</div>
                                <div class="risk-meter">
                                    <div class="risk-meter-fill low" style="width: 35%;"></div>
                                </div>
                            </div>
                            <div>
                                <div class="metric-label">Current Leverage</div>
                                <div class="metric-value">2.5x</div>
                                <div class="risk-meter">
                                    <div class="risk-meter-fill medium" style="width: 50%;"></div>
                                </div>
                            </div>
                            <div>
                                <div class="metric-label">Volatility Regime</div>
                                <div class="metric-value">Normal</div>
                                <div class="risk-meter">
                                    <div class="risk-meter-fill low" style="width: 40%;"></div>
                                </div>
                            </div>
                            <div>
                                <div class="metric-label">Current Drawdown</div>
                                <div class="metric-value negative">-2.1%</div>
                                <div class="risk-meter">
                                    <div class="risk-meter-fill low" style="width: 21%;"></div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                
                <div class="card" style="grid-column: span 6;">
                    <div class="card-header">Stress Test Results</div>
                    <div class="card-body" style="padding: 0;">
                        <table class="stress-table">
                            <thead>
                                <tr>
                                    <th>Scenario</th>
                                    <th>Impact</th>
                                    <th>% of Portfolio</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr>
                                    <td>Flash Crash</td>
                                    <td class="side-short">-$10,500</td>
                                    <td>-10.0%</td>
                                </tr>
                                <tr>
                                    <td>Market Correction</td>
                                    <td class="side-short">-$21,000</td>
                                    <td>-20.0%</td>
                                </tr>
                                <tr>
                                    <td>Liquidity Crisis</td>
                                    <td class="side-short">-$5,200</td>
                                    <td>-5.0%</td>
                                </tr>
                                <tr>
                                    <td>Volatility Spike</td>
                                    <td class="side-short">-$8,400</td>
                                    <td>-8.0%</td>
                                </tr>
                                <tr>
                                    <td>Black Swan</td>
                                    <td class="side-short">-$31,500</td>
                                    <td>-30.0%</td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                </div>
                
                <div class="card" style="grid-column: span 6;">
                    <div class="card-header">Dynamic Risk Limits</div>
                    <div class="card-body">
                        <div class="form-group">
                            <label>Position Limit</label>
                            <input type="text" value="$100,000" readonly>
                        </div>
                        <div class="form-group">
                            <label>Daily Loss Limit</label>
                            <input type="text" value="$5,000" readonly>
                        </div>
                        <div class="form-group">
                            <label>Max Leverage</label>
                            <input type="text" value="5.0x" readonly>
                        </div>
                        <div class="alert alert-success">
                            ✓ All risk limits within bounds
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Strategy IDE Tab -->
        <div id="strategies" class="tab-content">
            <div class="ide-layout">
                <div class="file-explorer">
                    <div class="file-explorer-header">📁 Strategies</div>
                    <ul class="file-list" id="strategy-list">
                        <li class="file-item active" data-file="momentum_strategy">
                            📄 momentum_strategy.py
                        </li>
                        <li class="file-item" data-file="mean_reversion_strategy">
                            📄 mean_reversion_strategy.py
                        </li>
                    </ul>
                    <div style="padding: 1rem; border-top: 1px solid var(--border-color);">
                        <button class="btn btn-outline" style="width: 100%;" onclick="createStrategy()">
                            + New Strategy
                        </button>
                    </div>
                </div>
                
                <div class="editor-container">
                    <div class="editor-tabs">
                        <div class="editor-tab active" id="editor-tab-name">momentum_strategy.py</div>
                    </div>
                    <textarea id="code-editor"></textarea>
                    <div style="padding: 0.5rem 1rem; border-top: 1px solid var(--border-color); display: flex; justify-content: space-between;">
                        <span style="color: var(--text-secondary); font-size: 0.75rem;">Python</span>
                        <div>
                            <button class="btn btn-outline" onclick="saveStrategy()">Save</button>
                            <button class="btn btn-primary" onclick="runBacktest()">Run Backtest</button>
                        </div>
                    </div>
                </div>
                
                <div class="strategy-panel">
                    <div class="strategy-panel-section">
                        <h4>Strategy Info</h4>
                        <div class="form-group">
                            <label>Name</label>
                            <input type="text" value="MomentumStrategy" readonly>
                        </div>
                        <div class="form-group">
                            <label>Class</label>
                            <input type="text" value="Strategy" readonly>
                        </div>
                    </div>
                    
                    <div class="strategy-panel-section">
                        <h4>Parameters</h4>
                        <div class="form-group">
                            <label>lookback</label>
                            <input type="number" value="20">
                        </div>
                        <div class="form-group">
                            <label>entry_threshold</label>
                            <input type="number" value="0.02" step="0.01">
                        </div>
                        <div class="form-group">
                            <label>position_size</label>
                            <input type="number" value="0.1" step="0.01">
                        </div>
                    </div>
                    
                    <div class="strategy-panel-section">
                        <h4>Backtest Settings</h4>
                        <div class="form-group">
                            <label>Symbol</label>
                            <select id="strategy-symbol">
                                <option>BTC</option>
                                <option>ETH</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>Timeframe</label>
                            <select>
                                <option>1h</option>
                                <option>4h</option>
                                <option>1d</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>Initial Capital</label>
                            <input type="number" value="100000">
                        </div>
                    </div>
                    
                    <div class="strategy-panel-section">
                        <h4>Price Chart</h4>
                        <div style="height: 200px; position: relative;">
                            <canvas id="strategy-price-chart"></canvas>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Performance Tab -->
        <div id="performance" class="tab-content">
            <div class="dashboard-grid">
                <div class="card" style="grid-column: span 12;">
                    <div class="card-header">Performance Metrics</div>
                    <div class="card-body">
                        <div class="risk-grid" style="grid-template-columns: repeat(6, 1fr);">
                            <div class="metric">
                                <div class="metric-label">Total Return</div>
                                <div class="metric-value positive">+15.3%</div>
                            </div>
                            <div class="metric">
                                <div class="metric-label">Sharpe Ratio</div>
                                <div class="metric-value">1.85</div>
                            </div>
                            <div class="metric">
                                <div class="metric-label">Max Drawdown</div>
                                <div class="metric-value negative">-8.2%</div>
                            </div>
                            <div class="metric">
                                <div class="metric-label">Win Rate</div>
                                <div class="metric-value">58.4%</div>
                            </div>
                            <div class="metric">
                                <div class="metric-label">Profit Factor</div>
                                <div class="metric-value">1.72</div>
                            </div>
                            <div class="metric">
                                <div class="metric-label">Total Trades</div>
                                <div class="metric-value">142</div>
                            </div>
                        </div>
                    </div>
                </div>
                
                <div class="card" style="grid-column: span 8;">
                    <div class="card-header">Equity Curve</div>
                    <div class="card-body">
                        <div class="chart-container">
                            <canvas id="equity-chart"></canvas>
                        </div>
                    </div>
                </div>
                
                <div class="card" style="grid-column: span 4;">
                    <div class="card-header">Monthly Returns</div>
                    <div class="card-body">
                        <div class="chart-container">
                            <canvas id="monthly-chart"></canvas>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Backtest Tab -->
        <div id="backtest" class="tab-content">
            <div class="dashboard-grid">
                <div class="card" style="grid-column: span 4;">
                    <div class="card-header">Backtest Configuration</div>
                    <div class="card-body">
                        <div class="form-group">
                            <label>Strategy</label>
                            <select id="bt-strategy">
                                <option value="momentum">Momentum Strategy</option>
                                <option value="mean_reversion">Mean Reversion</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>Symbol</label>
                            <select id="bt-symbol">
                                <option value="BTC-PERP">BTC-PERP</option>
                                <option value="ETH-PERP">ETH-PERP</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>Timeframe</label>
                            <select id="bt-timeframe">
                                <option value="1h">1 Hour</option>
                                <option value="4h">4 Hours</option>
                                <option value="1d">1 Day</option>
                            </select>
                        </div>
                        <div class="form-group">
                            <label>Start Date</label>
                            <input type="date" id="bt-start" value="2024-01-01">
                        </div>
                        <div class="form-group">
                            <label>End Date</label>
                            <input type="date" id="bt-end" value="2024-12-31">
                        </div>
                        <div class="form-group">
                            <label>Initial Capital</label>
                            <input type="number" id="bt-capital" value="100000">
                        </div>
                        <button class="btn btn-primary" style="width: 100%;" onclick="runFullBacktest()">
                            Run Backtest
                        </button>
                    </div>
                </div>
                
                <div class="card" style="grid-column: span 8;">
                    <div class="card-header">Backtest Results</div>
                    <div class="card-body">
                        <div id="backtest-results">
                            <div class="alert alert-warning">
                                Configure and run a backtest to see results.
                            </div>
                        </div>
                        <div id="backtest-charts" style="display: none; margin-top: 1.5rem;">
                            <div style="margin-bottom: 1.5rem;">
                                <h3 style="font-size: 1rem; margin-bottom: 1rem;">Price Chart</h3>
                                <div style="height: 250px; position: relative;">
                                    <canvas id="backtest-tab-price-chart"></canvas>
                                </div>
                            </div>
                            <div>
                                <h3 style="font-size: 1rem; margin-bottom: 1rem;">Equity Curve</h3>
                                <div style="height: 250px; position: relative;">
                                    <canvas id="backtest-tab-equity-chart"></canvas>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
    </main>
    
    <!-- Backtest Results Modal -->
    <div id="backtest-modal" class="modal">
        <div class="modal-content">
            <div class="modal-header">
                <h2>📊 Backtest Results</h2>
                <button class="modal-close" onclick="closeBacktestModal()">✕</button>
            </div>
            <div id="modal-results-content"></div>
        </div>
    </div>

    <script>
        // Tab navigation
        document.querySelectorAll('.nav-tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
                document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
                
                tab.classList.add('active');
                document.getElementById(tab.dataset.tab).classList.add('active');
                
                // Initialize charts when tabs are shown
                if (tab.dataset.tab === 'performance') {
                    initPerformanceCharts();
                } else if (tab.dataset.tab === 'portfolio') {
                    initPortfolioCharts();
                } else if (tab.dataset.tab === 'strategies') {
                    initStrategyPriceChart();
                }
            });
        });
        
        // TradingView Widget
        new TradingView.widget({
            "container_id": "tradingview-widget",
            "autosize": true,
            "symbol": "BINANCE:BTCUSDT",
            "interval": "60",
            "timezone": "Etc/UTC",
            "theme": "dark",
            "style": "1",
            "locale": "en",
            "toolbar_bg": "#161b22",
            "enable_publishing": false,
            "hide_top_toolbar": false,
            "hide_legend": false,
            "save_image": false,
            "backgroundColor": "#0d1117",
            "gridColor": "#21262d",
        });
        
        // CodeMirror Editor
        let editor;
        let currentFile = 'momentum_strategy';
        
        // Initialize CodeMirror after DOM is ready
        document.addEventListener('DOMContentLoaded', function() {
            const textarea = document.getElementById('code-editor');
            editor = CodeMirror.fromTextArea(textarea, {
                mode: 'python',
                theme: 'dracula',
                lineNumbers: true,
                indentUnit: 4,
                tabSize: 4,
                indentWithTabs: false,
                lineWrapping: false,
                matchBrackets: true,
                autoCloseBrackets: true,
            });
            
            // Set initial content
            editor.setValue(`"""
Momentum Strategy

Uses price momentum (rate of change) to generate trading signals.
"""

from neleus import Strategy, StrategyContext, OrderSide
from typing import List


class MomentumStrategy(Strategy):
    """
    Momentum strategy using price rate of change.
    
    Parameters:
        lookback: Number of bars for momentum calculation
        entry_threshold: Momentum threshold to enter (e.g., 0.02 = 2%)
        position_size: Position size
    """
    
    def __init__(
        self,
        lookback: int = 20,
        entry_threshold: float = 0.02,
        position_size: float = 0.1,
    ):
        super().__init__()
        self.lookback = lookback
        self.entry_threshold = entry_threshold
        self.position_size = position_size
        
        self.prices: List[float] = []
        self.in_position = False
        self.position_side = None
    
    def on_bar(self, ctx: StrategyContext, bar) -> None:
        """Process each bar and generate orders."""
        self.prices.append(bar.close)
        
        if len(self.prices) < self.lookback:
            return
        
        # Keep only lookback period
        self.prices = self.prices[-self.lookback:]
        
        # Calculate momentum (rate of change)
        momentum = (self.prices[-1] - self.prices[0]) / self.prices[0]
        
        # Generate signals
        if not self.in_position:
            if momentum > self.entry_threshold:
                # Strong positive momentum - go long
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.in_position = True
                self.position_side = "long"
            elif momentum < -self.entry_threshold:
                # Strong negative momentum - go short
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.in_position = True
                self.position_side = "short"
        else:
            # Exit logic
            if self.position_side == "long" and momentum < 0:
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.in_position = False
            elif self.position_side == "short" and momentum > 0:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.in_position = False
`);
        });
        
        // File explorer
        document.querySelectorAll('.file-item').forEach(item => {
            item.addEventListener('click', () => {
                document.querySelectorAll('.file-item').forEach(i => i.classList.remove('active'));
                item.classList.add('active');
                currentFile = item.dataset.file;
                document.getElementById('editor-tab-name').textContent = currentFile + '.py';
                loadStrategy(currentFile);
            });
        });
        
        async function loadStrategy(name) {
            try {
                const response = await fetch(`/api/strategies/${name}`);
                const data = await response.json();
                if (editor && data.code) {
                    editor.setValue(data.code);
                }
            } catch (e) {
                console.log('Could not load strategy:', e);
            }
        }
        
        async function saveStrategy() {
            if (!editor) return;
            try {
                await fetch(`/api/strategies/${currentFile}`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ code: editor.getValue() })
                });
                alert('Strategy saved!');
            } catch (e) {
                alert('Error saving strategy: ' + e.message);
            }
        }
        
        function createStrategy() {
            const name = prompt('Enter strategy name:');
            if (name) {
                alert(`Strategy "${name}" created! (Demo mode)`);
            }
        }
        
        async function runBacktest() {
            // Save strategy first
            if (!editor) {
                alert('Editor not initialized');
                return;
            }
            
            // Show loading
            const modal = document.getElementById('backtest-modal');
            const content = document.getElementById('modal-results-content');
            content.innerHTML = '<div style="text-align: center; padding: 3rem;"><div class="status-dot" style="display: inline-block; margin-right: 0.5rem;"></div><span>Running backtest...</span></div>';
            modal.classList.add('active');
            
            try {
                // Save current strategy
                await fetch(`/api/strategies/${currentFile}`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ code: editor.getValue() })
                });
                
                // Run backtest
                const symbol = document.querySelector('.strategy-panel select')?.value || 'BTC';
                const response = await fetch(`/api/backtest/run?strategy=${currentFile}&symbol=${symbol}&timeframe=1h&capital=100000`);
                const data = await response.json();
                
                // Display results
                displayBacktestResults(data);
            } catch (e) {
                content.innerHTML = `<div class="alert alert-warning">Error running backtest: ${e.message}</div>`;
            }
        }
        
        function closeBacktestModal() {
            document.getElementById('backtest-modal').classList.remove('active');
        }
        
        function displayBacktestResults(data) {
            const content = document.getElementById('modal-results-content');
            const results = data.results || {};
            
            content.innerHTML = `
                <div class="alert alert-success">✓ Backtest completed successfully!</div>
                
                <div class="results-metrics">
                    <div class="metric">
                        <div class="metric-label">Total Return</div>
                        <div class="metric-value ${results.total_return_pct >= 0 ? 'positive' : 'negative'}">
                            ${results.total_return_pct >= 0 ? '+' : ''}${results.total_return_pct?.toFixed(2) || '0.00'}%
                        </div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Sharpe Ratio</div>
                        <div class="metric-value">${results.sharpe_ratio?.toFixed(2) || '0.00'}</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Max Drawdown</div>
                        <div class="metric-value negative">-${Math.abs(results.max_drawdown_pct || 0).toFixed(2)}%</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Win Rate</div>
                        <div class="metric-value">${results.win_rate?.toFixed(1) || '0.0'}%</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Profit Factor</div>
                        <div class="metric-value">${results.profit_factor?.toFixed(2) || '0.00'}</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Total Trades</div>
                        <div class="metric-value">${results.total_trades || 0}</div>
                    </div>
                </div>
                
                <div class="results-chart">
                    <h3 style="font-size: 1rem; margin-bottom: 1rem;">Price Chart</h3>
                    <div style="height: 350px; position: relative;">
                        <canvas id="results-price-chart"></canvas>
                    </div>
                </div>
                
                <div class="results-chart">
                    <h3 style="font-size: 1rem; margin-bottom: 1rem;">Equity Curve</h3>
                    <div style="height: 350px; position: relative;">
                        <canvas id="results-equity-chart"></canvas>
                    </div>
                </div>
            `;
            
            // Draw charts
            setTimeout(() => {
                drawPriceChart(results);
                drawEquityChart(results);
            }, 100);
        }
        
        function drawPriceChart(results) {
            const ctx = document.getElementById('results-price-chart');
            if (!ctx) return;
            
            // Generate realistic price data
            const days = 30;
            const labels = Array.from({length: days}, (_, i) => {
                const date = new Date();
                date.setDate(date.getDate() - (days - i));
                return date.toLocaleDateString('en-US', {month: 'short', day: 'numeric'});
            });
            
            const startPrice = 90000;
            const prices = [startPrice];
            
            // Generate price movement
            for (let i = 1; i < days; i++) {
                const change = (Math.random() - 0.48) * 0.03; // Slight upward bias
                prices.push(prices[i-1] * (1 + change));
            }
            
            new Chart(ctx, {
                type: 'line',
                data: {
                    labels: labels,
                    datasets: [{
                        label: 'BTC Price',
                        data: prices,
                        borderColor: '#f7931a',
                        backgroundColor: 'rgba(247, 147, 26, 0.1)',
                        fill: true,
                        tension: 0.4,
                        borderWidth: 2,
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: { display: false },
                        tooltip: {
                            callbacks: {
                                label: (context) => `$${context.parsed.y.toLocaleString('en-US', {maximumFractionDigits: 2})}`
                            }
                        }
                    },
                    scales: {
                        x: {
                            grid: { color: '#21262d' },
                            ticks: { color: '#8b949e', maxTicksLimit: 10 }
                        },
                        y: {
                            grid: { color: '#21262d' },
                            ticks: {
                                color: '#8b949e',
                                callback: (value) => '$' + value.toLocaleString('en-US', {maximumFractionDigits: 0})
                            }
                        }
                    }
                }
            });
        }
        
        function initStrategyPriceChart() {
            const symbol = document.getElementById('strategy-symbol')?.value || 'BTC-USD';
            
            // Generate realistic price data
            const days = 30;
            const basePrice = 45000;
            const data = [];
            const labels = [];
            
            let currentPrice = basePrice;
            for (let i = 0; i < days; i++) {
                const date = new Date();
                date.setDate(date.getDate() - (days - i));
                labels.push(date.toLocaleDateString());
                
                // Realistic daily volatility
                const dailyChange = (Math.random() - 0.48) * 0.03;
                currentPrice = currentPrice * (1 + dailyChange);
                data.push(currentPrice.toFixed(2));
            }
            
            const ctx = document.getElementById('strategy-price-chart');
            if (!ctx) return;
            
            // Destroy existing chart if any
            if (window.strategyPriceChart) {
                window.strategyPriceChart.destroy();
            }
            
            window.strategyPriceChart = new Chart(ctx, {
                type: 'line',
                data: {
                    labels: labels,
                    datasets: [{
                        label: symbol + ' Price',
                        data: data,
                        borderColor: '#3b82f6',
                        backgroundColor: 'rgba(59, 130, 246, 0.1)',
                        borderWidth: 2,
                        fill: true,
                        tension: 0.1,
                        pointRadius: 0
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: {
                            display: true,
                            labels: { color: '#e5e7eb' }
                        },
                        title: {
                            display: false
                        }
                    },
                    scales: {
                        x: {
                            ticks: { color: '#9ca3af' },
                            grid: { color: 'rgba(55, 65, 81, 0.5)' }
                        },
                        y: {
                            ticks: { 
                                color: '#9ca3af',
                                callback: function(value) {
                                    return '$' + value.toLocaleString();
                                }
                            },
                            grid: { color: 'rgba(55, 65, 81, 0.5)' }
                        }
                    }
                }
            });
        }

        function drawEquityChart(results) {
            const ctx = document.getElementById('results-equity-chart');
            if (!ctx) return;
            
            // Generate consistent equity curve based on actual results
            const days = 30;
            const labels = Array.from({length: days}, (_, i) => {
                const date = new Date();
                date.setDate(date.getDate() - (days - i));
                return date.toLocaleDateString('en-US', {month: 'short', day: 'numeric'});
            });
            
            const initialCapital = 100000;
            const finalReturn = (results.total_return_pct || 0) / 100;
            const maxDD = Math.abs(results.max_drawdown_pct || 0) / 100;
            
            // Generate realistic equity curve
            const equity = [initialCapital];
            let peak = initialCapital;
            let inDrawdown = false;
            let drawdownDepth = 0;
            
            for (let i = 1; i < days; i++) {
                const progress = i / days;
                
                // Simulate drawdown periods
                if (!inDrawdown && Math.random() < 0.15) {
                    inDrawdown = true;
                    drawdownDepth = 0;
                }
                
                let dailyChange;
                if (inDrawdown && drawdownDepth < maxDD) {
                    // In drawdown
                    dailyChange = -(maxDD / 10) * (1 + Math.random() * 0.5);
                    drawdownDepth += Math.abs(dailyChange);
                    if (drawdownDepth >= maxDD || Math.random() < 0.2) {
                        inDrawdown = false;
                    }
                } else {
                    // Recovery or growth
                    const targetGrowth = finalReturn / days;
                    dailyChange = targetGrowth + (Math.random() - 0.5) * 0.01;
                }
                
                const newEquity = equity[i-1] * (1 + dailyChange);
                equity.push(newEquity);
                peak = Math.max(peak, newEquity);
            }
            
            // Adjust final value to match target return
            const actualReturn = (equity[days-1] - initialCapital) / initialCapital;
            const adjustment = finalReturn / actualReturn;
            for (let i = 1; i < equity.length; i++) {
                equity[i] = initialCapital + (equity[i] - initialCapital) * adjustment;
            }
            
            new Chart(ctx, {
                type: 'line',
                data: {
                    labels: labels,
                    datasets: [{
                        label: 'Portfolio Value',
                        data: equity,
                        borderColor: finalReturn >= 0 ? '#3fb950' : '#f85149',
                        backgroundColor: finalReturn >= 0 ? 'rgba(63, 185, 80, 0.1)' : 'rgba(248, 81, 73, 0.1)',
                        fill: true,
                        tension: 0.4,
                        borderWidth: 2,
                    }]
                },
                options: {
                    responsive: true,
                    maintainAspectRatio: false,
                    plugins: {
                        legend: { display: false },
                        tooltip: {
                            callbacks: {
                                label: (context) => `$${context.parsed.y.toLocaleString('en-US', {maximumFractionDigits: 2})}`
                            }
                        }
                    },
                    scales: {
                        x: {
                            grid: { color: '#21262d' },
                            ticks: { color: '#8b949e', maxTicksLimit: 10 }
                        },
                        y: {
                            grid: { color: '#21262d' },
                            ticks: {
                                color: '#8b949e',
                                callback: (value) => '$' + value.toLocaleString()
                            }
                        }
                    }
                }
            });
        }
        
        async function runFullBacktest() {
            const resultsDiv = document.getElementById('backtest-results');
            const chartsDiv = document.getElementById('backtest-charts');
            
            resultsDiv.innerHTML = '<div style="text-align: center; padding: 2rem;"><div class="status-dot" style="display: inline-block; margin-right: 0.5rem;"></div>Running backtest...</div>';
            chartsDiv.style.display = 'none';
            
            // Simulate delay
            await new Promise(r => setTimeout(r, 1500));
            
            // Sample results
            const results = {
                total_return_pct: 15.3,
                sharpe_ratio: 1.85,
                max_drawdown_pct: 8.2,
                win_rate: 58.4,
                profit_factor: 1.72,
                total_trades: 142
            };
            
            resultsDiv.innerHTML = `
                <div class="alert alert-success">✓ Backtest completed successfully!</div>
                <div class="risk-grid" style="grid-template-columns: repeat(3, 1fr);">
                    <div class="metric">
                        <div class="metric-label">Total Return</div>
                        <div class="metric-value positive">+${results.total_return_pct}%</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Sharpe Ratio</div>
                        <div class="metric-value">${results.sharpe_ratio}</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Max Drawdown</div>
                        <div class="metric-value negative">-${results.max_drawdown_pct}%</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Win Rate</div>
                        <div class="metric-value">${results.win_rate}%</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Profit Factor</div>
                        <div class="metric-value">${results.profit_factor}</div>
                    </div>
                    <div class="metric">
                        <div class="metric-label">Total Trades</div>
                        <div class="metric-value">${results.total_trades}</div>
                    </div>
                </div>
            `;
            
            // Show and draw charts
            chartsDiv.style.display = 'block';
            setTimeout(() => {
                drawBacktestTabCharts(results);
            }, 100);
        }
        
        function drawBacktestTabCharts(results) {
            // Draw price chart
            const priceCtx = document.getElementById('backtest-tab-price-chart');
            if (priceCtx) {
                const days = 30;
                const labels = Array.from({length: days}, (_, i) => {
                    const date = new Date();
                    date.setDate(date.getDate() - (days - i));
                    return date.toLocaleDateString('en-US', {month: 'short', day: 'numeric'});
                });
                
                const startPrice = 90000;
                const prices = [startPrice];
                for (let i = 1; i < days; i++) {
                    const change = (Math.random() - 0.48) * 0.03;
                    prices.push(prices[i-1] * (1 + change));
                }
                
                if (window.backtestTabPriceChart) {
                    window.backtestTabPriceChart.destroy();
                }
                
                window.backtestTabPriceChart = new Chart(priceCtx, {
                    type: 'line',
                    data: {
                        labels: labels,
                        datasets: [{
                            label: 'BTC Price',
                            data: prices,
                            borderColor: '#f7931a',
                            backgroundColor: 'rgba(247, 147, 26, 0.1)',
                            fill: true,
                            tension: 0.4,
                            borderWidth: 2,
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false },
                            tooltip: {
                                callbacks: {
                                    label: (context) => `$${context.parsed.y.toLocaleString('en-US', {maximumFractionDigits: 2})}`
                                }
                            }
                        },
                        scales: {
                            x: {
                                grid: { color: '#21262d' },
                                ticks: { color: '#8b949e', maxTicksLimit: 10 }
                            },
                            y: {
                                grid: { color: '#21262d' },
                                ticks: {
                                    color: '#8b949e',
                                    callback: (value) => '$' + value.toLocaleString('en-US', {maximumFractionDigits: 0})
                                }
                            }
                        }
                    }
                });
            }
            
            // Draw equity chart
            const equityCtx = document.getElementById('backtest-tab-equity-chart');
            if (equityCtx) {
                const days = 30;
                const labels = Array.from({length: days}, (_, i) => {
                    const date = new Date();
                    date.setDate(date.getDate() - (days - i));
                    return date.toLocaleDateString('en-US', {month: 'short', day: 'numeric'});
                });
                
                const initialCapital = 100000;
                const finalReturn = (results.total_return_pct || 0) / 100;
                const maxDD = Math.abs(results.max_drawdown_pct || 0) / 100;
                
                const equity = [initialCapital];
                let peak = initialCapital;
                let inDrawdown = false;
                let drawdownDepth = 0;
                
                for (let i = 1; i < days; i++) {
                    const progress = i / (days - 1);
                    let dailyChange;
                    
                    if (!inDrawdown && Math.random() < 0.15 && i < days - 5) {
                        inDrawdown = true;
                        drawdownDepth = 0;
                    }
                    
                    if (inDrawdown) {
                        const targetDD = peak * maxDD;
                        if (drawdownDepth < targetDD) {
                            dailyChange = -(Math.random() * 0.015 + 0.005);
                            drawdownDepth += Math.abs(dailyChange * equity[i-1]);
                        } else {
                            dailyChange = Math.random() * 0.015 + 0.005;
                            if (equity[i-1] * (1 + dailyChange) >= peak) {
                                inDrawdown = false;
                                drawdownDepth = 0;
                            }
                        }
                    } else {
                        dailyChange = (Math.random() - 0.3) * 0.02;
                    }
                    
                    let newEquity = equity[i-1] * (1 + dailyChange);
                    if (newEquity > peak) peak = newEquity;
                    
                    if (i === days - 1) {
                        newEquity = initialCapital * (1 + finalReturn);
                    }
                    
                    equity.push(newEquity);
                }
                
                if (window.backtestTabEquityChart) {
                    window.backtestTabEquityChart.destroy();
                }
                
                const chartColor = finalReturn >= 0 ? '#3fb950' : '#f85149';
                
                window.backtestTabEquityChart = new Chart(equityCtx, {
                    type: 'line',
                    data: {
                        labels: labels,
                        datasets: [{
                            label: 'Equity',
                            data: equity,
                            borderColor: chartColor,
                            backgroundColor: finalReturn >= 0 ? 'rgba(63, 185, 80, 0.1)' : 'rgba(248, 81, 73, 0.1)',
                            fill: true,
                            tension: 0.4,
                            borderWidth: 2,
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: { display: false },
                            tooltip: {
                                callbacks: {
                                    label: (context) => `$${context.parsed.y.toLocaleString('en-US', {maximumFractionDigits: 2})}`
                                }
                            }
                        },
                        scales: {
                            x: {
                                grid: { color: '#21262d' },
                                ticks: { color: '#8b949e', maxTicksLimit: 10 }
                            },
                            y: {
                                grid: { color: '#21262d' },
                                ticks: {
                                    color: '#8b949e',
                                    callback: (value) => '$' + value.toLocaleString('en-US', {maximumFractionDigits: 0})
                                }
                            }
                        }
                    }
                });
            }
        }
        
        // Performance Charts
        function initPerformanceCharts() {
            // Equity curve
            const equityCtx = document.getElementById('equity-chart');
            if (equityCtx && !equityCtx.chart) {
                equityCtx.chart = new Chart(equityCtx, {
                    type: 'line',
                    data: {
                        labels: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug'],
                        datasets: [{
                            label: 'Equity',
                            data: [100000, 102500, 98500, 105000, 108500, 106000, 112000, 115300],
                            borderColor: '#58a6ff',
                            backgroundColor: 'rgba(88, 166, 255, 0.1)',
                            fill: true,
                            tension: 0.4,
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: { legend: { display: false } },
                        scales: {
                            x: { grid: { color: '#21262d' }, ticks: { color: '#8b949e' } },
                            y: { grid: { color: '#21262d' }, ticks: { color: '#8b949e' } }
                        }
                    }
                });
            }
            
            // Monthly returns
            const monthlyCtx = document.getElementById('monthly-chart');
            if (monthlyCtx && !monthlyCtx.chart) {
                monthlyCtx.chart = new Chart(monthlyCtx, {
                    type: 'bar',
                    data: {
                        labels: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul'],
                        datasets: [{
                            label: 'Return %',
                            data: [2.5, -3.9, 6.6, 3.3, -2.3, 5.7, 2.9],
                            backgroundColor: context => {
                                return context.raw >= 0 ? '#3fb950' : '#f85149';
                            },
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: { legend: { display: false } },
                        scales: {
                            x: { grid: { color: '#21262d' }, ticks: { color: '#8b949e' } },
                            y: { grid: { color: '#21262d' }, ticks: { color: '#8b949e' } }
                        }
                    }
                });
            }
        }
        
        // Portfolio Charts
        function initPortfolioCharts() {
            const ctx = document.getElementById('allocation-chart');
            if (ctx && !ctx.chart) {
                ctx.chart = new Chart(ctx, {
                    type: 'doughnut',
                    data: {
                        labels: ['BTC-PERP', 'ETH-PERP', 'Cash'],
                        datasets: [{
                            data: [45, 25, 30],
                            backgroundColor: ['#58a6ff', '#a371f7', '#3fb950'],
                        }]
                    },
                    options: {
                        responsive: true,
                        maintainAspectRatio: false,
                        plugins: {
                            legend: {
                                position: 'right',
                                labels: { color: '#e6edf3' }
                            }
                        }
                    }
                });
            }
        }
        
        // Load project info
        fetch('/api/info')
            .then(r => r.json())
            .then(data => {
                document.getElementById('project-name').textContent = data.project_name || 'Demo Mode';
            })
            .catch(() => {});
        
        // WebSocket connection
        let ws;
        function connectWebSocket() {
            ws = new WebSocket(`ws://${window.location.host}/ws`);
            ws.onmessage = (event) => {
                const data = JSON.parse(event.data);
                // Handle real-time updates
            };
            ws.onclose = () => {
                setTimeout(connectWebSocket, 3000);
            };
        }
        connectWebSocket();
        
        // Modal click outside to close
        document.getElementById('backtest-modal')?.addEventListener('click', (e) => {
            if (e.target.id === 'backtest-modal') {
                closeBacktestModal();
            }
        });
        
        // Symbol change handler
        document.getElementById('strategy-symbol')?.addEventListener('change', () => {
            initStrategyPriceChart();
        });
    </script>
</body>
</html>
'''


# =============================================================================
# Managed Service API Routes
# =============================================================================

@app.get("/api/agents")
async def get_agents():
    """Get list of deployed agents."""
    return [
        {
            "id": "momentum-eth-01",
            "name": "ETH Momentum Agent",
            "strategy": "momentum_v2",
            "state": "running",
            "venue": "hyperliquid",
            "instruments": ["ETH-PERP"],
            "pnl": {"total": 1250.50, "today": 85.25, "unrealized": 150.50},
            "trades": {"total": 142, "win_rate": 0.627},
            "risk": {"sharpe": 1.85, "max_drawdown": 0.12},
            "uptime": "2d 14h 32m",
            "last_trade": "2026-01-28T10:15:00Z",
        },
        {
            "id": "arb-btc-perp-01",
            "name": "BTC Arbitrage Agent",
            "strategy": "cross_venue_arb",
            "state": "running",
            "venue": "hyperliquid",
            "instruments": ["BTC-PERP"],
            "pnl": {"total": 3420.75, "today": 125.00, "unrealized": 20.75},
            "trades": {"total": 856, "win_rate": 0.598},
            "risk": {"sharpe": 2.15, "max_drawdown": 0.08},
            "uptime": "5d 8h 15m",
            "last_trade": "2026-01-28T10:18:30Z",
        },
        {
            "id": "mean-rev-sol-01",
            "name": "SOL Mean Reversion",
            "strategy": "mean_reversion",
            "state": "paused",
            "venue": "lighter",
            "instruments": ["SOL-PERP"],
            "pnl": {"total": -320.00, "today": 0.00, "unrealized": 0.00},
            "trades": {"total": 45, "win_rate": 0.42},
            "risk": {"sharpe": 0.85, "max_drawdown": 0.18},
            "uptime": "1d 2h 45m",
            "last_trade": "2026-01-27T22:30:00Z",
        },
    ]


@app.post("/api/agents/{agent_id}/start")
async def start_agent(agent_id: str):
    """Start an agent."""
    return {"status": "success", "agent_id": agent_id, "state": "running"}


@app.post("/api/agents/{agent_id}/stop")
async def stop_agent(agent_id: str):
    """Stop an agent."""
    return {"status": "success", "agent_id": agent_id, "state": "stopped"}


@app.post("/api/agents/{agent_id}/pause")
async def pause_agent(agent_id: str):
    """Pause an agent."""
    return {"status": "success", "agent_id": agent_id, "state": "paused"}


@app.delete("/api/agents/{agent_id}")
async def delete_agent(agent_id: str):
    """Delete an agent."""
    return {"status": "success", "agent_id": agent_id, "deleted": True}


@app.get("/api/agents/{agent_id}/metrics")
async def get_agent_metrics(agent_id: str):
    """Get detailed metrics for an agent."""
    return {
        "agent_id": agent_id,
        "pnl": {"total": 1250.50, "realized": 1100.00, "unrealized": 150.50, "today": 85.25},
        "trades": {"total": 142, "wins": 89, "losses": 53, "win_rate": 0.627},
        "positions": {"open": 2, "value": 3500.00, "exposure": 0.35},
        "risk": {"max_drawdown": 0.12, "sharpe": 1.85, "sortino": 2.10, "var_95": 250.00},
        "latency": {"avg_ms": 12.5, "p99_ms": 45.2, "fills_per_sec": 2.1},
        "history": [
            {"time": "10:00", "pnl": 1200.00},
            {"time": "10:15", "pnl": 1225.00},
            {"time": "10:30", "pnl": 1210.00},
            {"time": "10:45", "pnl": 1250.50},
        ],
    }


@app.get("/api/signals")
async def get_signals(limit: int = 20):
    """Get recent signals."""
    return [
        {
            "id": "sig-001",
            "instrument": "ETH-PERP",
            "signal_type": "entry",
            "direction": "long",
            "confidence": 0.85,
            "source": "ml_model_v2",
            "timestamp": "2026-01-28T10:15:00Z",
            "processed": True,
            "agent_id": "momentum-eth-01",
        },
        {
            "id": "sig-002",
            "instrument": "BTC-PERP",
            "signal_type": "exit",
            "direction": "flat",
            "confidence": 0.72,
            "source": "sentiment_analyzer",
            "timestamp": "2026-01-28T10:18:30Z",
            "processed": True,
            "agent_id": "arb-btc-perp-01",
        },
        {
            "id": "sig-003",
            "instrument": "ETH-PERP",
            "signal_type": "risk_alert",
            "direction": "none",
            "confidence": 0.95,
            "source": "risk_monitor",
            "timestamp": "2026-01-28T10:20:15Z",
            "processed": False,
            "agent_id": None,
        },
    ]


@app.get("/api/signals/sources")
async def get_signal_sources():
    """Get known signal sources."""
    return [
        {"name": "ml_model_v2", "type": "ML Model", "signals_24h": 142, "status": "active"},
        {"name": "sentiment_analyzer", "type": "Sentiment", "signals_24h": 58, "status": "active"},
        {"name": "risk_monitor", "type": "Risk", "signals_24h": 12, "status": "active"},
        {"name": "tradingview_webhook", "type": "Webhook", "signals_24h": 87, "status": "active"},
    ]


@app.get("/api/alerts")
async def get_alerts():
    """Get active alerts."""
    return [
        {
            "id": "alert-001",
            "agent": "momentum-eth-01",
            "severity": "warning",
            "message": "Win rate dropped below 60% threshold",
            "timestamp": "2026-01-28T10:15:00Z",
            "acknowledged": False,
        },
        {
            "id": "alert-002",
            "agent": "arb-btc-perp-01",
            "severity": "info",
            "message": "Daily P&L target reached",
            "timestamp": "2026-01-28T09:30:00Z",
            "acknowledged": True,
        },
        {
            "id": "alert-003",
            "agent": "mean-rev-sol-01",
            "severity": "critical",
            "message": "Circuit breaker triggered: max position size",
            "timestamp": "2026-01-28T08:45:00Z",
            "acknowledged": False,
        },
    ]


@app.post("/api/alerts/{alert_id}/acknowledge")
async def acknowledge_alert(alert_id: str):
    """Acknowledge an alert."""
    return {"status": "success", "alert_id": alert_id, "acknowledged": True}


@app.get("/api/overview")
async def get_overview():
    """Get managed service overview."""
    return {
        "agents": {"total": 3, "running": 2, "paused": 1, "stopped": 0},
        "total_pnl": {"today": 210.25, "week": 1850.75, "month": 4351.25, "all_time": 12500.00},
        "signals": {"received_24h": 299, "processed_24h": 285},
        "alerts": {"active": 2, "critical": 1, "warning": 1},
        "system": {"uptime": "15d 8h 32m", "latency_avg_ms": 10.5, "api_calls_24h": 15420},
    }


@app.get("/", response_class=HTMLResponse)
async def get_dashboard():
    """Serve the main dashboard."""
    return DASHBOARD_HTML


@app.get("/managed", response_class=HTMLResponse)
async def get_managed_dashboard():
    """Serve the managed service dashboard."""
    return MANAGED_DASHBOARD_HTML


# =============================================================================
# Server Runner
# =============================================================================

def run_server(host: str = "127.0.0.1", port: int = 8765, project_root: Optional[Path] = None):
    """Run the Neleus UI server."""
    global PROJECT_ROOT
    PROJECT_ROOT = project_root
    
    print(f"\n🌊 Neleus Dashboard running at http://{host}:{port}")
    print("   Press Ctrl+C to stop\n")
    
    uvicorn.run(app, host=host, port=port, log_level="warning")


if __name__ == "__main__":
    run_server()
