"""
Neleus CLI - Signals Command

Send and manage signals to the Signal Hub.
"""

import os
import json
from typing import Optional
from datetime import datetime

import typer
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.progress import Progress, SpinnerColumn, TextColumn

console = Console()

signals_app = typer.Typer(
    name="signals",
    help="Send and manage external signals",
)


def get_signal_hub_url() -> str:
    """Get signal hub URL from environment or default."""
    return os.environ.get("NELEUS_SIGNAL_HUB_URL", "http://localhost:8081")


def get_client(url: str):
    """Get signal client instance."""
    try:
        from ..signals import SignalClient
        return SignalClient(hub_url=url)
    except ImportError:
        return None


# Demo signals for display
DEMO_SIGNALS = [
    {
        "id": "sig-001",
        "instrument": "ETH-PERP",
        "signal_type": "entry",
        "direction": "long",
        "confidence": 0.85,
        "source": "ml_model_v2",
        "timestamp": "2026-01-28 10:15:00",
    },
    {
        "id": "sig-002",
        "instrument": "BTC-PERP",
        "signal_type": "exit",
        "direction": "flat",
        "confidence": 0.72,
        "source": "sentiment_analyzer",
        "timestamp": "2026-01-28 10:18:30",
    },
    {
        "id": "sig-003",
        "instrument": "ETH-PERP",
        "signal_type": "risk_alert",
        "direction": "none",
        "confidence": 0.95,
        "source": "risk_monitor",
        "timestamp": "2026-01-28 10:20:15",
    },
]


def signal_type_style(signal_type: str) -> str:
    """Get styled signal type."""
    styles = {
        "entry": "[green]entry[/green]",
        "exit": "[yellow]exit[/yellow]",
        "scale_in": "[cyan]scale_in[/cyan]",
        "scale_out": "[blue]scale_out[/blue]",
        "risk_alert": "[red]risk_alert[/red]",
        "rebalance": "[magenta]rebalance[/magenta]",
    }
    return styles.get(signal_type, signal_type)


def direction_style(direction: str) -> str:
    """Get styled direction."""
    styles = {
        "long": "[green]▲ long[/green]",
        "short": "[red]▼ short[/red]",
        "flat": "[dim]○ flat[/dim]",
        "none": "[dim]- none[/dim]",
    }
    return styles.get(direction, direction)


