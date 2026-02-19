"""
Neleus CLI - Agents Command

Manage deployed trading agents.
"""

import logging
import os
from typing import Optional
from datetime import datetime

logger = logging.getLogger(__name__)

import typer
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.progress import Progress, SpinnerColumn, TextColumn
from rich.live import Live

console = Console()

agents_app = typer.Typer(
    name="agents",
    help="Manage deployed trading agents",
)


def get_orchestrator_url() -> str:
    """Get orchestrator URL from environment or default."""
    return os.environ.get("NELEUS_ORCHESTRATOR_URL", "http://localhost:8080")


def get_manager(url: str):
    """Get agent manager instance."""
    try:
        from ..agents import AgentManager
        return AgentManager(orchestrator_url=url)
    except ImportError:
        return None


# Demo data for when orchestrator is not available
DEMO_AGENTS = [
    {
        "id": "eth-momentum-01",
        "name": "ETH Momentum Agent",
        "strategy": "momentum_strategy",
        "state": "running",
        "venue": "hyperliquid",
        "instruments": ["ETH-PERP"],
        "pnl": 1250.50,
        "uptime": "2d 14h 32m",
    },
    {
        "id": "btc-reversion-02",
        "name": "BTC Mean Reversion",
        "strategy": "mean_reversion",
        "state": "paused",
        "venue": "hyperliquid",
        "instruments": ["BTC-PERP"],
        "pnl": -320.25,
        "uptime": "1d 8h 15m",
    },
    {
        "id": "multi-mm-03",
        "name": "Multi-Asset Market Maker",
        "strategy": "market_maker",
        "state": "stopped",
        "venue": "lighter",
        "instruments": ["ETH-PERP", "BTC-PERP", "SOL-PERP"],
        "pnl": 5420.00,
        "uptime": "0h",
    },
]


def state_style(state: str) -> str:
    """Get style for agent state."""
    styles = {
        "running": "[green]●[/green] running",
        "paused": "[yellow]◐[/yellow] paused",
        "stopped": "[dim]○[/dim] stopped",
        "error": "[red]✗[/red] error",
        "starting": "[cyan]◔[/cyan] starting",
        "stopping": "[yellow]◔[/yellow] stopping",
    }
    return styles.get(state, state)


