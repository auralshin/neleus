"""
Neleus CLI - Metrics Command

View agent metrics and performance data.
"""

import os
import json
from typing import Optional
from datetime import datetime, timedelta

import typer
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.progress import Progress, SpinnerColumn, TextColumn

console = Console()

metrics_app = typer.Typer(
    name="metrics",
    help="View agent metrics and performance",
)


def get_monitor_url() -> str:
    """Get monitor URL from environment or default."""
    return os.environ.get("NELEUS_MONITOR_URL", "http://localhost:8082")


def get_orchestrator_url() -> str:
    """Get orchestrator URL from environment or default."""
    return os.environ.get("NELEUS_ORCHESTRATOR_URL", "http://localhost:8080")


# Demo metrics for display
DEMO_AGENT_METRICS = {
    "momentum-eth-01": {
        "pnl": {"total": 1250.50, "realized": 1100.00, "unrealized": 150.50, "today": 85.25},
        "trades": {"total": 142, "wins": 89, "losses": 53, "win_rate": 0.627},
        "positions": {"open": 2, "value": 3500.00, "exposure": 0.35},
        "risk": {"max_drawdown": 0.12, "sharpe": 1.85, "sortino": 2.10, "var_95": 250.00},
        "latency": {"avg_ms": 12.5, "p99_ms": 45.2, "fills_per_sec": 2.1},
    },
    "arb-btc-perp-01": {
        "pnl": {"total": 3420.75, "realized": 3400.00, "unrealized": 20.75, "today": 125.00},
        "trades": {"total": 856, "wins": 512, "losses": 344, "win_rate": 0.598},
        "positions": {"open": 1, "value": 1200.00, "exposure": 0.12},
        "risk": {"max_drawdown": 0.08, "sharpe": 2.15, "sortino": 2.85, "var_95": 180.00},
        "latency": {"avg_ms": 8.2, "p99_ms": 28.5, "fills_per_sec": 4.5},
    },
}


def format_pnl(value: float) -> str:
    """Format P&L with color."""
    if value > 0:
        return f"[green]+${value:,.2f}[/green]"
    elif value < 0:
        return f"[red]-${abs(value):,.2f}[/red]"
    else:
        return f"$0.00"


def format_percent(value: float, invert: bool = False) -> str:
    """Format percentage with color."""
    color = "green" if value > 0 else "red"
    if invert:
        color = "red" if value > 0 else "green"
    return f"[{color}]{value:.1%}[/{color}]"


