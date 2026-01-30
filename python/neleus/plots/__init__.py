import json
from dataclasses import dataclass
from decimal import Decimal
from typing import List, Optional, Dict, Any
from datetime import datetime


@dataclass
class PlotData:
                                  
    title: str
    chart_type: str
    data: Dict[str, Any]
    options: Dict[str, Any] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "title": self.title,
            "chartType": self.chart_type,
            "data": self.data,
            "options": self.options or {},
        }
    
    def to_json(self) -> str:
        return json.dumps(self.to_dict(), default=str)


class BacktestPlotter:
\
\
\
\
\
\
\
\
\
       
    
    def __init__(self, results):
\
\
\
\
\
           
        self.results = results
    
    def equity_curve(self) -> PlotData:
                                              
        equity = getattr(self.results, "equity_curve", [])
        
        return PlotData(
            title="Equity Curve",
            chart_type="line",
            data={
                "labels": [e.get("timestamp", i) for i, e in enumerate(equity)],
                "datasets": [{
                    "label": "Portfolio Value",
                    "data": [float(e.get("value", 0)) for e in equity],
                    "borderColor": "#0f6b6d",
                    "fill": False,
                }],
            },
            options={
                "responsive": True,
                "scales": {
                    "y": {"title": {"display": True, "text": "Value"}},
                    "x": {"title": {"display": True, "text": "Time"}},
                },
            },
        )
    
    def drawdown(self) -> PlotData:
                                      
        equity = getattr(self.results, "equity_curve", [])
        
                            
        values = [float(e.get("value", 0)) for e in equity]
        if not values:
            values = [0]
        
        peak = values[0]
        drawdowns = []
        for v in values:
            if v > peak:
                peak = v
            dd = (v - peak) / peak * 100 if peak > 0 else 0
            drawdowns.append(dd)
        
        return PlotData(
            title="Drawdown",
            chart_type="line",
            data={
                "labels": [e.get("timestamp", i) for i, e in enumerate(equity)],
                "datasets": [{
                    "label": "Drawdown %",
                    "data": drawdowns,
                    "borderColor": "#c97a2b",
                    "backgroundColor": "rgba(201, 122, 43, 0.3)",
                    "fill": True,
                }],
            },
            options={
                "responsive": True,
                "scales": {
                    "y": {"title": {"display": True, "text": "Drawdown %"}, "reverse": True},
                },
            },
        )
    
    def trade_scatter(self) -> PlotData:
                                                         
        trades = getattr(self.results, "trades", [])
        prices = getattr(self.results, "prices", [])
        
        buys = [t for t in trades if t.get("side") == "buy"]
        sells = [t for t in trades if t.get("side") == "sell"]
        
        return PlotData(
            title="Trades on Price",
            chart_type="scatter",
            data={
                "datasets": [
                    {
                        "label": "Price",
                        "data": [{"x": i, "y": float(p)} for i, p in enumerate(prices)],
                        "type": "line",
                        "borderColor": "#888",
                        "pointRadius": 0,
                    },
                    {
                        "label": "Buy",
                        "data": [{"x": t.get("bar_index", 0), "y": float(t.get("price", 0))} for t in buys],
                        "backgroundColor": "#22c55e",
                        "pointRadius": 6,
                    },
                    {
                        "label": "Sell",
                        "data": [{"x": t.get("bar_index", 0), "y": float(t.get("price", 0))} for t in sells],
                        "backgroundColor": "#ef4444",
                        "pointRadius": 6,
                    },
                ],
            },
        )
    
    def pnl_distribution(self) -> PlotData:
                                                  
        trades = getattr(self.results, "trades", [])
        pnls = [float(t.get("pnl", 0)) for t in trades]
        
                                  
        if not pnls:
            bins = [0]
            counts = [0]
        else:
            min_pnl, max_pnl = min(pnls), max(pnls)
            n_bins = min(20, len(pnls))
            bin_width = (max_pnl - min_pnl) / n_bins if n_bins > 0 else 1
            
            bins = [min_pnl + i * bin_width for i in range(n_bins + 1)]
            counts = [0] * n_bins
            
            for pnl in pnls:
                for i in range(n_bins):
                    if bins[i] <= pnl < bins[i + 1]:
                        counts[i] += 1
                        break
        
        return PlotData(
            title="PnL Distribution",
            chart_type="bar",
            data={
                "labels": [f"{b:.2f}" for b in bins[:-1]] if len(bins) > 1 else ["0"],
                "datasets": [{
                    "label": "Trade Count",
                    "data": counts,
                    "backgroundColor": [
                        "#22c55e" if bins[i] >= 0 else "#ef4444"
                        for i in range(len(counts))
                    ] if len(bins) > 1 else ["#888"],
                }],
            },
        )
    
    def rolling_sharpe(self, window: int = 30) -> PlotData:
                                                  
        equity = getattr(self.results, "equity_curve", [])
        values = [float(e.get("value", 0)) for e in equity]
        
        if len(values) < window:
            rolling_sharpe = [0] * len(values)
        else:
                               
            returns = []
            for i in range(1, len(values)):
                if values[i - 1] > 0:
                    returns.append((values[i] - values[i - 1]) / values[i - 1])
                else:
                    returns.append(0)
            
                            
            import math
            rolling_sharpe = [0] * (window - 1)
            for i in range(window - 1, len(returns)):
                window_returns = returns[i - window + 1:i + 1]
                mean_ret = sum(window_returns) / len(window_returns)
                std_ret = math.sqrt(sum((r - mean_ret) ** 2 for r in window_returns) / len(window_returns))
                sharpe = (mean_ret / std_ret * math.sqrt(252)) if std_ret > 0 else 0
                rolling_sharpe.append(sharpe)
        
        return PlotData(
            title=f"Rolling Sharpe Ratio ({window}-period)",
            chart_type="line",
            data={
                "labels": list(range(len(rolling_sharpe))),
                "datasets": [{
                    "label": "Sharpe Ratio",
                    "data": rolling_sharpe,
                    "borderColor": "#0f6b6d",
                }],
            },
        )
    
    def all_plots(self) -> List[PlotData]:
                                          
        return [
            self.equity_curve(),
            self.drawdown(),
            self.trade_scatter(),
            self.pnl_distribution(),
            self.rolling_sharpe(),
        ]