@agents_app.command("list")
def agents_list(
    state: Optional[str] = typer.Option(None, "--state", "-s", help="Filter by state"),
    venue: Optional[str] = typer.Option(None, "--venue", "-v", help="Filter by venue"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """List all deployed agents."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    console.print("\n📋 [bold]Deployed Agents[/bold]\n")
    
    if manager:
        try:
            agents = manager.list_agents()
        except Exception as e:
            logger.debug("Could not connect to orchestrator: %s", e)
            console.print(f"[yellow]Warning:[/yellow] Could not connect to orchestrator")
            console.print(f"[dim]{e}[/dim]\n")
            agents = DEMO_AGENTS
    else:
        console.print("[dim]Demo mode - showing sample agents[/dim]\n")
        agents = DEMO_AGENTS
    
    # Filter
    if state:
        agents = [a for a in agents if a.get("state") == state]
    if venue:
        agents = [a for a in agents if a.get("venue") == venue]
    
    if not agents:
        console.print("[dim]No agents found[/dim]")
        return
    
    table = Table()
    table.add_column("ID", style="cyan")
    table.add_column("Name")
    table.add_column("Strategy")
    table.add_column("State")
    table.add_column("Venue")
    table.add_column("P&L", justify="right")
    table.add_column("Uptime")
    
    for agent in agents:
        pnl = agent.get("pnl", 0)
        pnl_str = f"[green]+${pnl:,.2f}[/green]" if pnl >= 0 else f"[red]-${abs(pnl):,.2f}[/red]"
        
        table.add_row(
            agent.get("id", ""),
            agent.get("name", ""),
            agent.get("strategy", ""),
            state_style(agent.get("state", "unknown")),
            agent.get("venue", ""),
            pnl_str,
            agent.get("uptime", ""),
        )
    
    console.print(table)
    
    # Summary
    running = sum(1 for a in agents if a.get("state") == "running")
    total_pnl = sum(a.get("pnl", 0) for a in agents)
    pnl_color = "green" if total_pnl >= 0 else "red"
    
    console.print(f"\n[dim]Total: {len(agents)} agents ({running} running) | Total P&L: [{pnl_color}]${total_pnl:,.2f}[/{pnl_color}][/dim]")


@agents_app.command("status")
def agents_status(
    agent_id: str = typer.Argument(..., help="Agent ID"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Get detailed status of an agent."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    # Get agent info
    if manager:
        try:
            stats = manager.get_stats(agent_id)
            agent = {
                "id": agent_id,
                "name": getattr(stats, "name", agent_id),
                "state": getattr(stats, "state", "unknown"),
                "strategy": getattr(stats, "strategy_id", "unknown"),
                "venue": getattr(stats, "venue", "unknown"),
                "instruments": getattr(stats, "instruments", []),
                "realized_pnl": getattr(stats, "realized_pnl", 0),
                "unrealized_pnl": getattr(stats, "unrealized_pnl", 0),
                "orders_placed": getattr(stats, "orders_placed", 0),
                "trades_executed": getattr(stats, "trades_executed", 0),
                "started_at": getattr(stats, "started_at", None),
                "restart_count": getattr(stats, "restart_count", 0),
            }
        except Exception as e:
            console.print(f"[red]Error:[/red] {e}")
            raise typer.Exit(1)
    else:
        # Demo mode
        agent = next((a for a in DEMO_AGENTS if a.get("id") == agent_id), None)
        if not agent:
            console.print(f"[red]Error:[/red] Agent '{agent_id}' not found")
            raise typer.Exit(1)
        agent["realized_pnl"] = agent.get("pnl", 0)
        agent["unrealized_pnl"] = 0
        agent["orders_placed"] = 142
        agent["trades_executed"] = 89
        agent["restart_count"] = 0
    
    # Display status
    pnl_color = "green" if agent.get("realized_pnl", 0) >= 0 else "red"
    
    console.print(Panel.fit(
        f"""[bold]Agent Status[/bold]

[cyan]ID:[/cyan]          {agent.get('id')}
[cyan]Name:[/cyan]        {agent.get('name')}
[cyan]State:[/cyan]       {state_style(agent.get('state', 'unknown'))}
[cyan]Strategy:[/cyan]    {agent.get('strategy')}
[cyan]Venue:[/cyan]       {agent.get('venue')}
[cyan]Instruments:[/cyan] {', '.join(agent.get('instruments', []))}

[bold]Performance[/bold]
[cyan]Realized P&L:[/cyan]   [{pnl_color}]${agent.get('realized_pnl', 0):,.2f}[/{pnl_color}]
[cyan]Unrealized P&L:[/cyan] ${agent.get('unrealized_pnl', 0):,.2f}
[cyan]Orders Placed:[/cyan]  {agent.get('orders_placed', 0)}
[cyan]Trades:[/cyan]         {agent.get('trades_executed', 0)}
[cyan]Restarts:[/cyan]       {agent.get('restart_count', 0)}
""",
        title=f"🤖 {agent.get('name', agent_id)}",
        border_style="cyan",
    ))


@agents_app.command("start")
def agents_start(
    agent_id: str = typer.Argument(..., help="Agent ID to start"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Start a stopped or paused agent."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
        task = progress.add_task(f"Starting agent {agent_id}...", total=None)
        
        if manager:
            try:
                manager.start(agent_id)
                progress.update(task, description="Agent started!")
                console.print(f"\n[green]✓[/green] Agent [cyan]{agent_id}[/cyan] started")
            except Exception as e:
                progress.update(task, description="[red]Failed[/red]")
                console.print(f"\n[red]Error:[/red] {e}")
                raise typer.Exit(1)
        else:
            progress.update(task, description="Demo mode")
            console.print(f"\n[yellow]Demo:[/yellow] Would start agent [cyan]{agent_id}[/cyan]")


@agents_app.command("stop")
def agents_stop(
    agent_id: str = typer.Argument(..., help="Agent ID to stop"),
    force: bool = typer.Option(False, "--force", "-f", help="Force stop without graceful shutdown"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Stop a running agent."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    if not force:
        confirm = typer.confirm(f"Stop agent {agent_id}?", default=True)
        if not confirm:
            raise typer.Exit(0)
    
    with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
        task = progress.add_task(f"Stopping agent {agent_id}...", total=None)
        
        if manager:
            try:
                manager.stop(agent_id)
                progress.update(task, description="Agent stopped!")
                console.print(f"\n[green]✓[/green] Agent [cyan]{agent_id}[/cyan] stopped")
            except Exception as e:
                progress.update(task, description="[red]Failed[/red]")
                console.print(f"\n[red]Error:[/red] {e}")
                raise typer.Exit(1)
        else:
            progress.update(task, description="Demo mode")
            console.print(f"\n[yellow]Demo:[/yellow] Would stop agent [cyan]{agent_id}[/cyan]")


@agents_app.command("pause")
def agents_pause(
    agent_id: str = typer.Argument(..., help="Agent ID to pause"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Pause a running agent (can be resumed)."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    if manager:
        try:
            manager.pause(agent_id)
            console.print(f"\n[green]✓[/green] Agent [cyan]{agent_id}[/cyan] paused")
            console.print(f"[dim]Resume with: neleus agents resume {agent_id}[/dim]")
        except Exception as e:
            console.print(f"\n[red]Error:[/red] {e}")
            raise typer.Exit(1)
    else:
        console.print(f"\n[yellow]Demo:[/yellow] Would pause agent [cyan]{agent_id}[/cyan]")


@agents_app.command("resume")
def agents_resume(
    agent_id: str = typer.Argument(..., help="Agent ID to resume"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Resume a paused agent."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    if manager:
        try:
            manager.resume(agent_id)
            console.print(f"\n[green]✓[/green] Agent [cyan]{agent_id}[/cyan] resumed")
        except Exception as e:
            console.print(f"\n[red]Error:[/red] {e}")
            raise typer.Exit(1)
    else:
        console.print(f"\n[yellow]Demo:[/yellow] Would resume agent [cyan]{agent_id}[/cyan]")


@agents_app.command("restart")
def agents_restart(
    agent_id: str = typer.Argument(..., help="Agent ID to restart"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Restart an agent (stop then start)."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
        task = progress.add_task(f"Restarting agent {agent_id}...", total=None)
        
        if manager:
            try:
                manager.stop(agent_id)
                progress.update(task, description="Stopped, starting...")
                manager.start(agent_id)
                progress.update(task, description="Agent restarted!")
                console.print(f"\n[green]✓[/green] Agent [cyan]{agent_id}[/cyan] restarted")
            except Exception as e:
                progress.update(task, description="[red]Failed[/red]")
                console.print(f"\n[red]Error:[/red] {e}")
                raise typer.Exit(1)
        else:
            progress.update(task, description="Demo mode")
            console.print(f"\n[yellow]Demo:[/yellow] Would restart agent [cyan]{agent_id}[/cyan]")


@agents_app.command("delete")
def agents_delete(
    agent_id: str = typer.Argument(..., help="Agent ID to delete"),
    force: bool = typer.Option(False, "--force", "-f", help="Skip confirmation"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """Delete an agent permanently."""
    url = orchestrator_url or get_orchestrator_url()
    manager = get_manager(url)
    
    if not force:
        confirm = typer.confirm(
            f"⚠️  Permanently delete agent {agent_id}?",
            default=False,
        )
        if not confirm:
            raise typer.Exit(0)
    
    if manager:
        try:
            manager.delete(agent_id)
            console.print(f"\n[green]✓[/green] Agent [cyan]{agent_id}[/cyan] deleted")
        except Exception as e:
            console.print(f"\n[red]Error:[/red] {e}")
            raise typer.Exit(1)
    else:
        console.print(f"\n[yellow]Demo:[/yellow] Would delete agent [cyan]{agent_id}[/cyan]")


@agents_app.command("logs")
def agents_logs(
    agent_id: str = typer.Argument(..., help="Agent ID"),
    follow: bool = typer.Option(False, "--follow", "-f", help="Follow log output"),
    lines: int = typer.Option(50, "--lines", "-n", help="Number of lines to show"),
    orchestrator_url: Optional[str] = typer.Option(None, "--url", help="Orchestrator URL"),
):
    """View agent logs."""
    url = orchestrator_url or get_orchestrator_url()
    
    console.print(f"\n📜 Logs for [cyan]{agent_id}[/cyan]\n")
    
    # Demo logs
    demo_logs = [
        "2026-01-28 10:15:32 [INFO] Agent starting...",
        "2026-01-28 10:15:33 [INFO] Connected to hyperliquid (testnet)",
        "2026-01-28 10:15:33 [INFO] Subscribed to ETH-PERP",
        "2026-01-28 10:15:34 [INFO] Strategy momentum_strategy initialized",
        "2026-01-28 10:15:34 [INFO] Agent running",
        "2026-01-28 10:16:00 [INFO] Bar received: ETH-PERP close=3245.50",
        "2026-01-28 10:17:00 [INFO] Bar received: ETH-PERP close=3248.25",
        "2026-01-28 10:17:00 [INFO] Signal: BUY 0.1 ETH-PERP",
        "2026-01-28 10:17:01 [INFO] Order filled: BUY 0.1 @ 3248.50",
        "2026-01-28 10:18:00 [INFO] Bar received: ETH-PERP close=3252.00",
    ]
    
    for log in demo_logs[-lines:]:
        # Color by log level
        if "[ERROR]" in log:
            console.print(f"[red]{log}[/red]")
        elif "[WARN]" in log:
            console.print(f"[yellow]{log}[/yellow]")
        elif "[INFO]" in log:
            console.print(log)
        else:
            console.print(f"[dim]{log}[/dim]")
    
    if follow:
        console.print("\n[dim]Live log streaming not available in demo mode[/dim]")


__all__ = ["agents_app"]