@metrics_app.command("get")
def metrics_get(
    agent_id: str = typer.Argument(..., help="Agent ID to get metrics for"),
    category: Optional[str] = typer.Option(None, "--category", "-c", help="Category: pnl, trades, positions, risk, latency"),
    output_format: str = typer.Option("table", "--format", "-f", help="Output format: table, json"),
    monitor_url: Optional[str] = typer.Option(None, "--url", help="Monitor URL"),
):
    """
    Get metrics for a specific agent.
    
    Examples:
        neleus metrics get momentum-eth-01
        neleus metrics get momentum-eth-01 -c pnl
        neleus metrics get momentum-eth-01 --format json
    """
    url = monitor_url or get_monitor_url()
    
    # Get metrics (demo for now)
    metrics = DEMO_AGENT_METRICS.get(agent_id)
    
    if not metrics:
        # Try first agent as fallback demo
        demo_id = list(DEMO_AGENT_METRICS.keys())[0]
        metrics = DEMO_AGENT_METRICS[demo_id]
        console.print(f"[yellow]Demo:[/yellow] Showing sample metrics for agent")
    
    if output_format == "json":
        if category:
            console.print_json(data=metrics.get(category, {}))
        else:
            console.print_json(data=metrics)
        return
    
    # Display panel
    console.print(f"\n📊 [bold]Metrics: {agent_id}[/bold]\n")
    
    # P&L Section
    if not category or category == "pnl":
        pnl = metrics.get("pnl", {})
        console.print(Panel.fit(
            f"""[bold]Total P&L:[/bold]      {format_pnl(pnl.get('total', 0))}
[bold]Realized:[/bold]        {format_pnl(pnl.get('realized', 0))}
[bold]Unrealized:[/bold]      {format_pnl(pnl.get('unrealized', 0))}
[bold]Today:[/bold]           {format_pnl(pnl.get('today', 0))}""",
            title="💰 P&L",
            border_style="green" if pnl.get('total', 0) > 0 else "red",
        ))
    
    # Trades Section
    if not category or category == "trades":
        trades = metrics.get("trades", {})
        console.print(Panel.fit(
            f"""[bold]Total Trades:[/bold]   {trades.get('total', 0):,}
[bold]Wins:[/bold]            [green]{trades.get('wins', 0):,}[/green]
[bold]Losses:[/bold]          [red]{trades.get('losses', 0):,}[/red]
[bold]Win Rate:[/bold]        {format_percent(trades.get('win_rate', 0))}""",
            title="📈 Trades",
            border_style="cyan",
        ))
    
    # Positions Section
    if not category or category == "positions":
        pos = metrics.get("positions", {})
        console.print(Panel.fit(
            f"""[bold]Open Positions:[/bold] {pos.get('open', 0)}
[bold]Position Value:[/bold]  ${pos.get('value', 0):,.2f}
[bold]Exposure:[/bold]        {format_percent(pos.get('exposure', 0))}""",
            title="📋 Positions",
            border_style="yellow",
        ))
    
    # Risk Section
    if not category or category == "risk":
        risk = metrics.get("risk", {})
        console.print(Panel.fit(
            f"""[bold]Max Drawdown:[/bold]   {format_percent(risk.get('max_drawdown', 0), invert=True)}
[bold]Sharpe Ratio:[/bold]   [cyan]{risk.get('sharpe', 0):.2f}[/cyan]
[bold]Sortino Ratio:[/bold]  [cyan]{risk.get('sortino', 0):.2f}[/cyan]
[bold]VaR (95%):[/bold]       ${risk.get('var_95', 0):,.2f}""",
            title="⚠️ Risk",
            border_style="magenta",
        ))
    
    # Latency Section
    if not category or category == "latency":
        lat = metrics.get("latency", {})
        console.print(Panel.fit(
            f"""[bold]Avg Latency:[/bold]    {lat.get('avg_ms', 0):.1f}ms
[bold]P99 Latency:[/bold]    {lat.get('p99_ms', 0):.1f}ms
[bold]Fills/sec:[/bold]       {lat.get('fills_per_sec', 0):.1f}""",
            title="⚡ Latency",
            border_style="blue",
        ))


@metrics_app.command("summary")
def metrics_summary(
    monitor_url: Optional[str] = typer.Option(None, "--url", help="Monitor URL"),
):
    """Show metrics summary for all agents."""
    console.print("\n📊 [bold]All Agents - Metrics Summary[/bold]\n")
    
    table = Table()
    table.add_column("Agent", style="cyan")
    table.add_column("P&L (Total)", justify="right")
    table.add_column("Today", justify="right")
    table.add_column("Trades", justify="right")
    table.add_column("Win Rate", justify="right")
    table.add_column("Sharpe", justify="right")
    table.add_column("Drawdown", justify="right")
    
    for agent_id, metrics in DEMO_AGENT_METRICS.items():
        pnl = metrics.get("pnl", {})
        trades = metrics.get("trades", {})
        risk = metrics.get("risk", {})
        
        table.add_row(
            agent_id,
            format_pnl(pnl.get("total", 0)),
            format_pnl(pnl.get("today", 0)),
            str(trades.get("total", 0)),
            f"{trades.get('win_rate', 0):.1%}",
            f"{risk.get('sharpe', 0):.2f}",
            format_percent(risk.get('max_drawdown', 0), invert=True),
        )
    
    console.print(table)