@signals_app.command("send")
def signals_send(
    instrument: str = typer.Option(..., "--instrument", "-i", help="Instrument (e.g., ETH-PERP)"),
    signal_type: str = typer.Option("entry", "--type", "-t", help="Signal type: entry, exit, scale_in, scale_out, risk_alert"),
    direction: str = typer.Option("long", "--direction", "-d", help="Direction: long, short, flat"),
    confidence: float = typer.Option(0.8, "--confidence", "-c", help="Confidence (0.0 to 1.0)"),
    source: str = typer.Option("cli", "--source", "-s", help="Signal source identifier"),
    metadata: Optional[str] = typer.Option(None, "--metadata", "-m", help="JSON metadata"),
    signal_hub_url: Optional[str] = typer.Option(None, "--url", help="Signal Hub URL"),
):
    """
    Send a signal to the Signal Hub.
    
    Examples:
        neleus signals send -i ETH-PERP -t entry -d long -c 0.85
        neleus signals send -i BTC-PERP -t exit -d flat
        neleus signals send -i ETH-PERP -t risk_alert -s my_model
    """
    url = signal_hub_url or get_signal_hub_url()
    
    # Parse metadata if provided
    meta = {}
    if metadata:
        try:
            meta = json.loads(metadata)
        except json.JSONDecodeError:
            console.print("[red]Error:[/red] Invalid JSON in metadata")
            raise typer.Exit(1)
    
    # Validate inputs
    valid_types = ["entry", "exit", "scale_in", "scale_out", "risk_alert", "rebalance"]
    if signal_type not in valid_types:
        console.print(f"[red]Error:[/red] Invalid signal type. Must be one of: {', '.join(valid_types)}")
        raise typer.Exit(1)
    
    valid_directions = ["long", "short", "flat", "none"]
    if direction not in valid_directions:
        console.print(f"[red]Error:[/red] Invalid direction. Must be one of: {', '.join(valid_directions)}")
        raise typer.Exit(1)
    
    if not 0.0 <= confidence <= 1.0:
        console.print("[red]Error:[/red] Confidence must be between 0.0 and 1.0")
        raise typer.Exit(1)
    
    # Display signal
    console.print(Panel.fit(
        f"""[bold]Signal[/bold]
        
Instrument:  [cyan]{instrument}[/cyan]
Type:        {signal_type_style(signal_type)}
Direction:   {direction_style(direction)}
Confidence:  {confidence:.0%}
Source:      {source}
Hub URL:     {url}
""",
        title="📡 Sending Signal",
        border_style="cyan",
    ))
    
    # Send signal
    with Progress(SpinnerColumn(), TextColumn("[progress.description]{task.description}"), console=console) as progress:
        task = progress.add_task("Sending signal...", total=None)
        
        client = get_client(url)
        
        if client:
            try:
                from ..signals import Signal
                
                signal = Signal(
                    instrument=instrument,
                    signal_type=signal_type,
                    direction=direction,
                    confidence=confidence,
                    source=source,
                    metadata=meta,
                )
                
                signal_id = client.send(signal)
                progress.update(task, description="Signal sent!")
                console.print(f"\n[green]✓[/green] Signal sent successfully")
                console.print(f"   Signal ID: [cyan]{signal_id}[/cyan]")
                
            except Exception as e:
                progress.update(task, description="[red]Failed[/red]")
                console.print(f"\n[red]Error:[/red] {e}")
                raise typer.Exit(1)
        else:
            progress.update(task, description="Demo mode")
            import uuid
            signal_id = f"sig-{str(uuid.uuid4())[:8]}"
            console.print(f"\n[yellow]Demo:[/yellow] Signal would be sent")
            console.print(f"   Signal ID: [cyan]{signal_id}[/cyan]")


@signals_app.command("list")
def signals_list(
    instrument: Optional[str] = typer.Option(None, "--instrument", "-i", help="Filter by instrument"),
    signal_type: Optional[str] = typer.Option(None, "--type", "-t", help="Filter by type"),
    source: Optional[str] = typer.Option(None, "--source", "-s", help="Filter by source"),
    limit: int = typer.Option(20, "--limit", "-n", help="Number of signals to show"),
    signal_hub_url: Optional[str] = typer.Option(None, "--url", help="Signal Hub URL"),
):
    """List recent signals."""
    url = signal_hub_url or get_signal_hub_url()
    
    console.print("\n📡 [bold]Recent Signals[/bold]\n")
    
    # Use demo data for now
    signals = DEMO_SIGNALS
    
    # Apply filters
    if instrument:
        signals = [s for s in signals if s.get("instrument") == instrument]
    if signal_type:
        signals = [s for s in signals if s.get("signal_type") == signal_type]
    if source:
        signals = [s for s in signals if s.get("source") == source]
    
    signals = signals[:limit]
    
    if not signals:
        console.print("[dim]No signals found[/dim]")
        return
    
    table = Table()
    table.add_column("ID", style="dim")
    table.add_column("Instrument", style="cyan")
    table.add_column("Type")
    table.add_column("Direction")
    table.add_column("Confidence", justify="right")
    table.add_column("Source")
    table.add_column("Time")
    
    for sig in signals:
        table.add_row(
            sig.get("id", ""),
            sig.get("instrument", ""),
            signal_type_style(sig.get("signal_type", "")),
            direction_style(sig.get("direction", "")),
            f"{sig.get('confidence', 0):.0%}",
            sig.get("source", ""),
            sig.get("timestamp", ""),
        )
    
    console.print(table)
    console.print(f"\n[dim]Showing {len(signals)} signals[/dim]")