class LivePlotter:
\
\
       
    
    def __init__(self):
        self.data_buffer = {
            "prices": [],
            "positions": [],
            "pnl": [],
            "orders": [],
        }
    
    def add_price(self, timestamp: datetime, price: float, symbol: str = "default"):
                                              
        self.data_buffer["prices"].append({
            "timestamp": timestamp.isoformat(),
            "price": price,
            "symbol": symbol,
        })
                               
        if len(self.data_buffer["prices"]) > 1000:
            self.data_buffer["prices"] = self.data_buffer["prices"][-1000:]
    
    def add_position(self, timestamp: datetime, size: float, symbol: str = "default"):
                                    
        self.data_buffer["positions"].append({
            "timestamp": timestamp.isoformat(),
            "size": size,
            "symbol": symbol,
        })
    
    def add_pnl(self, timestamp: datetime, pnl: float):
                              
        self.data_buffer["pnl"].append({
            "timestamp": timestamp.isoformat(),
            "pnl": pnl,
        })
    
    def get_chart_data(self) -> Dict[str, Any]:
                                                   
        return self.data_buffer


def generate_html_report(
    results,
    output_path: str,
    title: str = "Neleus Backtest Report",
) -> None:
\
\
       
    plotter = BacktestPlotter(results)
    plots = plotter.all_plots()
    
                         
    summary = getattr(results, "summary_dict", lambda: {})()
    
    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        :root {{
            --bg: #f6f1e9;
            --card: #ffffff;
            --text: #1c1f24;
            --muted: #6b7280;
            --accent: #0f6b6d;
            --border: #e5e7eb;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg);
            color: var(--text);
            padding: 24px;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1 {{ font-size: 28px; margin-bottom: 24px; }}
        .metrics {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 16px;
            margin-bottom: 32px;
        }}
        .metric {{
            background: var(--card);
            border-radius: 8px;
            padding: 16px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }}
        .metric-label {{ font-size: 12px; color: var(--muted); text-transform: uppercase; }}
        .metric-value {{ font-size: 24px; font-weight: 600; margin-top: 4px; }}
        .charts {{ display: grid; gap: 24px; }}
        .chart-card {{
            background: var(--card);
            border-radius: 8px;
            padding: 20px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }}
        .chart-title {{ font-size: 16px; font-weight: 600; margin-bottom: 16px; }}
        canvas {{ max-height: 300px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>{title}</h1>
        
        <div class="metrics">
            <div class="metric">
                <div class="metric-label">Total Return</div>
                <div class="metric-value">{summary.get('total_return', 0):.2%}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Sharpe Ratio</div>
                <div class="metric-value">{summary.get('sharpe_ratio', 0):.2f}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Max Drawdown</div>
                <div class="metric-value">{summary.get('max_drawdown', 0):.2%}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Win Rate</div>
                <div class="metric-value">{summary.get('win_rate', 0):.1%}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Total Trades</div>
                <div class="metric-value">{summary.get('total_trades', 0)}</div>
            </div>
            <div class="metric">
                <div class="metric-label">Profit Factor</div>
                <div class="metric-value">{summary.get('profit_factor', 0):.2f}</div>
            </div>
        </div>
        
        <div class="charts">
"""
    
    for i, plot in enumerate(plots):
        html += f"""
            <div class="chart-card">
                <div class="chart-title">{plot.title}</div>
                <canvas id="chart{i}"></canvas>
            </div>
"""
    
    html += """
        </div>
    </div>
    
    <script>
"""
    
    for i, plot in enumerate(plots):
        html += f"""
        new Chart(document.getElementById('chart{i}'), {{
            type: '{plot.chart_type}',
            data: {json.dumps(plot.data, default=str)},
            options: {json.dumps(plot.options or {}, default=str)}
        }});
"""
    
    html += """
    </script>
</body>
</html>
"""
    
    with open(output_path, "w") as f:
        f.write(html)