@metrics_app.command("history")
def metrics_history(
    agent_id: str = typer.Argument(..., help="Agent ID"),
    metric: str = typer.Option("pnl", "--metric", "-m", help="Metric: pnl, win_rate, sharpe, trades"),
    period: str = typer.Option("7d", "--period", "-p", help="Period: 1h, 24h, 7d, 30d"),
    output_format: str = typer.Option("table", "--format", "-f", help="Output format: table, json, csv"),
    monitor_url: Optional[str] = typer.Option(None, "--url", help="Monitor URL"),
):
    """
    View historical metrics for an agent.
    
    Examples:
        neleus metrics history momentum-eth-01
        neleus metrics history momentum-eth-01 -m sharpe -p 30d
    """
    console.print(f"\n📈 [bold]History: {agent_id} - {metric}[/bold] ({period})\n")
    
    # Generate demo historical data
    now = datetime.now()
    data_points = []
    
    periods = {"1h": (12, 5), "24h": (24, 60), "7d": (7, 1440), "30d": (30, 1440)}
    count, interval_mins = periods.get(period, (7, 1440))
    
    import random
    base_value = 1000.0 if metric == "pnl" else 0.6 if metric == "win_rate" else 1.5
    
    for i in range(count):
        timestamp = now - timedelta(minutes=interval_mins * (count - 1 - i))
        if metric == "pnl":
            value = base_value + random.uniform(-50, 100) * (i + 1)
        elif metric == "win_rate":
            value = min(0.9, max(0.3, base_value + random.uniform(-0.1, 0.1)))
        elif metric == "sharpe":
            value = max(0, base_value + random.uniform(-0.5, 0.5))
        else:
            value = int(base_value + random.randint(0, 20) * i)
        
        data_points.append({
            "timestamp": timestamp.strftime("%Y-%m-%d %H:%M"),
            "value": value,
        })
    
    if output_format == "json":
        console.print_json(data=data_points)
        return
    
    if output_format == "csv":
        console.print("timestamp,value")
        for dp in data_points:
            console.print(f"{dp['timestamp']},{dp['value']}")
        return
    
    table = Table()
    table.add_column("Timestamp", style="dim")
    table.add_column(metric.upper(), justify="right")
    
    for dp in data_points:
        if metric == "pnl":
            formatted = format_pnl(dp["value"])
        elif metric == "win_rate":
            formatted = format_percent(dp["value"])
        elif metric == "sharpe":
            formatted = f"[cyan]{dp['value']:.2f}[/cyan]"
        else:
            formatted = str(int(dp["value"]))
        
        table.add_row(dp["timestamp"], formatted)
    
    console.print(table)


@metrics_app.command("export")
def metrics_export(
    agent_id: str = typer.Argument(..., help="Agent ID"),
    output_file: str = typer.Option("metrics.json", "--output", "-o", help="Output file path"),
    period: str = typer.Option("30d", "--period", "-p", help="Period: 7d, 30d, 90d, all"),
    monitor_url: Optional[str] = typer.Option(None, "--url", help="Monitor URL"),
):
    """
    Export agent metrics to a file.
    
    Examples:
        neleus metrics export momentum-eth-01 -o my_metrics.json
        neleus metrics export momentum-eth-01 -p 90d -o quarter.json
    """
    console.print(f"\n📤 [bold]Exporting Metrics: {agent_id}[/bold]\n")
    
    with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
        task = progress.add_task("Fetching metrics...", total=None)
        
        # Get metrics (demo)
        metrics = DEMO_AGENT_METRICS.get(agent_id, list(DEMO_AGENT_METRICS.values())[0])
        
        export_data = {
            "agent_id": agent_id,
            "period": period,
            "exported_at": datetime.now().isoformat(),
            "metrics": metrics,
        }
        
        progress.update(task, description="Writing file...")
        
        try:
            with open(output_file, "w") as f:
                json.dump(export_data, f, indent=2)
            
            progress.update(task, description="Done!")
            console.print(f"[green]✓[/green] Metrics exported to [cyan]{output_file}[/cyan]")
            
        except Exception as e:
            progress.update(task, description="[red]Failed[/red]")
            console.print(f"[red]Error:[/red] {e}")
            raise typer.Exit(1)