@signals_app.command("test")
def signals_test(
    agent_id: Optional[str] = typer.Option(None, "--agent", "-a", help="Target agent ID"),
    signal_hub_url: Optional[str] = typer.Option(None, "--url", help="Signal Hub URL"),
):
    """Send a test signal to verify connectivity."""
    url = signal_hub_url or get_signal_hub_url()
    
    console.print("\n🧪 [bold]Signal Hub Test[/bold]\n")
    
    # Test connectivity
    client = get_client(url)
    
    if client:
        try:
            # Try to send a test signal
            from ..signals import Signal
            
            signal = Signal(
                instrument="TEST-PERP",
                signal_type="entry",
                direction="long",
                confidence=1.0,
                source="cli_test",
                metadata={"test": True},
            )
            
            signal_id = client.send(signal)
            console.print(f"[green]✓[/green] Signal Hub is reachable at {url}")
            console.print(f"[green]✓[/green] Test signal sent: {signal_id}")
            
            if agent_id:
                console.print(f"[green]✓[/green] Signal routed to agent: {agent_id}")
            
        except Exception as e:
            console.print(f"[red]✗[/red] Failed to connect to Signal Hub")
            console.print(f"   [dim]{e}[/dim]")
            raise typer.Exit(1)
    else:
        console.print(f"[yellow]Demo:[/yellow] Would test Signal Hub at {url}")
        console.print(f"[green]✓[/green] Signal Hub connectivity (simulated)")
        if agent_id:
            console.print(f"[green]✓[/green] Agent routing: {agent_id} (simulated)")


@signals_app.command("subscribe")
def signals_subscribe(
    agent_id: str = typer.Argument(..., help="Agent ID to subscribe"),
    instruments: str = typer.Option(..., "--instruments", "-i", help="Comma-separated instruments"),
    sources: Optional[str] = typer.Option(None, "--sources", "-s", help="Comma-separated sources"),
    min_confidence: float = typer.Option(0.5, "--min-confidence", help="Minimum confidence filter"),
    signal_hub_url: Optional[str] = typer.Option(None, "--url", help="Signal Hub URL"),
):
    """Create a signal subscription for an agent."""
    url = signal_hub_url or get_signal_hub_url()
    
    instruments_list = [i.strip() for i in instruments.split(",")]
    sources_list = [s.strip() for s in sources.split(",")] if sources else None
    
    console.print(Panel.fit(
        f"""[bold]New Subscription[/bold]
        
Agent:          [cyan]{agent_id}[/cyan]
Instruments:    {', '.join(instruments_list)}
Sources:        {', '.join(sources_list) if sources_list else 'all'}
Min Confidence: {min_confidence:.0%}
""",
        title="📥 Subscribe",
    ))
    
    console.print(f"[green]✓[/green] Subscription created (demo)")
    console.print(f"[dim]Agent {agent_id} will now receive signals for {', '.join(instruments_list)}[/dim]")


@signals_app.command("sources")
def signals_sources(
    signal_hub_url: Optional[str] = typer.Option(None, "--url", help="Signal Hub URL"),
):
    """List known signal sources."""
    console.print("\n📡 [bold]Signal Sources[/bold]\n")
    
    # Demo sources
    sources = [
        {"name": "ml_model_v2", "type": "ML Model", "signals_24h": 142, "status": "active"},
        {"name": "sentiment_analyzer", "type": "Sentiment", "signals_24h": 58, "status": "active"},
        {"name": "risk_monitor", "type": "Risk", "signals_24h": 12, "status": "active"},
        {"name": "tradingview_webhook", "type": "Webhook", "signals_24h": 87, "status": "active"},
    ]
    
    table = Table()
    table.add_column("Source", style="cyan")
    table.add_column("Type")
    table.add_column("Signals (24h)", justify="right")
    table.add_column("Status")
    
    for src in sources:
        status = "[green]● active[/green]" if src["status"] == "active" else "[dim]○ inactive[/dim]"
        table.add_row(
            src["name"],
            src["type"],
            str(src["signals_24h"]),
            status,
        )
    
    console.print(table)