@metrics_app.command("alerts")
def metrics_alerts(
    agent_id: Optional[str] = typer.Option(None, "--agent", "-a", help="Filter by agent"),
    severity: Optional[str] = typer.Option(None, "--severity", "-s", help="Filter by severity: info, warning, critical"),
    monitor_url: Optional[str] = typer.Option(None, "--url", help="Monitor URL"),
):
    """View active alerts and notifications."""
    console.print("\n🔔 [bold]Active Alerts[/bold]\n")
    
    # Demo alerts
    alerts = [
        {
            "agent": "momentum-eth-01",
            "severity": "warning",
            "message": "Win rate dropped below 60% threshold",
            "timestamp": "2026-01-28 10:15:00",
        },
        {
            "agent": "arb-btc-perp-01",
            "severity": "info",
            "message": "Daily P&L target reached",
            "timestamp": "2026-01-28 09:30:00",
        },
        {
            "agent": "momentum-eth-01",
            "severity": "critical",
            "message": "Circuit breaker triggered: max position size",
            "timestamp": "2026-01-28 08:45:00",
        },
    ]
    
    # Apply filters
    if agent_id:
        alerts = [a for a in alerts if a["agent"] == agent_id]
    if severity:
        alerts = [a for a in alerts if a["severity"] == severity]
    
    if not alerts:
        console.print("[dim]No active alerts[/dim]")
        return
    
    table = Table()
    table.add_column("Time", style="dim")
    table.add_column("Severity")
    table.add_column("Agent", style="cyan")
    table.add_column("Message")
    
    severity_styles = {
        "info": "[blue]ℹ info[/blue]",
        "warning": "[yellow]⚠ warning[/yellow]",
        "critical": "[red]🚨 critical[/red]",
    }
    
    for alert in alerts:
        table.add_row(
            alert["timestamp"],
            severity_styles.get(alert["severity"], alert["severity"]),
            alert["agent"],
            alert["message"],
        )
    
    console.print(table)


@metrics_app.command("dashboard")
def metrics_dashboard(
    refresh: int = typer.Option(5, "--refresh", "-r", help="Refresh interval in seconds"),
    monitor_url: Optional[str] = typer.Option(None, "--url", help="Monitor URL"),
):
    """Launch terminal dashboard for real-time metrics."""
    console.print("\n📊 [bold]Launching Metrics Dashboard[/bold]\n")
    console.print("[dim]Press Ctrl+C to exit[/dim]\n")
    
    try:
        import time
        
        while True:
            # Clear screen and redraw
            console.clear()
            console.print("[bold]NELEUS METRICS DASHBOARD[/bold]", justify="center")
            console.print(f"[dim]Last updated: {datetime.now().strftime('%H:%M:%S')}[/dim]\n")
            
            # Summary table
            table = Table(title="Agent Performance")
            table.add_column("Agent", style="cyan")
            table.add_column("Status")
            table.add_column("P&L", justify="right")
            table.add_column("Positions", justify="right")
            table.add_column("Win Rate", justify="right")
            
            for agent_id, metrics in DEMO_AGENT_METRICS.items():
                pnl = metrics.get("pnl", {})
                trades = metrics.get("trades", {})
                pos = metrics.get("positions", {})
                
                table.add_row(
                    agent_id,
                    "[green]● running[/green]",
                    format_pnl(pnl.get("total", 0)),
                    str(pos.get("open", 0)),
                    f"{trades.get('win_rate', 0):.1%}",
                )
            
            console.print(table)
            console.print(f"\n[dim]Refreshing every {refresh}s... Press Ctrl+C to exit[/dim]")
            
            time.sleep(refresh)
            
    except KeyboardInterrupt:
        console.print("\n[dim]Dashboard closed[/dim]")
